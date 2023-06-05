use crate::big2rules::{self, cards::Cards};
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use std::{convert::TryFrom, io, mem, thread};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc::{channel, Receiver, Sender},
};

use futures::executor::block_on;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum StateMessageActionType {
    Update = 0,
    Deal = 1,
    Play = 2,
    Pass = 3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    kind: u32,
    size: u32,
    pad: [u64; 256 / std::mem::size_of::<u64>()],
}

impl Message {
    #[must_use]
    pub fn new(kind: u32) -> Self {
        Self {
            kind,
            size: u32::try_from(std::mem::size_of::<Message>()).expect("Should Fit!"),
            pad: [0; 32],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinMessage {
    kind: u32,
    size: u32,
    magicnumber: u32,
    version: u32,
    name: muon::String16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateMessagePlayer {
    pub name: muon::String16,
    pub score: i32,
    pub num_cards: u32,
    pub delta_score: i32,
    pub is_ready: bool,
    pub has_passed_this_cycle: bool,
    pub padding: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DetectMessage {
    pub kind: u32,
    pub size: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayMessage {
    pub kind: u32,
    pub size: u32,
    pub cards: muon::InlineList8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateMessageAction {
    pub action_type: StateMessageActionType,
    pub player: i32,
    pub cards: muon::InlineList8,
    pub is_end_of_cycle: bool,
    pub padding: [u8; 3],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateMessage {
    pub kind: u32,
    pub size: u32,
    pub round: u32,
    pub num_rounds: u32,
    pub turn: i32, // -1 means in between rounds
    pub your_index: i32,
    pub your_hand: muon::InlineList16,
    pub players: [StateMessagePlayer; 4],
    pub board: muon::InlineList8,
    pub action: StateMessageAction,
}

pub enum GameUpdate<'a> {
    Deal {
        yourhand: Cards,
        to_act: PlayerID,
    },
    Play {
        acted: PlayerID,
        played: Cards,
        to_act: PlayerID,
    },
    EndRound {
        round_score: [i8; 4],
        ready: u8,
    },
    Update(&'a ServerStatePlayers),
    Full(&'a ServerState),
}

pub struct ServerStatePlayers {
    pub name: SmolStr,
    pub score: i16,
    pub num_cards: u8,
}

pub struct ServerState {
    pub round: u8,
    pub rounds: u8,
    pub turn: Option<PlayerID>,
    pub player_id: PlayerID,
    pub player_hand: Cards,
    pub players: [ServerStatePlayers; 4],
    pub board: Option<Cards>,
}

impl ServerState {
    #[must_use]
    pub fn from(sm: &StateMessage) -> Option<Self> {
        Some(ServerState {
            round: u8::try_from(sm.round).ok()?,
            rounds: u8::try_from(sm.num_rounds).ok()?,
            turn: PlayerID::try_from(sm.turn),
            player_id: PlayerID::try_from(sm.your_index)?,
            player_hand: (&sm.your_hand).try_into().ok()?,
            players: sm
                .players
                .iter()
                .filter_map(|player| {
                    Some(ServerStatePlayers {
                        name: player.name.as_str().ok()?.into(),
                        score: i16::try_from(player.score).ok()?,
                        num_cards: u8::try_from(player.num_cards).ok()?,
                    })
                })
                .collect::<Vec<ServerStatePlayers>>()
                .try_into()
                .ok()?,
            board: Some((sm.board).try_into().ok()?),
        })
        //None
    }
}

#[non_exhaustive]
pub struct PlayerID(pub u8);

impl PlayerID {
    pub fn try_from<T>(val: T) -> Option<Self>
    where
        u8: TryFrom<T>,
    {
        u8::try_from(val)
            .ok()
            .filter(|v| (0..=3).contains(v))
            .map(Self)
    }

    #[must_use]
    pub fn in_to<T>(&self) -> T
    where
        T: From<u8>,
    {
        self.0.into()
    }
}

impl StateMessage {
    #[must_use]
    pub fn new(init_buffer: Option<&[u8]>) -> Self {
        let buf: &[u8] = if let Some(b) = init_buffer {
            b
        } else {
            &[0; std::mem::size_of::<Self>()]
        };
        let mut sm: StateMessage =
            bincode::deserialize(buf).expect("Can't deserialize StateMessage");
        sm.size = u32::try_from(std::mem::size_of::<StateMessage>()).expect("Should Fit!");
        sm
    }
    #[must_use]
    pub fn current_player(&self) -> Option<usize> {
        PlayerID::try_from(self.turn).map(|p| p.in_to())
    }
    #[must_use]
    pub fn current_player_name(&self) -> Option<&str> {
        self.current_player()
            .and_then(|p| self.players.get(p))
            .and_then(|p| p.name.as_str().ok())
    }
    #[must_use]
    pub fn player_name(&self, p: i32) -> Option<&str> {
        PlayerID::try_from(p)
            .and_then(|p| self.players.get(p.in_to::<usize>()))
            .and_then(|p| p.name.as_str().ok())
    }
    #[must_use]
    pub fn action_msg(&self) -> u64 {
        let Some(player) = PlayerID::try_from(self.action.player) else {
            trace!(                "Strang: Some action but no results p{}: {:?}",
                self.turn,
                self.action.action_type
            );
            return 0xFFFF_FFFF_FFFF_FFFF;
        };
        let mut p: u64 = player.in_to();
        #[allow(clippy::cast_sign_loss)]
        let turn = self.turn as u64;
        p |= (turn & 0x7) << 4;

        match self.action.action_type {
            StateMessageActionType::Play => {
                let mut cards = TryInto::<Cards>::try_into(self.action.cards)
                    .expect("invalid cards")
                    .0;
                cards |= p;
                cards
            }
            StateMessageActionType::Pass => {
                let mut cards = TryInto::<Cards>::try_into(self.board)
                    .expect("invalid cards")
                    .0;
                cards |= 0x100;
                cards |= p;
                cards
            }
            StateMessageActionType::Update => {
                let mut ready: u64 = 0;
                for i in 0..self.players.len() {
                    if self.players[i].is_ready {
                        ready |= 0x1000 << (i * 4);
                    }
                }
                ready |= 0x800;
                ready
            }
            StateMessageActionType::Deal => {
                let mut cards = TryInto::<Cards>::try_into(&self.your_hand)
                    .expect("Valid hand")
                    .0;
                cards |= 0x400;
                cards |= PlayerID::try_from(self.your_index)
                    .expect("Shoud fit")
                    .in_to::<u64>()
                    & 0x7;
                #[allow(clippy::cast_sign_loss)]
                let turn = self.turn as u64;
                cards |= (turn & 0x7) << 4;
                cards
            }
        }
    }
}

pub mod muon {
    use crate::big2rules::{
        cards::{CardNum, Cards, ParseCardsError},
        rules::is_valid_hand,
    };

    use super::{big2rules, Deserialize, Serialize, TryFrom};

    // #[non_exhaustive]
    // pub struct Cards(pub u64);

    // impl Cards {
    //     #[must_use]
    //     pub fn from_hand(hand: u64) -> Option<Self> {
    //         // if hand != 0 {
    //         //     score_hand(hand)?;
    //         // }

    //         Some(Self(hand))
    //     }
    // }

    #[allow(clippy::copy_iterator)]
    impl Iterator for Cards {
        type Item = CardNum;

        fn next(&mut self) -> Option<Self::Item> {
            if self.0 == 0 {
                None
            } else {
                let val = self.0;
                let card_num = u8::try_from(val.trailing_zeros()).ok()?;
                let mask = 1 << card_num;
                self.0 = val ^ mask;

                CardNum::try_from(card_num)
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let size = usize::try_from(self.count_ones()).unwrap();
            (size, Some(size))
        }
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub struct String16 {
        data: [u8; 16],
        count: u32,
    }

    impl String16 {
        pub fn as_str(&self) -> Result<&str, &str> {
            let Some(cnt) = usize::try_from(self.count).ok().and_then(|v| if (0..=16).contains(&v) { Some(v) } else { None } ) else {
                return Err("Invalid string");
            };

            match core::str::from_utf8(&self.data[..cnt]) {
                Err(_) => Err("UTF8 Error"),
                Ok(st) => Ok(st),
            }
        }

        #[must_use]
        pub fn from_string(name: &str) -> Self {
            let mut name_str16_bytes = [0; 16];
            let name_bytes = name.as_bytes();

            let str_size = std::cmp::min(name_str16_bytes.len(), name_bytes.len());

            let nb = &name_bytes[..str_size];
            name_str16_bytes[..str_size].clone_from_slice(nb);

            String16 {
                count: u32::try_from(str_size).expect("str_size should fit i32"),
                data: name_str16_bytes,
            }
        }

        #[must_use]
        pub fn is_empty(&self) -> bool {
            self.count == 0
        }
    }

    #[cfg(kani)]
    mod verification {
        use super::*;

        #[kani::proof]
        pub fn kani_i8() {
            let ret = "                ";
            let cnt: u32 = kani::any();
            kani::assume(cnt < 18);
            let name = String16 {
                data: [32; 16],
                count: cnt,
            };
            let sname = name.as_str();
            if cnt > 16 {
                assert!(sname.is_err())
            } else {
                assert!(sname.is_ok());
                assert_eq!(sname.unwrap().len(), cnt as usize);
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
    pub struct InlineList8 {
        pub data: [u8; 8],
        pub count: u32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct InlineList16 {
        pub data: [u8; 16],
        pub count: u32,
    }

    impl TryInto<Cards> for &InlineList16 {
        type Error = ParseCardsError;

        fn try_into(self) -> Result<Cards, Self::Error> {
            let mut cards = Cards::default();
            if self.count < 14 {
                if let Ok(count) = usize::try_from(self.count) {
                    for c in 0..count {
                        let card = self.data[c];
                        cards |= card_from_byte(card);
                    }
                }
                Ok(cards)
            } else {
                Err(ParseCardsError::InvalidInput)
            }
        }
    }

    impl TryFrom<u64> for InlineList8 {
        type Error = &'static str;

        fn try_from(hand: u64) -> Result<Self, Self::Error> {
            let mut cards = InlineList8 {
                data: [0; 8],
                count: 0,
            };

            if hand != 0 && !is_valid_hand(hand) {
                return Err("Invalid Hand!");
            }

            cards.count = hand.count_ones();

            for (card, data) in Cards(hand).zip(&mut cards.data) {
                *data = cards_to_byte(card);
            }
            Ok(cards)
        }
    }

    impl TryInto<Cards> for InlineList8 {
        type Error = ParseCardsError;

        fn try_into(self) -> Result<Cards, Self::Error> {
            let Some(count) = usize::try_from(self.count).ok().and_then(|v| if (0..=self.data.len()).contains(&v) { Some(v)} else { None }) else {
                return Err(ParseCardsError::InvalidInput);
            };
            let mut cards = Cards::default();
            for &card in &self.data[0..count] {
                let c = card & 0b1100_1111;
                if !(2..=14).contains(&c) {
                    return Err(ParseCardsError::InvalidInput);
                }
                cards |= card_from_byte(card);
            }
            Ok(cards)
        }
    }

    #[must_use]
    pub fn card_from_byte(byte: u8) -> u64 {
        let card = u64::from(byte);
        let suit = 1 << ((card & 0x30) >> 4);
        let mut rank = card & 0xF;
        if rank == 2 {
            rank = 15;
        }
        suit << (rank << 2)
    }

    #[must_use]
    pub fn cards_to_byte(card: CardNum) -> u8 {
        let mut rank = card.rank() as u8;
        if rank == big2rules::cards::CardRank::TWO as u8 {
            rank = 2;
        }
        let suit = match card.suit() {
            big2rules::cards::CardSuit::Clubs => 0,
            big2rules::cards::CardSuit::Diamonds => 1,
            big2rules::cards::CardSuit::Hearts => 2,
            big2rules::cards::CardSuit::Spades => 3,
        } << 4;
        rank | suit
    }

    #[test]
    fn cards_iter() {
        let hand = 0;
        let cards = Cards(hand).collect::<Vec<CardNum>>();
        assert_eq!(cards, vec![]);
    }

    #[test]
    fn string16_test() {
        let valid = "";
        let str16 = String16::from_string(valid);
        assert_eq!(
            str16,
            String16 {
                data: [0; 16],
                count: 0
            }
        );

        let valid = "Name";
        let str16 = String16::from_string(valid);
        assert_eq!(
            str16,
            String16 {
                data: [b'N', b'a', b'm', b'e', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                count: 4
            }
        );

        let valid = "Name  Full  Size";
        let str16 = String16::from_string(valid);
        assert_eq!(
            str16,
            String16 {
                data: [
                    b'N', b'a', b'm', b'e', b' ', b' ', b'F', b'u', b'l', b'l', b' ', b' ', b'S',
                    b'i', b'z', b'e'
                ],
                count: 16
            }
        );

        let valid = "LongNameSuperLarge";
        let str16 = String16::from_string(valid);
        assert_eq!(
            str16,
            String16 {
                data: [
                    b'L', b'o', b'n', b'g', b'N', b'a', b'm', b'e', b'S', b'u', b'p', b'e', b'r',
                    b'L', b'a', b'r'
                ],
                count: 16
            }
        );

        let invalid = "éééééééééééééééé";
        let str16 = String16::from_string(invalid);
        assert_eq!(
            str16,
            String16 {
                data: [
                    195, 169, 195, 169, 195, 169, 195, 169, 195, 169, 195, 169, 195, 169, 195, 169
                ],
                count: 16
            }
        );
    }
}

pub mod common {
    pub const PORT: u16 = 27191;
    pub const VERSION: u32 = 6;
    pub const MAGICNUMBER: u32 = 0x3267_6962;
    pub const BUFSIZE: usize = 4096;
}

pub mod client {
    use std::net::ToSocketAddrs;

    use super::*;

    const DM_SIZE: usize = mem::size_of::<client::DetectMessage>();
    const M_SIZE: usize = mem::size_of::<Message>();
    const SM_SIZE: usize = mem::size_of::<StateMessage>();

    pub struct TcpClient {
        id: Option<thread::JoinHandle<()>>,
        pub rx: Receiver<StateMessage>,
        tx: Sender<Vec<u8>>,
    }

    async fn thread_tcp(mut ts: TcpStream, tx: &Sender<StateMessage>, mut rx: Receiver<Vec<u8>>) {
        let mut buffer = [0u8; common::BUFSIZE];

        loop {
            tokio::select! {
                recv = rx.recv() =>
                        if let Some(data) = recv {
                            trace!("TCP: PUSH: {:x?}", data);
                            if let Err(e) = ts.write_all(&data).await {
                                error!("TCP: Error write. {}", e);
                            }
                        } else {
                            info!("Recv channel closed! Shutdown thread");
                            break;
                        },
                n_bytes = ts.read(&mut buffer) => {
                    let mut n_bytes = match n_bytes {
                        Ok(0) => {
                            info!("Connection Closed!");
                            break;
                        }
                        Ok(n) => n,
                        Err(e) => {
                            // if readtimeout then continue.
                            // if e.kind() == io::ErrorKind::TimedOut {
                            //     ();
                            // }
                            error!("TCP: RX error {:?}", e);
                            break;
                        }
                    };

                    let mut pos: usize = 0;

                    while n_bytes >= DM_SIZE {
                        let dm: client::DetectMessage = match bincode::deserialize(&buffer[pos..]) {
                            Ok(dm) => dm,
                            Err(e) => {
                                error!("TCP: DM decode error: {e:?}");
                                break;
                            }
                        };

                        let Ok(msg_size) = usize::try_from(dm.size) else { n_bytes += 4; continue; };

                        match (dm.kind, msg_size) {
                            // Update
                            (5, SM_SIZE) => {
                                if n_bytes >= SM_SIZE {
                                    let buf = buffer[pos..pos + SM_SIZE].to_vec();

                                    trace!("TCP: P{} B{} SM: {:?}", pos, n_bytes, buf);

                                    let sm = StateMessage::new(Some(&buf));

                                    if let Err(e) = tx.send(sm).await {
                                        error!("TCP: MPSC TX ERROR {e:?}");
                                        break;
                                    }
                                    pos += SM_SIZE;
                                    n_bytes -= SM_SIZE;
                                }
                            }
                            // HeartbeatMessage
                            (6, M_SIZE) => {
                                info!("TCP: Packet Hearthbeat");
                                pos += M_SIZE;
                                n_bytes -= M_SIZE;
                            }
                            _ => {
                                if msg_size == buffer.len() {
                                    error!("TCP: GET: {:x?}", &buffer[0..msg_size]);
                                } else {
                                    error!(
                                        "TCP: Invalid packet! - Bytes {} Kind {} Size {} - {:?} -",
                                        n_bytes,
                                        dm.kind,
                                        dm.size,
                                        &buffer[0..n_bytes]
                                    );
                                }
                            }
                        }
                    }
                },
            }
        }
    }

    pub fn disconnect(mut tc: TcpClient) {
        debug!("Shutdown tcp thread!");
        drop(tc.tx);
        if let Some(thread) = tc.id.take() {
            if let Err(e) = thread.join() {
                error!("Thread shutdown issue: {e:?}");
            };
        }
    }

    impl TcpClient {
        pub async fn connect(remote_addr: &str) -> Result<TcpClient, io::Error> {
            let mut servers = remote_addr
                .to_socket_addrs()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            loop {
                let server = servers.next();
                let Some(l) = server else { break };
                info!("Connecting to {:?}", l);
                //let ret = TcpStream::connect_timeout(&l, Duration::from_secs(1));
                let ret = TcpStream::connect(&l).await;
                match ret {
                    Err(_) => continue,
                    Ok(s) => {
                        // s.set_read_timeout(Some(Duration::from_millis(10)))?;
                        let (tx, rx) = channel(10);
                        let (tx1, rx1) = channel(10);

                        info!("Connected to {:?}!", s.peer_addr());

                        let tcp_thread = thread::Builder::new().name("big2_tcp".into());
                        let id = tcp_thread.spawn(move || block_on(thread_tcp(s, &tx, rx1)))?;

                        // if let Err(e) = id {
                        //     println!("Can't create thread {}!", e);
                        //     return Err(e);
                        // }

                        return Ok(TcpClient {
                            rx,
                            tx: tx1,
                            id: Some(id),
                        });
                    }
                }
            }
            info!("Unable to connect!");
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Can't Connect Timeout!",
            ))
        }

        pub async fn action_pass(&mut self) -> Result<(), io::Error> {
            let sm = Message::new(3);
            let byte_buf =
                bincode::serialize(&sm).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            // println!("{:?}", byte_buf);
            if self.tx.send(byte_buf).await.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(())
        }

        pub async fn action_ready(&mut self) -> Result<(), io::Error> {
            let sm = Message::new(4);
            let byte_buf =
                bincode::serialize(&sm).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            if self.tx.send(byte_buf).await.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(())
        }

        pub async fn action_play(&mut self, cards: u64) -> Result<(), io::Error> {
            let sm = PlayMessage {
                kind: 2,
                size: u32::try_from(mem::size_of::<PlayMessage>()).expect("Should fit u32!"),
                cards: muon::InlineList8::try_from(cards)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
            };
            let byte_buf =
                bincode::serialize(&sm).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            // println!("action_play: {:x?}", byte_buf);
            if self.tx.send(byte_buf).await.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(())
        }

        pub async fn send_join_msg(&mut self, name: &str) -> Result<(), io::Error> {
            let jm = JoinMessage {
                kind: 1,
                size: u32::try_from(mem::size_of::<JoinMessage>()).expect("Should fit u32!"),
                magicnumber: common::MAGICNUMBER,
                version: common::VERSION,
                name: muon::String16::from_string(name),
            };

            // Send Join Message.
            let jmb =
                bincode::serialize(&jm).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            if self.tx.send(jmb).await.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DM_SIZE: usize = mem::size_of::<self::DetectMessage>();
    const SM_SIZE: usize = mem::size_of::<StateMessage>();
    const M_SIZE: usize = mem::size_of::<Message>();

    // #[test]
    // fn a_connect() {
    //  let name = String::from("René");
    //  let JM: JoinMessage = client::joinMessage(&name);
    //  let JMB = bincode::serialize(&JM).unwrap();
    //  println!("{:x?}", JMB);
    //  //                       12356789T123456
    //  let name = String::from("René to long to5123123");
    //  let JM: JoinMessage = client::joinMessage(&name);
    //  let JMB = bincode::serialize(&JM).unwrap();
    //  println!("{:x?}", JMB);
    // }
    /*  #[test]
    fn b_connect() {
        let sm_size = std::mem::size_of::<StateMessage>();
        let eb = &[0u8; std::mem::size_of::<StateMessage>()];
        let mut sm: StateMessage = bincode::deserialize(eb).unwrap();
        sm.size = sm_size as u32;
        sm.action = StateMessageActionType::play;

        let smb = bincode::serialize(&sm).unwrap();
        println!("{:x?}", smb);
    } */
    #[test]
    fn c_statemessage_respone() {
        let buffer: &[u8] = &[
            5, 0, 0, 0, 0xe0, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0x15, 7,
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

        let sm = StateMessage::new(Some(buffer));

        let mut mycards: u64 = 0;
        for c in 0..sm.your_hand.count as usize {
            let card = u64::from(sm.your_hand.data[c]);
            let suit = 1 << ((card & 0x30) >> 4);
            let mut rank = card & 0xF;
            if rank == 2 {
                rank = 15;
            }
            mycards |= suit << (rank << 2);
        }

        assert_eq!(mycards, 0x10a4_c18c_9020_0000);

        let mut mycards: u64 = 0;
        for (_idx, data) in (0..sm.your_hand.count).zip(sm.your_hand.data) {
            mycards |= muon::card_from_byte(data);
        }
        assert_eq!(mycards, 0x10a4_c18c_9020_0000);
    }
    #[test]
    fn d_statemessage_respone() {
        let cards: u64 = 0b111 << 12;
        let sm = PlayMessage {
            kind: 2,
            size: u32::try_from(mem::size_of::<PlayMessage>()).unwrap(),
            cards: muon::InlineList8::try_from(cards).unwrap(),
        };
        let byte_buf = bincode::serialize(&sm).unwrap();
        let packet: &[u8] = &[
            2, 0, 0, 0, 20, 0, 0, 0, 3, 19, 35, 0, 0, 0, 0, 0, 3, 0, 0, 0,
        ];
        assert_eq!(byte_buf, packet);
    }

    #[test]
    fn muon_inline8_try_from_valid() {
        // No cards
        let hand = Cards::from(0);
        let muon_hand = muon::InlineList8::try_from(hand.0);
        assert!(muon_hand.is_ok());
        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        let muon_hand = muon_hand.unwrap();
        assert_eq!(muon_hand, il8);
        let cards = il8.try_into();
        assert!(cards.is_ok());
        assert_eq!(hand, cards.unwrap());

        // lowest card 3d
        let hand = Cards::from(0x1000);
        let muon_hand = muon::InlineList8::try_from(hand.0);
        assert!(muon_hand.is_ok());
        let il8 = muon::InlineList8 {
            data: [0x3, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        let muon_hand = muon_hand.unwrap();
        assert_eq!(muon_hand, il8);
        let cards = il8.try_into();
        assert!(cards.is_ok());
        assert_eq!(hand, cards.unwrap());

        // higest card 2s
        let hand = Cards::from(0x8000_0000_0000_0000);
        let muon_hand = muon::InlineList8::try_from(hand.0);
        assert!(muon_hand.is_ok());
        let il8 = muon::InlineList8 {
            data: [0x32, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        let muon_hand = muon_hand.unwrap();
        assert_eq!(muon_hand, il8);
        let cards = il8.try_into();
        assert!(cards.is_ok());
        assert_eq!(hand, cards.unwrap());

        let hand = Cards::from(0xF100_0000_0000_0000);
        let il8 = muon::InlineList8 {
            data: [14, 2, 18, 34, 50, 0, 0, 0],
            count: 5,
        };
        let muon_hand = muon::InlineList8::try_from(hand.0).unwrap();
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon_hand.try_into().unwrap());

        let hand = Cards::from(0x1F000);
        let il8 = muon::InlineList8 {
            data: [3, 19, 35, 51, 4, 0, 0, 0],
            count: 5,
        };
        let muon_hand = muon::InlineList8::try_from(hand.0).unwrap();
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon_hand.try_into().unwrap());

        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        assert_eq!(TryInto::<Cards>::try_into(il8), Ok(Cards::default()));
    }

    #[test]
    fn muon_inline8_try_from_invalid() {
        let hand: u64 = 0xFF000;
        assert!(muon::InlineList8::try_from(hand).is_err());

        let hand: u64 = 0xF000;
        assert!(muon::InlineList8::try_from(hand).is_err());

        let hand: u64 = 0x1;
        assert!(muon::InlineList8::try_from(hand).is_err());

        let hand: u64 = 0x1001;
        assert!(muon::InlineList8::try_from(hand).is_err());

        let hand: u64 = 0x1100;
        assert!(muon::InlineList8::try_from(hand).is_err());
    }

    #[test]
    fn muon_inline8_into_cards_invalid() {
        let il8 = muon::InlineList8 {
            data: [0xFF; 8],
            count: 1,
        };
        assert!(TryInto::<Cards>::try_into(il8).is_err());

        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 9,
        };
        assert!(TryInto::<Cards>::try_into(il8).is_err());

        let il8 = muon::InlineList8 {
            data: [0xFF, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(TryInto::<Cards>::try_into(il8).is_err());

        let il8 = muon::InlineList8 {
            data: [0x4d, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(TryInto::<Cards>::try_into(il8).is_err());

        let il8 = muon::InlineList8 {
            data: [0x3f, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(TryInto::<Cards>::try_into(il8).is_err());

        let il8 = muon::InlineList8 {
            data: [0x3d, 0, 0, 0, 0, 0, 0, 0],
            count: 2,
        };
        assert!(TryInto::<Cards>::try_into(il8).is_err());
    }
    #[test]
    fn statemessage_current_players_names() {
        let buffer: &[u8] = &[
            5, 0, 0, 0, 0xe0, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0x15, 7,
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
        let mut sm = StateMessage::new(Some(buffer));

        assert_eq!(sm.action_msg(), 0x0111_1800);

        assert_eq!(sm.current_player().unwrap(), 0);
        assert_eq!(sm.current_player_name().unwrap(), "Tikkie");
        sm.turn = -1;
        assert!(sm.current_player().is_none());
        assert!(sm.current_player_name().is_none());
        sm.turn = 1;
        assert_eq!(sm.current_player_name().unwrap(), "host");
        sm.turn = 2;
        assert_eq!(sm.current_player_name().unwrap(), "Rene123");
        sm.turn = 3;
        assert_eq!(sm.current_player_name().unwrap(), "Rene");
        sm.turn = 4;
        assert!(sm.current_player().is_none());
    }

    #[test]
    fn trail_test_ready() {
        // 2 person in the game.
        // Game end and 3 players are ready
        let buffer: &[u8] = &[
            5, 0, 0, 0, 224, 0, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 255, 255, 255, 255, 2, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 68, 97, 78, 111, 111, 78, 101, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 28, 0, 0, 0, 7, 0, 0, 0, 249, 255, 255, 255, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 219, 255, 255, 255,
            2, 0, 0, 0, 254, 255, 255, 255, 0, 0, 0, 0, 82, 101, 110, 101, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 4, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255,
            255, 1, 0, 0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let sm = StateMessage::new(Some(buffer));
        assert_eq!(sm.action_msg(), 0x0110_1800);
    }

    #[test]
    fn trail_test_play_hand() {
        // PLAY:             Rene: 3♦ 4♦ 5♣ 6♦ 7♣
        let buffer: &[u8] = &[
            0x5, 0x0, 0x0, 0x0, 0xe0, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x3,
            0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x29, 0x39, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x44, 0x61, 0x4e, 0x6f, 0x6f, 0x4e,
            0x65, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x1c, 0x0, 0x0,
            0x0, 0x4, 0x0, 0x0, 0x0, 0xf7, 0xff, 0xff, 0xff, 0x1, 0x0, 0x0, 0x0, 0x52, 0x65, 0x6d,
            0x63, 0x6f, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x5, 0x0, 0x0, 0x0,
            0xe8, 0xff, 0xff, 0xff, 0x7, 0x0, 0x0, 0x0, 0xfc, 0xff, 0xff, 0xff, 0x1, 0x0, 0x0, 0x0,
            0x52, 0x65, 0x6e, 0x65, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x4, 0x0, 0x0, 0x0, 0xd3, 0xff, 0xff, 0xff, 0x2, 0x0, 0x0, 0x0, 0xea, 0xff, 0xff, 0xff,
            0x1, 0x0, 0x0, 0x0, 0x4e, 0x67, 0x6f, 0x48, 0x6f, 0x6e, 0x67, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x29, 0x0, 0x0, 0x0, 0x5, 0x0, 0x0, 0x0, 0x23,
            0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x3, 0x4, 0x15, 0x6, 0x17, 0x0, 0x0,
            0x0, 0x5, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];
        let sm = StateMessage::new(Some(buffer));
        let cards = sm.action.cards.try_into().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x2121_1032);
        assert_eq!(Cards::from(trail & 0xFFFF_FFFF_FFFF_F000), cards);

        // PLAY:             NG: FULLHOUSE
        let buffer: &[u8] = &[
            0x5, 0x0, 0x0, 0x0, 0xe0, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0xff,
            0xff, 0xff, 0xff, 0x2, 0x0, 0x0, 0x0, 0x29, 0x39, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x44, 0x61, 0x4e, 0x6f, 0x6f,
            0x4e, 0x65, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x18, 0x0,
            0x0, 0x0, 0x4, 0x0, 0x0, 0x0, 0xfc, 0xff, 0xff, 0xff, 0x0, 0x0, 0x0, 0x0, 0x52, 0x65,
            0x6d, 0x63, 0x6f, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x5, 0x0, 0x0,
            0x0, 0xe1, 0xff, 0xff, 0xff, 0x7, 0x0, 0x0, 0x0, 0xf9, 0xff, 0xff, 0xff, 0x0, 0x0, 0x0,
            0x0, 0x52, 0x65, 0x6e, 0x65, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x4, 0x0, 0x0, 0x0, 0xd1, 0xff, 0xff, 0xff, 0x2, 0x0, 0x0, 0x0, 0xfe, 0xff, 0xff,
            0xff, 0x0, 0x0, 0x0, 0x0, 0x4e, 0x67, 0x6f, 0x48, 0x6f, 0x6e, 0x67, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x36, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0xd, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x3, 0x4, 0x15, 0x6, 0x17, 0x0, 0x0, 0x0, 0x5,
            0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x14, 0x24, 0x34, 0x5, 0x25,
            0x0, 0x0, 0x0, 0x5, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];
        let sm = StateMessage::new(Some(buffer));
        let cards = sm.action.cards.try_into().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x005E_0073);
        assert_eq!(Cards::from(trail & 0xFFFF_FFFF_FFFF_F000), cards);
    }

    #[test]
    fn trail_test_pass() {
        // PLAY:          NH: PASSED
        let buffer: &[u8] = &[
            0x5, 0x0, 0x0, 0x0, 0xe0, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x3, 0x4, 0x15, 0x6, 0x17, 0x29, 0x39, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x44, 0x61, 0x4e, 0x6f, 0x6f,
            0x4e, 0x65, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x1c, 0x0,
            0x0, 0x0, 0x4, 0x0, 0x0, 0x0, 0xf7, 0xff, 0xff, 0xff, 0x1, 0x0, 0x0, 0x0, 0x52, 0x65,
            0x6d, 0x63, 0x6f, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x5, 0x0, 0x0,
            0x0, 0xe8, 0xff, 0xff, 0xff, 0x7, 0x0, 0x0, 0x0, 0xfc, 0xff, 0xff, 0xff, 0x1, 0x1, 0x0,
            0x0, 0x52, 0x65, 0x6e, 0x65, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x4, 0x0, 0x0, 0x0, 0xd3, 0xff, 0xff, 0xff, 0x7, 0x0, 0x0, 0x0, 0xea, 0xff, 0xff,
            0xff, 0x1, 0x0, 0x0, 0x0, 0x4e, 0x67, 0x6f, 0x48, 0x6f, 0x6e, 0x67, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x29, 0x0, 0x0, 0x0, 0x5, 0x0, 0x0, 0x0,
            0x23, 0x0, 0x0, 0x0, 0x1, 0x1, 0x0, 0x0, 0x12, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1,
            0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];
        let sm = StateMessage::new(Some(buffer));
        let cards = sm.board.try_into().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x2000_0000_0000_0103);
        assert_eq!(Cards::from(trail & 0xFFFF_FFFF_FFFF_F000), cards);
    }

    #[test]
    fn message_struct_size() {
        assert_eq!(std::mem::size_of::<Message>(), 264);
    }

    #[test]
    fn statemessage_struct_size() {
        assert_eq!(std::mem::size_of::<StateMessage>(), 224);
    }

    #[test]
    fn parse_packet() {
        let buffer: &[u8] = &[
            5, 0, 0, 0, 224, 0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 255, 255, 255, 255, 3, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 66, 79, 84, 48, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 240, 255, 255, 255, 5, 0, 0, 0, 251, 255, 255, 255, 0, 0,
            0, 0, 66, 79, 84, 49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 14, 0, 0, 0, 6,
            0, 0, 0, 250, 255, 255, 255, 1, 0, 0, 0, 66, 79, 84, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 4, 0, 0, 0, 239, 255, 255, 255, 7, 0, 0, 0, 249, 255, 255, 255, 1, 0, 0, 0, 66,
            79, 84, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 19, 0, 0, 0, 0, 0, 0, 0,
            18, 0, 0, 0, 1, 0, 0, 0, 59, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 224, 0, 0, 0, 7, 0, 0, 0,
            8, 0, 0, 0, 255, 255, 255, 255, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 66, 79, 84, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 240,
            255, 255, 255, 5, 0, 0, 0, 251, 255, 255, 255, 1, 0, 0, 0, 66, 79, 84, 49, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 14, 0, 0, 0, 6, 0, 0, 0, 250, 255, 255, 255, 1, 0,
            0, 0, 66, 79, 84, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 239, 255, 255,
            255, 7, 0, 0, 0, 249, 255, 255, 255, 1, 0, 0, 0, 66, 79, 84, 51, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 4, 0, 0, 0, 19, 0, 0, 0, 0, 0, 0, 0, 18, 0, 0, 0, 1, 0, 0, 0, 59, 0, 0,
            0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 5, 0, 0, 0, 224, 0, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0,
            19, 37, 23, 24, 56, 9, 10, 42, 11, 43, 60, 13, 46, 0, 0, 0, 13, 0, 0, 0, 66, 79, 84,
            48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 240, 255, 255, 255, 13, 0, 0, 0,
            251, 255, 255, 255, 1, 0, 0, 0, 66, 79, 84, 49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4,
            0, 0, 0, 14, 0, 0, 0, 13, 0, 0, 0, 250, 255, 255, 255, 1, 0, 0, 0, 66, 79, 84, 50, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 239, 255, 255, 255, 13, 0, 0, 0, 249, 255,
            255, 255, 1, 0, 0, 0, 66, 79, 84, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0,
            19, 0, 0, 0, 13, 0, 0, 0, 18, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 8,
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut n_bytes = buffer.len();
        let mut pos: usize = 0;

        while n_bytes >= DM_SIZE {
            println!("Bytes {n_bytes} Pos {pos}");

            let dm: self::DetectMessage =
                bincode::deserialize(&buffer[pos..pos + DM_SIZE]).unwrap();

            let msg_size = usize::try_from(dm.size).unwrap();

            // Update
            if dm.kind == 5 && msg_size == SM_SIZE {
                if n_bytes < SM_SIZE {
                    break;
                }

                let buf = buffer[pos..pos + SM_SIZE].to_vec();

                info!("TCP: P{} B{} SM: {:?}", pos, n_bytes, buf);
                pos += SM_SIZE;
                n_bytes -= SM_SIZE;
                continue;
            }

            // HeartbeatMessage
            if dm.kind == 6 && msg_size == M_SIZE {
                info!("TCP: <T>HB");
                pos += M_SIZE;
                n_bytes -= M_SIZE;
                continue;
            }

            if (msg_size) == buffer.len() {
                error!("TCP: GET: {:x?}", &buffer[0..msg_size]);
            } else {
                error!(
                    "TCP: Invalid packet! - Bytes {} Kind {} Size {} - {:?} -",
                    n_bytes,
                    dm.kind,
                    dm.size,
                    &buffer[0..n_bytes]
                );
            }
        }
    }

    #[test]
    fn legacy_to_new_statemessage_current_players_names() {
        let buffer: &[u8] = &[
            5, 0, 0, 0, 0xe0, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0x15, 7,
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
        let mut sm = StateMessage::new(Some(buffer));

        assert_eq!(sm.action_msg(), 0x0111_1800);

        assert_eq!(sm.current_player().unwrap(), 0);
        assert_eq!(sm.current_player_name().unwrap(), "Tikkie");
        sm.turn = -1;
        assert!(sm.current_player().is_none());
        assert!(sm.current_player_name().is_none());
        sm.turn = 1;
        assert_eq!(sm.current_player_name().unwrap(), "host");
        sm.turn = 2;
        assert_eq!(sm.current_player_name().unwrap(), "Rene123");
        sm.turn = 3;
        assert_eq!(sm.current_player_name().unwrap(), "Rene");
        sm.turn = 4;
        assert!(sm.current_player().is_none());

        let new = ServerState::from(&sm).unwrap();

        assert_eq!(new.players[0].name.as_str(), "Tikkie");
        assert_eq!(new.players[1].name.as_str(), "host");
        assert_eq!(new.players[2].name.as_str(), "Rene123");
        assert_eq!(new.players[3].name.as_str(), "Rene");
    }
}
