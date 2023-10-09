use std::{sync::{atomic::{AtomicUsize, Ordering}, Arc}, thread, io::{Read, Write, ErrorKind}};
use core::result::Result::Err;

use log::{as_display, as_debug};

use crate::scaffolding::Context;

pub(crate) fn run(ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    if ctx.problem_arguments.len() != 1 {
        return Err(String::from("A port must be specified").into());
    }
    let port = ctx.problem_arguments
        .get(0)
        .ok_or::<Box<dyn std::error::Error>>(String::from("A port must be specified").into())?
        .parse::<u16>()?;

    let active_threads = Arc::new(AtomicUsize::new(0));
    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", port))?;

    loop {
        let incoming = listener.accept()?;
        let active_threads_clone = active_threads.clone();
        log::info!(
            other_active_threads = as_display!(active_threads_clone.load(Ordering::SeqCst)),
            incoming = as_debug!(incoming.1);
            "Spawning a thread"
        );
        thread::spawn(move || {
            active_threads_clone.fetch_add(1, Ordering::SeqCst);
            let (mut stream, _) = incoming;
            let mut buffer = [0; 1024];
            loop {
                match stream.read(&mut buffer) {
                    Ok(bytes_read) => match stream.write(&buffer[..bytes_read]) {
                        Ok(bytes_written) if bytes_written != bytes_read => {
                            log::error!(
                                bytes_read = as_display!(bytes_read),
                                bytes_written = as_display!(bytes_written);
                                "Failed to write all bytes"
                            );
                        },
                        Ok(_) => (),
                        Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(_) => {
                            break
                        }
                    },
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            active_threads_clone.fetch_sub(1, Ordering::SeqCst);
            log::info!(
                other_active_threads = as_display!(active_threads_clone.load(Ordering::SeqCst)),
                incoming = as_debug!(stream.peer_addr().unwrap());
                "Thread exiting"
            );
        });
    }
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    println!("Usage: {} smoke_test <port>", ctx.program_name);
    println!("Runs an RFC 862 compliant echo server on the specified port.");
    Ok(())
}