use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub ip: IpAddr,
    pub username: String,
    port: Option<u16>,            // default: 22
    private_key: Option<PathBuf>, // default: $HOME/.ssh/id_rsa
    scp: Option<String>,          // default: scp
}

impl Config {
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::new(self.ip, self.port.unwrap_or(22))
    }

    pub fn private_key_path(&self) -> PathBuf {
        match self.private_key.clone() {
            Some(d) => d,
            None => {
                let mut p = dirs::home_dir().expect("home dir is not set");
                p.push(".ssh/id_rsa");
                p
            }
        }
    }

    pub fn scp(&self) -> &str {
        match &self.scp {
            Some(d) => &*d,
            None => "scp",
        }
    }
}
