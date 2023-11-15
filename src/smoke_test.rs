use crate::{scaffolding::Context, server};
use std::error::Error;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};

pub(crate) fn run(ctx: &Context) -> Result<(), Box<dyn Error>> {
    let shutdown_signal = server::serve(ctx, handle)?;
    shutdown_signal.set_as_ctrl_c_handler()?;
    shutdown_signal.sleep_until_shutdown();
    Ok(())
}

fn handle(stream: &mut TcpStream, _remote_address: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream.try_clone()?);

    let mut buffer = [0u8; 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break Ok(()), // EOF
            Ok(bytes_read) => writer.write_all(&buffer[..bytes_read]),
            Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
            Err(e) => Err(e),
        }?
    }
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    println!("Usage: {}", ctx.program_name);
    Ok(())
}
