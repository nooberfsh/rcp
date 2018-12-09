mod jumpserver_config;

use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::{Path, PathBuf};

use ssh2::Session;
use uuid::Uuid;

fn main() {
    let config = jumpserver_config::Config {
        ip: "10.20.34.27".parse().unwrap(),
        port: 22,
        username: "huajiongjiong".into(),
        private_key: "/home/nooberfsh/.ssh/id_rsa_qiniu".parse().unwrap(),
    };
    let conn = Connection::connect(&config).unwrap();
    // conn.send("123.txt", "xs5:~/huajiongjiong/rcp/").unwrap();
    conn.recv(".", "xs5:~/huajiongjiong/server.sh").unwrap();
}

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Ssh(ssh2::Error),
    Cmd(String),
    InvalidRemoteAddr,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<ssh2::Error> for Error {
    fn from(e: ssh2::Error) -> Self {
        Error::Ssh(e)
    }
}

struct Connection {
    sess: Session,
    _socket: TcpStream,
}

impl Connection {
    fn new(sess: Session, socket: TcpStream) -> Self {
        Connection {
            sess: sess,
            _socket: socket,
        }
    }

    // Connect to the local SSH server
    fn connect(config: &jumpserver_config::Config) -> Result<Self, Error> {
        let tcp = TcpStream::connect(config.addr()).expect("can not connect to the jumpserver");
        let mut sess = Session::new().expect("initialize a session failed");
        sess.handshake(&tcp)?;

        sess.userauth_pubkey_file(&config.username, None, &config.private_key, None)?;
        assert!(sess.authenticated());
        Ok(Connection::new(sess, tcp))
    }

    fn recv<P: AsRef<Path>>(&self, local: P, remote: &str) -> Result<(), Error> {
        let dir = self.create_dir()?;
        let _clean = Clean(self, dir.clone());

        let cmd = format!("qscp {} {}", remote, dir);
        self.exec(&cmd)?;

        let name = match extract_file_name(remote) {
            Some(f) => f,
            None => return Err(Error::InvalidRemoteAddr),
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
        Ok(())
    }

    fn send<P: AsRef<Path>>(&self, local: P, remote: &str) -> Result<(), Error> {
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
        println!("copy file to jumpserver success");

        let cmd = format!("qscp {} {}", filename.to_str().unwrap(), remote);
        let code = self.exec(&cmd)?;
        if code != 0 {
            let err = format!("exec {} failed", cmd);
            return Err(Error::Cmd(err));
        }

        Ok(())
    }

    fn create_dir(&self) -> Result<String, Error> {
        //create a tmp directory
        let uuid = Uuid::new_v4();
        let mkdir = format!("mkdir {}", uuid);
        let code = self.exec(&mkdir)?;
        if code != 0 {
            let err = format!("create dir {} failed", uuid);
            return Err(Error::Cmd(err));
        }
        Ok(format!("{}", uuid))
    }

    fn exec(&self, cmd: &str) -> Result<i32, Error> {
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
