use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub ip: IpAddr,
    pub port: u16,
    pub username: String,
    pub private_key: PathBuf,
}

impl Config {
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::new(self.ip, self.port)
    }
}
