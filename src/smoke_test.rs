use crate::{scaffolding::Context, server};
use std::error::Error;
use std::io::{BufRead, BufReader, BufWriter, Cursor, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};

pub(crate) fn run(ctx: &Context) -> Result<(), Box<dyn Error>> {
    let shutdown_signal = server::serve(ctx, handle)?;
    shutdown_signal.set_as_ctrl_c_handler()?;
    shutdown_signal.sleep_until_shutdown();
    Ok(())
}

fn handle(stream: &mut TcpStream, _remote_address: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut reader = BufReader::new(stream);
    let mut writer = Cursor::new(Vec::<u8>::new());

    for line in reader.by_ref().lines() {
        match line {
            Ok(val) => {
                write!(writer, "{}", &val)
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
            Err(e) => Err(e),
        }?
    }

    match reader.into_inner().write_all(writer.get_ref()) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    println!("Usage: {}", ctx.program_name);
    Ok(())
}
