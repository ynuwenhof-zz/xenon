use log::Level;
use clap::Parser;
use thiserror::Error;
use serde::Deserialize;

use tokio::{io, fs};
use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::result;
use std::sync::Arc;
use std::str::FromStr;
use std::path::PathBuf;
use std::string::FromUtf8Error;
use std::net::{IpAddr, Ipv6Addr, Ipv4Addr, SocketAddr};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Opts {
    /// Set listener address
    #[clap(short, long, default_value_t = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))]
    addr: IpAddr,
    /// Set listener port
    #[clap(short, long, default_value_t = 1080)]
    port: u16,
    /// Set verbosity
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,
    /// Set users path
    #[clap(short, long, value_name = "PATH")]
    users: Option<PathBuf>,
}

#[derive(Deserialize)]
struct User {
    username: String,
    password: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let opts = Opts::parse();

    let level = match opts.verbose {
        1 => Level::Error,
        2 => Level::Warn,
        3 => Level::Info,
        4 => Level::Debug,
        5 => Level::Trace,
        _ => Level::Info,
    };

    simple_logger::init_with_level(level)
        .expect("failed to init logger");

    let mut users: Option<Vec<User>> = Option::default();

    if let Some(path) = opts.users {
        let content = fs::read_to_string(path).await?;
        users = Some(serde_json::from_str(&content)?);

        log::info!("loaded users");
    }

    let users = Arc::new(users);
    let listener = TcpListener::bind((opts.addr, opts.port)).await?;

    log::info!("listening for incoming connections on {}:{}", opts.addr, opts.port);

    loop {
        let (stream, addr) = listener.accept().await?;
        let users = users.clone();

        log::info!("new connection from {}", addr);

        tokio::spawn(async move {
            let mut stream = stream;

            let res = handle(&mut stream, users).await;
            if let Err(Error::Command(err)) = res {
                let buf = [
                    SOCKS_VERSION,
                    err as u8,
                    0, 1,
                    0, 0, 0, 0,
                    0, 0
                ];

                stream.write(&buf).await?;
            };

            stream.shutdown().await
        });
    }
}

const IPV4_TYPE: u8 = 0x01;
const IPV6_TYPE: u8 = 0x04;
const DOMAIN_TYPE: u8 = 0x03;
const CONNECT_CMD: u8 = 0x01;
const AUTH_VERSION: u8 = 0x01;
const SUCCESS_REPLY: u8 = 0x00;
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
    Utf(#[from] FromUtf8Error),
    #[error("{0}")]
    Command(#[from] CommandError),
}

#[allow(dead_code)]
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

async fn handle(stream: &mut TcpStream, users: Arc<Option<Vec<User>>>) -> Result<()> {
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;

    if buf[0] != SOCKS_VERSION {
        return Err(Error::InvalidVersion(SOCKS_VERSION, buf[0]));
    }

    let mut buf = vec![0u8; buf[1] as usize];
    stream.read_exact(&mut buf).await?;

    let method = Method::from(*buf
        .iter()
        .find(|&&m| {
            m == Method::NoAuth as u8 && users.is_none() ||
            m == Method::Auth as u8 && users.is_some()
        })
        .unwrap_or(&(Method::NoAcceptable as u8))
    );

    let buf = [SOCKS_VERSION, method as u8];
    stream.write(&buf).await?;

    if method == Method::NoAcceptable {
        return Err(Error::NoAcceptableMethod);
    }

    if method == Method::Auth {
        let mut buf = [0u8; 2];
        stream.read_exact(&mut buf).await?;

        if buf[0] != AUTH_VERSION {
            return Err(Error::InvalidVersion(AUTH_VERSION, buf[0]));
        }

        let mut buf = vec![0u8; buf[1] as usize];
        stream.read_exact(&mut buf).await?;
        let username = String::from_utf8(buf)?;

        let len = stream.read_u8().await?;
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).await?;
        let password = String::from_utf8(buf)?;

        if let Some(ref users) = *users {
            let exists = users
                .iter()
                .any(|u|
                    u.username == username &&
                    u.password == password
                );

            let buf = [AUTH_VERSION, !exists as u8];
            stream.write(&buf).await?;

            if !exists {
                return Err(Error::InvalidCredentials);
            }
        }
    }

    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).await?;

    if buf[0] != SOCKS_VERSION {
        return Err(Error::InvalidVersion(SOCKS_VERSION, buf[0]));
    }

    if buf[1] != CONNECT_CMD {
        return Err(Error::Command(CommandError::UnsupportedCommand));
    }

    let dest = match buf[3] {
        IPV4_TYPE => {
            let mut octets = [0u8; 4];
            stream.read_exact(&mut octets).await?;
            let port = stream.read_u16().await?;

            SocketAddr::new(IpAddr::V4(Ipv4Addr::from(octets)), port)
        }
        DOMAIN_TYPE => {
            let len = stream.read_u8().await?;

            let mut buf = vec![0u8; len as usize];
            stream.read_exact(&mut buf).await?;

            let domain = String::from_utf8(buf)
                .map_err(|_| Error::Command(CommandError::ServerFailure))?;
                
            let port = stream.read_u16().await?;

            SocketAddr::from_str(&format!("{}:{}", domain, port))
                .map_err(|_| Error::Command(CommandError::HostUnreachable))?
        }
        IPV6_TYPE => {
            let mut octets = [0u8; 16];
            stream.read_exact(&mut octets).await?;
            let port = stream.read_u16().await?;

            SocketAddr::new(IpAddr::V6(Ipv6Addr::from(octets)), port)
        }
        _ => return Err(Error::Command(CommandError::UnsupportedAddr)),
    };

    let mut peer = TcpStream::connect(dest).await
        .map_err(|_| Error::Command(CommandError::HostUnreachable))?;

    let buf = [
        SOCKS_VERSION,
        SUCCESS_REPLY,
        0, 1,
        0, 0, 0, 0,
        0, 0
    ];

    stream.write(&buf).await?;

    io::copy_bidirectional(stream, &mut peer).await?;
    Ok(())
}
