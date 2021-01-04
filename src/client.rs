use tokio::net::{ToSocketAddrs, TcpStream};
use std::io::{Result};
use tokio::io::{AsyncWriteExt};

use crate::muon;

pub async fn connect<T: ToSocketAddrs>(addr: T, name: &str) -> Result<MuonClient> {
    let socket = TcpStream::connect(addr).await?;

    let mut client = MuonClient::new(socket);

    client.join(name).await?;

    Ok ( client )
}

pub struct MuonClient {
    stream: TcpStream,
}

impl MuonClient {
    pub fn new(socket: TcpStream) -> Self {
        MuonClient {
            stream: socket,
        }
    }

    pub async fn join(&mut self, name: &str) -> Result<()> {
        let buf = muon::create_join_msg(name);

        self.stream.write(&buf).await?;

        Ok(())
    }


}