use crate::{scaffolding::Context, server};
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
    let mut buffer = [0; 1];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break Ok(()), // EOF,
            Ok(bytes_read) => match stream.write(&buffer[..bytes_read]) {
                Ok(bytes_written) if bytes_written != bytes_read => Err(std::io::Error::new(
                    ErrorKind::Other,
                    format!(
                        "Wrote {} bytes, but read {} bytes",
                        bytes_written, bytes_read
                    ),
                )),
                Ok(_) => Ok(()),
                Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
                Err(e) => Err(e),
            },
            Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
            Err(e) => Err(e),
        }?
    }
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    println!("Usage: {}", ctx.program_name);
    Ok(())
}
