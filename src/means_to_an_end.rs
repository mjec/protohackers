use crate::{scaffolding::Context, server};
use std::collections::BTreeMap;
use std::error::Error;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};

pub(crate) fn run(ctx: &Context) -> Result<(), Box<dyn Error>> {
    let shutdown_signal = server::serve(ctx, handle)?;
    shutdown_signal.set_as_ctrl_c_handler()?;
    shutdown_signal.sleep_until_shutdown();
    Ok(())
}

fn handle(stream: &mut TcpStream, _remote_address: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut database = BTreeMap::<i32, i32>::new();

    let mut buffer: [u8; 9] = [0u8; 9];
    loop {
        match stream.read_exact(&mut buffer) {
            Ok(()) => match buffer[0] {
                b'I' => {
                    database.insert(
                        i32::from_be_bytes(buffer[1..=4].try_into()?),
                        i32::from_be_bytes(buffer[5..=8].try_into()?),
                    );
                    Ok(())
                }
                b'Q' => {
                    let start = i32::from_be_bytes(buffer[1..=4].try_into()?);
                    let end = i32::from_be_bytes(buffer[5..=8].try_into()?);
                    let result: (usize, i64) = if start <= end {
                        database.range(start..=end)
                    } else {
                        Default::default()
                    }
                    .fold((0usize, 0i64), |acc, price| {
                        (acc.0 + 1, acc.1 + i64::from(*price.1))
                    });

                    if result.0 != 0 {
                        stream.write_all(
                            &((result.1 / i64::try_from(result.0)?) as i32).to_be_bytes(),
                        )
                    } else {
                        stream.write_all(&0i32.to_be_bytes())
                    }
                }
                _ => return Ok(()), // disconnect
            },
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break Ok(()), // we didn't get 9 bytes, so disconnect
            Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
            Err(e) => Err(e),
        }?
    }
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    println!("Usage: {} means_to_an_end", ctx.program_name);
    Ok(())
}
