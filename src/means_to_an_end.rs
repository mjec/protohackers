use crate::{scaffolding::Context, server};
use server::{Server as _, TcpServer};
use std::collections::BTreeMap;
use std::error::Error;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};

pub(crate) fn run(ctx: &Context) -> Result<(), Box<dyn Error>> {
    let shutdown_signal = TcpServer::new().serve(ctx, handle)?;
    shutdown_signal.set_as_ctrl_c_handler()?;
    shutdown_signal.sleep_until_shutdown();
    Ok(())
}

fn handle(stream: &mut TcpStream, _remote_address: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut database = BTreeMap::<i32, i32>::new();

    // all incoming messages are exactly 9 bytes long (convenient, right?)
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
                    // result is (count, sum); we use i64 for the sum so it doesn't overflow
                    let result: (usize, i64) = if start <= end {
                        database.range(start..=end)
                    } else {
                        // this will be empty, so result will be the initial value passed to fold()
                        Default::default()
                    }
                    .fold((0usize, 0i64), |acc, price| {
                        // count one more record, add the price
                        (acc.0 + 1, acc.1 + i64::from(*price.1))
                    });

                    if result.0 != 0 {
                        // we have at least one record, so divide the sum by the count to get the mean
                        stream.write_all(
                            &((result.1 / i64::try_from(result.0)?) as i32).to_be_bytes(),
                        )
                    } else {
                        // we have no records, so return 0
                        stream.write_all(&0i32.to_be_bytes())
                    }
                }
                _ => return Ok(()), // disconnect
            },
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break Ok(()), // we didn't get 9 bytes, so disconnect
            Err(e) => Err(e),
        }?
    }
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    println!("Usage: {} means_to_an_end", ctx.program_name);
    Ok(())
}
