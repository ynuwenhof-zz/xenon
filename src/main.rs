use clap::Parser;

use std::net::IpAddr;
use std::net::Ipv4Addr;

use tokio::io;
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Opts {
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
        let (_stream, _addr) = listener.accept().await?;

        // TODO: Handle connection
    }
}
