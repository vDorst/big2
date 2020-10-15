use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

use std::net::SocketAddr;
use tokio::sync::mpsc;

use log::{error, info, trace};

use crate::big2rules;
use crate::muon;

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::Sender<Vec<u8>>;

struct GameServerState {
    clients: HashMap<SocketAddr, Tx>,
    names: HashMap<SocketAddr, String>,
}

impl GameServerState {
    pub fn new() -> Self {
        GameServerState {
            clients: HashMap::new(),
            names: HashMap::new(),
        }
    }
    pub fn seats_left(&self) -> bool {
        self.clients.len() != 4
    }
    /// Create a new instance of `Peer`.
    async fn new_client(&mut self, addr: SocketAddr, tx: Tx) -> Result<(), Box<dyn Error>> {
        // Add an entry for this `Peer` in the shared state map.
        self.clients.insert(addr, tx);

        println!("Add client {}", addr);

        Ok(())
    }
    async fn remove_client(&mut self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
        println!("Remove client {}", addr);
        self.clients.remove(&addr);
        if let Some(name) = self.names.remove(&addr) {
            let msg = vec![123];
            self.broadcast(addr, msg).await?;
        }
        Ok(())
    }

    async fn join(&mut self, addr: SocketAddr, name: String) -> Result<(), Box<dyn Error>> {
        self.names.insert(addr, name);

        let buffer = vec![
            5u8, 0, 0, 0, 0xe0, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0x15, 7,
            0x37, 0x28, 0x38, 0x39, 0xa, 0x2b, 0x3b, 0x2c, 0x1d, 0x3d, 2, 0, 0, 0, 0xd, 0, 0, 0,
            0x54, 0x69, 0x6b, 0x6b, 0x69, 0x65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0,
            0, 9, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0x68, 0x6f, 0x73, 0x74, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0x52, 0x65,
            0x6e, 0x65, 0x31, 0x32, 0x33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0xb,
            0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0x52, 0x65, 0x6e, 0x65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0xd, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0x16, 0x26, 0, 0,
            0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        self.broadcast(addr, buffer).await?;
        Ok(())
    }

    /// Send a `LineCodec` encoded message to every peer, except
    /// for the sender.
    async fn broadcast(&mut self, sender: SocketAddr, message: Vec<u8>) -> Result<(), Box<dyn Error>> {
        for peer in self.clients.iter_mut() {
            if let Err(e) = peer.1.send(message.clone()).await {
                println!("broadcast error {:?}", e);
            }
            println!("Send message {:?} to {}", message, peer.0);
        }
        Ok(())
    }
}

async fn big2_handler(
    gs: Arc<Mutex<GameServerState>>,
    mut socket: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let remote_ip = socket.peer_addr()?;
    println!("big2_handler: New connection from {}", remote_ip);

    {
        if !gs.lock().await.seats_left() {
            println!("No more seats left!");
            socket.write(&"No more seats left!\n".as_bytes()).await?;
            return Ok(());
        }
    }

    // Add an entry for this `Peer` in the shared state map.
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    gs.lock().await.new_client(remote_ip, tx).await?;

    let mut timeout_timer = time::delay_for(Duration::from_secs(5));

    let mut joined = false;
    // Read the first line from the `LineCodec` stream to get the username.
    let mut buf = [0u8; 512];
    tokio::select! {
        nbytes = socket.read(&mut buf) => {
            match nbytes {
                Err(e) => {
                    println!("TCP Error ! {:?}", e);
                }
                Ok(0) => {
                    println!("Socket closed!");
                }
                Ok(nbytes) => {
                    let muon_ret = muon::parse_packet(nbytes, &buf);
                    match muon_ret {
                        Ok(muon::StateMessageActions::Join(name)) => {
                            gs.lock().await.join(remote_ip, name).await?;
                            joined = true;
                        },
                        _ => { println!("Invalid packet! {:?}", buf);
                            let _ = socket.write(&"Invalid UTF8\n".as_bytes()).await?;
                        },
                    }
                }
            }
        }
        _ = &mut timeout_timer => {
            socket.write(&"\nTimeout! Bye!\n".as_bytes()).await?;
        }
    }

    if !joined {
        gs.lock().await.remove_client(remote_ip).await?;
        return Ok(());
    }

    loop {
        let mut buf = [0u8; 512];

        let mut hartbeat_timer = time::delay_for(Duration::from_secs(5));

        tokio::select! {
            to_clt = rx.recv() => {
                match to_clt {
                    Some(to_clt) =>
                    {
                        println!("Write to client {:?}", to_clt);
                        socket.write(&to_clt).await?;
                    }
                    None => {
                        println!("Channel RX: None");
                        break;
                    }
                }
            }
            nbytes = socket.read(&mut buf) => {
                match nbytes {
                    Err(e) => {
                        println!("TCP Error ! {:?}", e);
                        break;
                    }
                    Ok(0) => {
                        println!("Socket closed!");
                        break;
                    }
                    Ok(nbytes) => {
                        let rec = String::from_utf8(buf[0..nbytes-1].to_vec());
                        match rec {
                            Ok(s) => {
                                println!("client send: {}", s);
                                // gs.lock().await.broadcast(remote_ip, &s).await?;
                            }
                            Err(_) => {
                                socket.write(&"Invalid UTF8\n".as_bytes()).await?;
                            }
                        }
                    }
                }
            }
            _ = &mut hartbeat_timer => {
                let hb_msg = muon::create_heartbeat_msg();
                socket.write(&hb_msg).await?;
            }
            else => {
                println!("iets");
                break;
            }
        }
    }

    gs.lock().await.remove_client(remote_ip).await?;

    Ok(())
}

pub async fn start_server(listener: TcpListener) {
    let peers = Arc::new(Mutex::new(GameServerState::new()));

    let mut listener = listener;
    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let peers = Arc::clone(&peers);
        tokio::spawn(async move {
            let x = big2_handler(peers, socket).await;
            if x.is_err() {
                println!("err {:?}", x.unwrap_err());
            }
        });
    }
}
