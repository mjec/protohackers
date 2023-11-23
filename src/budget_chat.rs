use crate::scaffolding::Context;
use crate::server::{Server as _, TcpServer};
use log::{as_debug, as_display};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};

use std::ops::Deref;
use std::sync::{
    mpsc::{channel, Sender},
    Arc, RwLock,
};
use std::thread;

static CHATROOM: Lazy<RwLock<HashMap<Arc<String>, Sender<Message>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug)]
enum MessageContent {
    UserList(Vec<Arc<String>>),
    Joined,
    Left,
    Message(Arc<String>),
}

#[derive(Clone, Debug)]
struct Message {
    from: Arc<String>,
    content: MessageContent,
}

pub(crate) fn run(ctx: &Context) -> Result<(), Box<dyn Error>> {
    let shutdown_signal = TcpServer::new().serve(ctx, handle)?;
    shutdown_signal.set_as_ctrl_c_handler()?;
    shutdown_signal.sleep_until_shutdown();
    Ok(())
}

fn send_to_room(message: Message) -> Result<(), Box<dyn Error>> {
    let user_sinks = CHATROOM.read().expect("Chatroom should not be poisoned");
    for (target, sink) in user_sinks.iter() {
        // Never send a message to the sender
        if target == &message.from {
            continue;
        }
        match sink.send(message.clone()) {
            Ok(()) => {}
            Err(e) => {
                log::warn!(
                    from = as_display!(message.from),
                    to = as_display!(target),
                    message = as_debug!(message),
                    error = as_debug!(e);
                    "Failed to forward message to client"
                );
            }
        }
    }
    Ok(())
}

fn handle(stream: &mut TcpStream, _remote_address: &SocketAddr) -> Result<(), Box<dyn Error>> {
    stream.set_read_timeout(Some(std::time::Duration::from_millis(10000)))?;
    let reader = BufReader::new(stream.try_clone()?);
    let mut lines = reader.lines();

    stream.write_all("Name pls:\n".as_bytes())?;
    stream.flush()?;
    let next_line = if let Some(r) = lines.next() {
        r
    } else {
        // This means no more lines from the client, so time to pack up and head home
        log::warn!("No name from client {} before timeout", _remote_address);
        return Ok(());
    };
    let name = Arc::new(if next_line.is_ok() {
        let inner_name = next_line?;
        if inner_name
            .matches(|c| char::is_ascii_alphanumeric(&c))
            .count()
            != inner_name.len()
        {
            stream.write_all("Name must be alphanumeric.".as_bytes())?;
            stream.flush()?;
            return Ok(());
        }
        if CHATROOM
            .read()
            .expect("Chatroom should not be poisoned")
            .contains_key(&inner_name)
        {
            stream.write_all("Name already taken.".as_bytes())?;
            stream.flush()?;
            return Ok(());
        }
        inner_name
    } else {
        log::warn!("No name provided");
        stream.write_all("Shoulda said a name.".as_bytes())?;
        stream.flush()?;
        return Ok(());
    });

    let (tx, rx) = channel::<Message>();

    let mut locked_chatroom = CHATROOM.write().expect("Chatroom should not be poisoned");
    let user_list: Vec<Arc<String>> = locked_chatroom.keys().cloned().collect();
    tx.send(Message {
        from: name.clone(),
        content: MessageContent::UserList(user_list),
    })?;
    locked_chatroom.insert(name.clone(), tx.clone());
    drop(locked_chatroom);

    send_to_room(Message {
        from: name.clone(),
        content: MessageContent::Joined,
    })?;

    let mut writer = stream.try_clone()?;
    let name_for_rx: Arc<String> = name.clone();
    thread::spawn(move || {
        for message in rx {
            log::debug!(to = as_display!(name_for_rx), message = as_debug!(message); "Got message");
            let response = match message.content {
                MessageContent::UserList(users) => format!(
                    "* The room contains: {}\n",
                    users
                        .iter()
                        .map(|s| s.deref().as_str())
                        .collect::<Vec<&str>>()
                        .join(", ")
                ),
                MessageContent::Joined => format!("* {} joined\n", message.from),
                MessageContent::Left => format!("* {} left\n", message.from),
                MessageContent::Message(content) => {
                    format!("[{}] {}\n", message.from, content)
                }
            };
            if let Err(e) = writer.write_all(response.as_bytes()) {
                log::error!("Error writing message to client {}: {}", name_for_rx, e);
                break;
            }
            if let Err(e) = writer.flush() {
                log::error!("Error flushing message to client {}: {}", name_for_rx, e);
                break;
            }
        }
    });

    for line in lines {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        log::debug!(from = as_display!(name), message = as_display!(line); "Forwarding message");
        send_to_room(Message {
            from: name.clone(),
            content: MessageContent::Message(line.into()),
        })?;
    }

    CHATROOM
        .write()
        .expect("Chatroom should not be poisoned")
        .remove(&name);
    send_to_room(Message {
        from: name.clone(),
        content: MessageContent::Left,
    })
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    println!("Usage: {} budget_chat", ctx.program_name);
    Ok(())
}
