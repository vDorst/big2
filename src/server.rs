#![allow(dead_code)]
#![allow(unused_variables)]

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

use std::net::SocketAddr;
use tokio::sync::mpsc;

use rand::seq::SliceRandom;
use rand::thread_rng;

use log::error;

use thiserror::Error;

use crate::big2rules;
use crate::muon;

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::Sender<Vec<u8>>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid String!")]
    InvalidString,
    #[error("TCP disconnected")]
    Disconnect(#[from] std::io::Error),

    #[error("No Seats left")]
    NoSeatsLeft,

    #[error("Peer not in list")]
    PeerNotInList,

    #[error("SendError")]
    SendError(#[from] mpsc::error::SendError<Vec<u8>>),
}

pub struct Peer {
    pub name: big2rules::Name,
    pub addr: Option<SocketAddr>,
    pub tx: Option<Tx>,
}

pub struct GameServerState {
    pub clients: Vec<Peer>,
    pub gs: big2rules::SrvGameState,
}

impl GameServerState {
    pub fn new(rounds: u8) -> Self {
        let mut clients = Vec::with_capacity(4);
        for _ in 1..=4 {
            clients.push(Peer {
                name: big2rules::Name::new(),
                addr: None,
                tx: None,
            });
        }
        assert_eq!(clients.len(), 4);

        GameServerState {
            clients: clients,
            gs: big2rules::SrvGameState::new(rounds),
        }
    }
    pub fn seats_left(&self) -> bool {
        self.clients.len() != 4
    }

    /// Create a new instance of `Peer`.
    fn new_client(
        &mut self,
        addr: SocketAddr,
        tx: Tx,
        name: big2rules::Name,
    ) -> Result<usize, Error> {
        let mut free = Vec::<usize>::with_capacity(4);

        for (i, c) in self.clients.iter_mut().enumerate() {
            if c.addr.is_none() {
                if c.name == name {
                    c.addr = Some(addr);
                    c.tx = Some(tx);
                    return Ok(i);
                }
                free.push(i);
            }
        }

        if free.is_empty() {
            return Err(Error::NoSeatsLeft);
        }

        free.shuffle(&mut thread_rng());

        let idx = free[0];

        // Add an entry for this `Peer` in the shared state map.
        self.clients[idx].addr = Some(addr);
        self.clients[idx].tx = Some(tx);
        self.clients[idx].name = name;

        Ok(idx)
    }
    async fn remove_client(&mut self, addr: SocketAddr) -> Result<(), Error> {
        for peer in self.clients.iter_mut() {
            if peer.addr == Some(addr) {
                peer.addr = None;
                peer.tx = None;
                return Ok(());
            }
        }

        Err(Error::PeerNotInList)
    }

    async fn send_state_update(&mut self) -> Result<(), Error> {
        for (p, peer) in self.clients.iter().enumerate() {
            if peer.addr.is_some() && peer.tx.is_some() {
                let message = self.to_statemessage(p);
                // if p == 0 { println!("Send StateMessage {:?}", message); }

                let tx = peer.tx.to_owned().unwrap();

                if let Err(e) = tx.send(message.clone()).await {
                    return Err(Error::SendError(e));
                }
            }
        }
        Ok(())
    }

    async fn broadcast(&mut self, sender: SocketAddr, message: Vec<u8>) -> Result<(), Error> {
        for peer in self.clients.iter() {
            if peer.addr.is_some() && peer.tx.is_some() {
                let tx = peer.tx.to_owned().unwrap();
                if let Err(e) = tx.send(message.clone()).await {
                    println!("broadcast error {:?}", e);
                }
                // println!("Send message {:?} to {}", message, peer.addr.unwrap());
            }
        }
        Ok(())
    }
}

async fn big2_handler(gs: Arc<Mutex<GameServerState>>, mut socket: TcpStream) -> Result<(), Error> {
    let remote_ip = socket.peer_addr()?;
    println!("big2_handler: New connection from {}", remote_ip);

    // Add an entry for this `Peer` in the shared state map.
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    let timeout_timer = time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout_timer);

    let mut joined = false;
    let mut idx: usize = 0;
    // Read the first line from the `LineCodec` stream to get the username.
    let mut buf = [0u8; 512];
    loop {
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
                                let mut b = gs.lock().await;
                                println!("Add client {} name {}", remote_ip, name.to_string());
                                idx = b.new_client(remote_ip, tx.clone(), name)?;
                                if b.gs.turn == -1 {
                                    b.gs.has_passed |= 1 << idx;
                                }
                                // b.gs.last_action = 0;
                                b.send_state_update().await?;
                                joined = true;

                                if b.gs.turn == -1 && b.gs.has_passed == 0xF {
                                    b.gs.deal(None);
                                    b.send_state_update().await?;
                                }
                            },
                            _ => { println!("Invalid packet! {:?}", buf);
                                let _ = socket.write_all(&"Invalid UTF8\n".as_bytes()).await?;
                            },
                        }
                    }
                }
            }
            _ = &mut timeout_timer => {
                socket.write(&"\nTimeout! Bye!\n".as_bytes()).await?;
                break;
            }
        }
    }

    if !joined {
        let mut b = gs.lock().await;
        b.remove_client(remote_ip).await?;
        b.gs.last_action = 0;
        b.send_state_update().await?;
        return Ok(());
    }

    loop {
        let mut buf = [0u8; 512];

        let hartbeat_timer = time::sleep(Duration::from_secs(5));
        tokio::pin!(hartbeat_timer);

        loop {
            tokio::select! {
                to_clt = rx.recv() => {
                    match to_clt {
                        Some(to_clt) =>
                        {
                            // println!("Write to client {:?}", to_clt);
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
                            let rec = muon::parse_packet(nbytes, &buf);

                            match rec {
                                Ok(s) => {
                                    {
                                        let mut g = gs.lock().await;
                                        match s {
                                            muon::StateMessageActions::Pass => {
                                                if g.gs.pass(idx as i32).is_ok() {
                                                    println!("{}. {} PASSED", idx, g.clients[idx].name.to_string());
                                                    g.send_state_update().await?;
                                                }
                                            },
                                            muon::StateMessageActions::Play(cards) => {
                                                if g.gs.play(idx as i32, cards).is_ok() {
                                                    println!("{}. {} PLAYS {}", idx, g.clients[idx].name.to_string(), big2rules::cards::cards_to_string(cards) );
                                                    g.send_state_update().await?;
                                                }
                                            },
                                            muon::StateMessageActions::Ready => {
                                                if g.gs.ready(idx as i32).is_ok() {
                                                    println!("{}. {} READY", idx, g.clients[idx].name.to_string());
                                                    g.send_state_update().await?;
                                                }
                                            },
                                            _ => (),
                                        }
                                    }
                                }
                                Err(_) => {
                                    println!("Error");
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
    }

    {
        let mut b = gs.lock().await;
        b.remove_client(remote_ip).await?;
        b.send_state_update().await?;
    }

    Ok(())
}

pub async fn start_server(listener: TcpListener) {
    let peers = Arc::new(Mutex::new(GameServerState::new(8)));

    let listener = listener;
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
