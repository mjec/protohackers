use serde::{Deserialize, Serialize};

use crate::{scaffolding::Context, server};
use std::error::Error;
use std::fmt::Display;
use std::io::{BufRead, BufReader, BufWriter, Write};
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Number {
    Integer(i64),
    Float(f64),
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
    let reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream.try_clone()?);

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let maybe_request: Result<Request, serde_json::Error> =
            serde_json::from_slice(line.as_bytes());
        if let Ok(request) = maybe_request {
            let response = Response {
                method: request.method,
                prime: match request.number {
                    Number::Integer(n) => is_prime(n),
                    Number::Float(_) => false,
                },
            };
            let mut response = serde_json::to_vec(&response)?;
            response.push(b'\n');
            writer.write_all(&response)?;
            writer.flush()?;
        } else {
            writer.write_all(b"kthxbai\n")?;
            return Ok(());
        }
    }

    Ok(())
}

fn is_prime(number: i64) -> bool {
    if number < 2 {
        false
    } else if number == 2 {
        true
    } else if number % 2 == 0 {
        false
    } else {
        !(3..=((number as f64).sqrt() as i64))
            .step_by(2)
            .any(|n| number % n == 0)
    }
}

pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    println!("Usage: {}", ctx.program_name);
    Ok(())
}
