use clap::Parser;

use std::net::IpAddr;
use std::net::Ipv4Addr;

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

fn main() {
    let _opts = Opts::parse();
}
