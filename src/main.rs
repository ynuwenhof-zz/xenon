use clap::Parser;
use thiserror::Error;

use std::result;
use std::net::IpAddr;
use std::net::Ipv4Addr;

use tokio::io;
use tokio::net::{TcpStream, TcpListener};

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

type Result<T> = result::Result<T, Error>;

#[derive(Error, Debug)]
enum Error {
    #[error("invalid version, expected {0} found {1}")]
    InvalidVersion(u8, u8),
    #[error("no acceptable method found")]
    NoMethod,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("{0}")]
    Io(#[from] io::Error),
}

async fn handle(_stream: &mut TcpStream) -> Result<()> {
    Ok(())
}
