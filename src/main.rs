mod config;

use std::env;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use ssh2::Session;
use uuid::Uuid;

fn main() {
    //TODO: this may be configured through command line argument
    let home_dir = dirs::home_dir().expect("fetch home dir failed");
    let config_path = home_dir.join(".rcp");
    let config = read_config(&config_path).unwrap();
    let scp = config.scp();

    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: rcp from to");
        return;
    }
    let conn = Connection::connect(&config).unwrap();
    if is_remote_addr(&args[1]) {
        conn.recv(&args[2], &args[1], scp).unwrap();
    } else if is_remote_addr(&args[2]) {
        conn.send(&args[1], &args[2], scp).unwrap();
    } else {
        println!("can not a remote addr")
    }
}

fn read_config<P: AsRef<Path>>(p: P) -> Result<config::Config> {
    let mut f = File::open(p)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    let config = toml::from_str(&s)?;
    Ok(config)
}

struct Connection {
    sess: Session,
}

impl Connection {
    fn new(sess: Session) -> Self {
        Connection { sess }
    }

    // Connect to the local SSH server
    fn connect(config: &config::Config) -> Result<Self> {
        let tcp = TcpStream::connect(config.addr()).expect("can not connect to the jumpserver");
        let mut sess = Session::new().expect("initialize a session failed");
        sess.set_tcp_stream(tcp);
        sess.handshake()?;

        sess.userauth_pubkey_file(&config.username, None, &config.private_key_path(), None)?;
        assert!(sess.authenticated());
        Ok(Connection::new(sess))
    }

    fn recv<P: AsRef<Path>>(&self, local: P, remote: &str, scp: &str) -> Result<()> {
        let dir = self.create_dir()?;
        let _clean = Clean(self, dir.clone());

        let cmd = format!("{} {} {}", scp, remote, dir);
        self.exec(&cmd)?;

        let name = match extract_file_name(remote) {
            Some(f) => f,
            None => return Err(anyhow!("invalid remote address: {}", remote)),
        };

        let p = local.as_ref();
        let mut local_file = if p.is_file() {
            File::open(p)?
        } else {
            if !p.is_dir() {
                fs::create_dir_all(p)?;
            }
            let p = local.as_ref().to_path_buf();
            File::create(p.join(&name))?
        };

        let remote: PathBuf = dir.into();
        let remote = remote.join(&name);

        let (mut remote_file, _) = self.sess.scp_recv(&remote)?;
        io::copy(&mut remote_file, &mut local_file)?;
        println!("copy to local success");
        Ok(())
    }

    fn send<P: AsRef<Path>>(&self, local: P, remote: &str, scp: &str) -> Result<()> {
        let dir = self.create_dir()?;
        let _clean = Clean(self, dir.clone());

        let p = local.as_ref();
        let meta = fs::metadata(p)?;

        let name = p.file_name().unwrap();
        let mut filename: PathBuf = dir.into();
        filename = filename.join(name);

        let mut remote_file = self.sess.scp_send(&filename, 0o644, meta.len(), None)?;

        let mut f = File::open(p)?;
        io::copy(&mut f, &mut remote_file)?;
        drop(remote_file);
        println!("copy file to jump server success");

        let cmd = format!("{} {} {}", scp, filename.to_str().unwrap(), remote);
        let code = self.exec(&cmd)?;
        if code != 0 {
            Err(anyhow!("exec {} failed", cmd))
        } else {
            Ok(())
        }
    }

    fn create_dir(&self) -> Result<String> {
        //create a tmp directory
        let uuid = Uuid::new_v4();
        let mkdir = format!("mkdir {}", uuid);
        let code = self.exec(&mkdir)?;
        if code != 0 {
            Err(anyhow!("create dir {} failed", uuid))
        } else {
            Ok(format!("{}", uuid))
        }
    }

    fn exec(&self, cmd: &str) -> Result<i32> {
        let mut channel = self.sess.channel_session()?;
        channel.exec(&cmd)?;

        let mut s = String::new();
        channel.read_to_string(&mut s)?;
        print!("{}", s);
        channel.wait_close()?;
        Ok(channel.exit_status()?)
    }
}

fn extract_file_name(remote: &str) -> Option<String> {
    let addr: Vec<_> = remote.split(":").collect();
    if addr.len() != 2 {
        return None;
    }

    let p: &Path = addr[1].as_ref();
    let ret = p.file_name()?.to_string_lossy().into_owned();
    Some(ret)
}

//TODO: is it correct?
fn is_remote_addr(addr: &str) -> bool {
    addr.contains(":")
}

struct Clean<'a>(&'a Connection, String);
impl<'a> Drop for Clean<'a> {
    fn drop(&mut self) {
        let cmd = format!("rm -fr {}", self.1);
        let code = self.0.exec(&cmd).expect("clean failed");
        if code != 0 {
            println!("clean failed");
        }
    }
}
