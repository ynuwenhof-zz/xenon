use clap::Parser;
use thiserror::Error;

use std::result;
use std::net::IpAddr;
use std::net::Ipv4Addr;

use tokio::io;
use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Opts {
    /// Set listener address
    #[clap(short, long, default_value_t = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))]
    addr: IpAddr,
    /// Set listener port
    #[clap(short, long, default_value_t = 1080)]
    port: u16,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let opts = Opts::parse();

    let listener = TcpListener::bind((opts.addr, opts.port)).await?;

    loop {
        let (stream, _addr) = listener.accept().await?;

        tokio::spawn(async move {
            let mut stream = stream;

            handle(&mut stream).await
        });
    }
}

const CONNECT_CMD: u8 = 0x01;
const SOCKS_VERSION: u8 = 0x05;

type Result<T> = result::Result<T, Error>;

#[derive(Error, Debug)]
enum Error {
    #[error("invalid version, expected {0} found {1}")]
    InvalidVersion(u8, u8),
    #[error("no acceptable method found")]
    NoAcceptableMethod,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Command(#[from] CommandError),
}

#[derive(Error, Debug)]
enum CommandError {
    #[error("general socks server failure")]
    ServerFailure = 0x01,
    #[error("connection not allowed by ruleset")]
    DisallowedConnection,
    #[error("network unreachable")]
    NetworkUnreachable,
    #[error("host unreachable")]
    HostUnreachable,
    #[error("connection refused")]
    ConnectionRefused,
    #[error("ttl expired")]
    TtlExpired,
    #[error("command not supported")]
    UnsupportedCommand,
    #[error("address type not supported")]
    UnsupportedAddr,
}

#[derive(Copy, Clone, PartialEq)]
enum Method {
    NoAuth,
    Auth = 0x02,
    NoAcceptable = 0xFF, 
}

impl From<u8> for Method {
    fn from(val: u8) -> Self {
        match val {
            0x00 => Self::NoAuth,
            0x02 => Self::Auth,
            _ => Self::NoAcceptable,
        }
    }
}

async fn handle(stream: &mut TcpStream) -> Result<()> {
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;

    if buf[0] != SOCKS_VERSION {
        return Err(Error::InvalidVersion(SOCKS_VERSION, buf[0]));
    }

    let mut buf = vec![0u8; buf[1] as usize];
    stream.read_exact(&mut buf).await?;

    // TODO: Accept user/pass authentication
    let method = Method::from(*buf
        .iter()
        .find(|&&m| {
            m == Method::NoAuth as u8
        })
        .unwrap_or(&(Method::NoAcceptable as u8))
    );

    let buf = [SOCKS_VERSION, method as u8];
    stream.write(&buf).await?;

    if method == Method::NoAcceptable {
        return Err(Error::NoAcceptableMethod);
    }

    // TODO: Handle user/pass authentication

    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).await?;

    if buf[0] != SOCKS_VERSION {
        return Err(Error::InvalidVersion(SOCKS_VERSION, buf[0]));
    }

    if buf[1] != CONNECT_CMD {
        return Err(Error::Command(CommandError::UnsupportedCommand));
    }

    Ok(())
}
