use serde::de::Visitor;
use serde::{Deserialize, Serialize};

use crate::{scaffolding::Context, server};
use std::error::Error;
use std::fmt::Display;
use std::io::{ErrorKind, Read, Write};
use std::marker::PhantomData;
use std::net::{SocketAddr, TcpStream};

#[derive(Debug)]
enum Method {
    IsPrime,
}

impl<'a> Deserialize<'a> for Method {
    fn deserialize<D>(deserializer: D) -> Result<Method, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "isPrime" => Ok(Method::IsPrime),
            _ => Err(serde::de::Error::custom(format!("Unknown method: {}", s))),
        }
    }
}

impl Serialize for Method {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        String::serialize(&format!("{}", &self), serializer)
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::IsPrime => write!(f, "isPrime"),
        }
    }
}

#[derive(Debug)]
enum Number {
    Integer(i64),
    Float(f64),
}

struct NumberVisitor<'a> {
    marker: PhantomData<fn() -> &'a Number>,
}
impl NumberVisitor<'_> {
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl Visitor<'_> for NumberVisitor<'_> {
    type Value = Number;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(formatter, "Expecting a number")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
        Ok(Number::Integer(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Number, E> {
        Ok(Number::Float(value))
    }
}

impl<'a> Deserialize<'a> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Number, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        deserializer.deserialize_any(NumberVisitor::<'a>::new())
    }
}

#[derive(Debug, Deserialize)]
struct Request {
    method: Method,
    number: Number,
}

#[derive(Debug, Serialize)]
struct Response {
    method: Method,
    prime: bool,
}

pub(crate) fn run(ctx: &Context) -> Result<(), Box<dyn Error>> {
    let shutdown_signal = server::serve(ctx, handle)?;
    shutdown_signal.set_as_ctrl_c_handler()?;
    shutdown_signal.sleep_until_shutdown();
    Ok(())
}

fn handle(stream: &mut TcpStream, _remote_address: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break Ok(()), // EOF,
            Ok(bytes_read) => todo!(),
            Err(e) if e.kind() == ErrorKind::Interrupted => Ok(()),
            Err(e) => Err(e),
        }?
    }
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    println!("Usage: {}", ctx.program_name);
    Ok(())
}
