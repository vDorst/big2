use crate::big2rules;
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};

use std::{
    convert::TryFrom,
    io::{self, Read, Write},
    mem,
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc::{Receiver, Sender},
    thread,
    time::Duration,
};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum StateMessageActionType {
    UPDATE = 0,
    DEAL = 1,
    PLAY = 2,
    PASS = 3,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    kind: u32,
    size: u32,
    pad: [u64; 256 / std::mem::size_of::<u64>()],
}

impl Message {
    pub fn new(kind: u32) -> Self {
        Self {
            kind: kind,
            size: std::mem::size_of::<Message>() as u32,
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
    pub num_cards: i32,
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

impl StateMessage {
    pub fn new(init_buffer: Option<&[u8]>) -> Self {
        let buf: &[u8];
        if let Some(b) = init_buffer {
            // assert!(b.len() < std::mem::size_of::<Self>());
            buf = b;
        } else {
            buf = &[0; std::mem::size_of::<Self>()];
        }
        let mut sm: StateMessage = bincode::deserialize(&buf).unwrap();
        sm.size = mem::size_of::<StateMessage>() as u32;
        sm
    }
    pub fn current_player(&self) -> Option<usize> {
        if self.turn == -1 || self.turn < 0 || self.turn > 3 {
            return None;
        }
        Some(self.turn as usize)
    }
    pub fn current_player_name(&self) -> Option<String> {
        match self.current_player() {
            None => return None,
            Some(p) => {
                return Some(self.players[p].name.to_string());
            }
        }
    }
    pub fn player_name(&self, p: i32) -> Option<String> {
        if p < 0 || p > 3 {
            return None;
        }
        return Some(self.players[p as usize].name.to_string());
    }
    pub fn action_msg(&self) -> u64 {
        let player = self.action.player;
        let name = self.player_name(player);
        if name.is_none() {
            trace!(
                "Strang: Some action but no results p{}: {:?}",
                player,
                self.action.action_type
            );
            return 0xFFFF_FFFF_FFFF_FFFF;
        }
        let mut p = (player as u64) & 0x7;
        p |= ((self.turn as u64) & 0x7) << 4;

        match self.action.action_type {
            StateMessageActionType::PLAY => {
                let mut cards = self.action.cards.into_card().unwrap();
                cards |= p;
                return cards;
            }
            StateMessageActionType::PASS => {
                let mut cards = self.board.into_card().unwrap();
                cards |= 0x100;
                cards |= p;
                return cards;
            }
            StateMessageActionType::UPDATE => {
                let mut ready: u64 = 0;
                for i in 0..4 {
                    if self.players[i].is_ready {
                        ready |= 0x1000 << (i * 4);
                    }
                }
                ready |= 0x800;
                return ready;
            }
            StateMessageActionType::DEAL => {
                let mut cards = self.your_hand.to_card();
                cards |= 0x400;
                cards |= self.your_index as u64 & 0x7;
                cards |= ((self.turn as u64) & 0x7) << 4;
                return cards;
            }
        };
    }
}

pub mod muon {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct String16 {
        pub data: [u8; 16],
        pub count: i32,
    }

    impl String16 {
        pub fn to_string(&self) -> String {
            let mut s = String::with_capacity(16);
            if self.count < 0 || self.count > 16 {
                s.push_str("Invalid string");
                return s;
            }

            let cnt: usize = self.count as usize;
            let s_ret = String::from_utf8(self.data[..cnt].to_vec());
            match s_ret {
                Err(_) => s.push_str("Can't convert"),
                Ok(st) => s = st,
            }
            return s;
        }

        pub fn from_string(name: &String) -> Self {
            let str_size = std::cmp::min(name.len(), 16);
            let mut name_bytes: [u8; 16] = [0; 16];
            let nb = &name.as_bytes()[..str_size];
            name_bytes[..str_size].clone_from_slice(nb);
            String16 {
                count: str_size as i32,
                data: name_bytes,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct InlineList16 {
        pub data: [u8; 16],
        pub count: i32,
    }

    #[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
    pub struct InlineList8 {
        pub data: [u8; 8],
        pub count: i32,
    }

    impl InlineList16 {
        pub fn to_card(&self) -> u64 {
            let mut cards: u64 = 0;
            if self.count > 0 && self.count < 14 {
                for c in 0..self.count as usize {
                    let card = self.data[c];
                    cards |= card_from_byte(card);
                }
            }
            return cards;
        }
    }

    impl TryFrom<u64> for InlineList8 {
        type Error = &'static str;

        fn try_from(hand: u64) -> Result<Self, Self::Error> {
            let mut cards = InlineList8 {
                data: [0; 8],
                count: 0,
            };

            let num_cards = hand.count_ones();
            if num_cards > 6 || num_cards == 4 || hand & 0xFFF != 0 {
                return Err("Invalid Hand!");
            }

            cards.count = num_cards as i32;

            let mut hand = hand;
            let mut p: usize = 0;
            while hand != 0 {
                let zeros = hand.trailing_zeros() as u64;

                let mask = 1 << zeros;
                hand ^= mask;
                cards.data[p] = cards_to_byte(mask);
                p += 1;
            }
            Ok(cards)
        }
    }

    impl InlineList8 {
        // pub fn to_card(&self) -> u64 {
        //     self.into_card().unwrap()
        // }
        pub fn into_card(&self) -> Result<u64, &'static str> {
            if self.count < 0 || self.count > 8 {
                return Err("Count out-of-range!");
            }
            let mut cards: u64 = 0;
            for c in 0..self.count as usize {
                let card = self.data[c];
                let c = card & 0b1100_1111;
                if c < 2 || c > 14 {
                    return Err("Card value out-of-range!");
                }
                cards |= card_from_byte(card);
            }
            Ok(cards)
        }
    }

    pub fn card_from_byte(byte: u8) -> u64 {
        let card = byte as u64;
        let suit = 1 << ((card & 0x30) >> 4);
        let mut rank = card & 0xF;
        if rank == 2 {
            rank = 15
        }
        return suit << (rank << 2);
    }

    pub fn cards_to_byte(card: u64) -> u8 {
        let mut rank = big2rules::cards::has_rank_idx(card);
        if rank == big2rules::cards::Rank::TWO {
            rank = 2;
        }
        let suit = (big2rules::cards::card_selected(card) & 0x3) << 4;
        return (rank | suit) as u8;
    }
}

pub mod common {
    pub const PORT: u16 = 27191;
    pub const VERSION: u32 = 6;
    pub const MAGICNUMBER: u32 = 0x3267_6962;
    pub const BUFSIZE: usize = 512;
}

pub mod client {
    use super::*;

    pub struct TcpClient {
        id: Option<thread::JoinHandle<()>>,
        rx: Receiver<Vec<u8>>,
        tx: Sender<Vec<u8>>,
    }

    fn thread_tcp(mut ts: TcpStream, tx: Sender<Vec<u8>>, rx: Receiver<Vec<u8>>) {
        let mut buffer = [0; common::BUFSIZE];

        loop {
            let tx_data = rx.try_recv();
            if let Err(e) = tx_data {
                if e == std::sync::mpsc::TryRecvError::Disconnected {
                    error!("TCP: TX channel disconnected");
                    break;
                }
            }
            if let Ok(data) = tx_data {
                trace!("TCP: PUSH: {:x?}", data);
                let ret = ts.write(&data);
                if let Err(e) = ret {
                    error!("TCP: Error write. {}", e);
                }
            }

            let ret = ts.read(&mut buffer);

            if let Err(e) = ret {
                // if readtimeout then continue.
                if e.kind() == io::ErrorKind::TimedOut {
                    continue;
                }
                if e.kind() == io::ErrorKind::WouldBlock {
                    continue;
                }
                error!("TCP: RX error {:?}", e);
                break;
            }

            let bytes = ret.unwrap();

            if bytes == 0 {
                error!("TCP: Socket closed!");
                break;
            }

            if bytes < mem::size_of::<client::DetectMessage>() {
                error!("TCP: Packet too small {}", bytes);
                thread::sleep(Duration::from_millis(1000));
                continue;
            }

            let dm: client::DetectMessage = bincode::deserialize(&buffer).unwrap();

            // Update
            if dm.kind == 5 && dm.size as usize == mem::size_of::<StateMessage>() {
                trace!("TCP: <T>SM: {:?}", &buffer[0..dm.size as usize]);
                let buffer = buffer.to_vec();
                let ret = tx.send(buffer);
                if ret.is_err() {
                    break;
                }
                continue;
            }

            // HeartbeatMessage
            if dm.kind == 6 && dm.size == 264 {
                trace!("TCP: <T>HB");
                continue;
            }

            if (dm.size as usize) == buffer.len() {
                trace!("TCP: GET: {:x?}", &buffer[0..dm.size as usize]);
            } else {
                error!("TCP: Invalid packet!");
            }
        }
    }

    pub fn disconnect(mut tc: TcpClient) {
        debug!("Shutdown tcp thread!");
        drop(tc.tx);
        if let Some(thread) = tc.id.take() {
            thread.join().unwrap();
        }
    }

    impl TcpClient {
        pub fn connect(remote_addr: String) -> Result<TcpClient, io::Error> {
            let server_list = remote_addr.to_socket_addrs();
            if let Err(_e) = server_list {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "DNS Name not found!",
                ));
            }
            let mut servers = server_list.unwrap();

            loop {
                let server = servers.next();
                if server.is_none() {
                    break;
                }
                let l = server.unwrap();
                info!("Connecting to {:?}", l);
                let ret = TcpStream::connect_timeout(&l, Duration::from_secs(1));
                match ret {
                    Err(_) => continue,
                    Ok(s) => {
                        s.set_read_timeout(Some(Duration::from_millis(100)))?;
                        let (tx, rx) = std::sync::mpsc::channel();
                        let (tx1, rx1) = std::sync::mpsc::channel();

                        info!("Connected to {:?}!", s.peer_addr());

                        let tcp_thread = thread::Builder::new().name("big2_tcp".into());
                        let id = tcp_thread.spawn(move || {
                            thread_tcp(s, tx, rx1);
                        })?;

                        // if let Err(e) = id {
                        //     println!("Can't create thread {}!", e);
                        //     return Err(e);
                        // }

                        return Ok(TcpClient {
                            rx: rx,
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

        pub fn action_pass(&mut self) -> Result<usize, io::Error> {
            let sm = Message::new(3);
            let byte_buf = bincode::serialize(&sm).unwrap();
            // println!("{:?}", byte_buf);
            let ret = self.tx.send(byte_buf);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn action_ready(&mut self) -> Result<usize, io::Error> {
            let sm = Message::new(4);
            let byte_buf = bincode::serialize(&sm).unwrap();
            let ret = self.tx.send(byte_buf);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn action_play(&mut self, cards: u64) -> Result<usize, io::Error> {
            let sm = PlayMessage {
                kind: 2,
                size: mem::size_of::<PlayMessage>() as u32,
                cards: muon::InlineList8::try_from(cards).unwrap(),
            };
            let byte_buf = bincode::serialize(&sm).unwrap();
            // println!("action_play: {:x?}", byte_buf);
            let ret = self.tx.send(byte_buf);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn send_join_msg(&mut self, name: &String) -> Result<usize, io::Error> {
            let jm = JoinMessage {
                kind: 1,
                size: mem::size_of::<JoinMessage>() as u32,
                magicnumber: common::MAGICNUMBER,
                version: common::VERSION,
                name: muon::String16::from_string(&name),
            };

            // Send Join Message.
            let jmb = bincode::serialize(&jm).unwrap();
            let ret = self.tx.send(jmb);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn check_buffer(&mut self) -> Result<Option<StateMessage>, io::Error> {
            let buffer = self.rx.try_recv();

            match buffer {
                Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(None),
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("check_buffer: Channel Disconnected {:?}", e),
                    ));
                }
                Ok(buffer) => {
                    let bytes = buffer.len();

                    if bytes < mem::size_of::<client::DetectMessage>() {
                        error!("Packet size to low {}", bytes);
                        return Ok(None);
                    }

                    let dm: client::DetectMessage = bincode::deserialize(&buffer).unwrap();

                    if dm.kind == 5 && dm.size as usize == mem::size_of::<StateMessage>() {
                        return Ok(Some(StateMessage::new(Some(&buffer))));
                    }

                    if dm.kind > 6 || dm.size as usize > bytes {
                        error!("Unknown packet drop {}", bytes);
                    }

                    return Ok(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let card = sm.your_hand.data[c] as u64;
            let suit = 1 << ((card & 0x30) >> 4);
            let mut rank = card & 0xF;
            if rank == 2 {
                rank = 15
            }
            mycards |= suit << (rank << 2);
        }

        assert_eq!(mycards, 0x10a4c18c90200000);

        let mut mycards: u64 = 0;
        for c in 0..sm.your_hand.count as usize {
            mycards |= muon::card_from_byte(sm.your_hand.data[c]);
        }
        assert_eq!(mycards, 0x10a4c18c90200000);
    }
    #[test]
    fn d_statemessage_respone() {
        let cards: u64 = 0b111 << 12;
        let sm = PlayMessage {
            kind: 2,
            size: mem::size_of::<PlayMessage>() as u32,
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
        let hand: u64 = 0;
        let muon_hand = muon::InlineList8::try_from(hand);
        assert!(muon_hand.is_ok());
        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        let muon_hand = muon_hand.unwrap();
        assert_eq!(muon_hand, il8);
        let cards = il8.into_card();
        assert!(cards.is_ok());
        assert_eq!(hand, cards.unwrap());

        // lowest card 3d
        let hand: u64 = 0x1000;
        let muon_hand = muon::InlineList8::try_from(hand);
        assert!(muon_hand.is_ok());
        let il8 = muon::InlineList8 {
            data: [0x3, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        let muon_hand = muon_hand.unwrap();
        assert_eq!(muon_hand, il8);
        let cards = il8.into_card();
        assert!(cards.is_ok());
        assert_eq!(hand, cards.unwrap());

        // higest card 2s
        let hand: u64 = 0x8000_0000_0000_0000;
        let muon_hand = muon::InlineList8::try_from(hand);
        assert!(muon_hand.is_ok());
        let il8 = muon::InlineList8 {
            data: [0x32, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        let muon_hand = muon_hand.unwrap();
        assert_eq!(muon_hand, il8);
        let cards = il8.into_card();
        assert!(cards.is_ok());
        assert_eq!(hand, cards.unwrap());

        let hand: u64 = 0xF100_0000_0000_0000;
        let il8 = muon::InlineList8 {
            data: [14, 2, 18, 34, 50, 0, 0, 0],
            count: 5,
        };
        let muon_hand = muon::InlineList8::try_from(hand).unwrap();
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon_hand.into_card().unwrap());

        let hand: u64 = 0x1F000;
        let il8 = muon::InlineList8 {
            data: [3, 19, 35, 51, 4, 0, 0, 0],
            count: 5,
        };
        let muon_hand = muon::InlineList8::try_from(hand).unwrap();
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon_hand.into_card().unwrap());

        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        assert!(il8.into_card().unwrap() == 0);
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
        assert!(il8.into_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 9,
        };
        assert!(il8.into_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: -1,
        };
        assert!(il8.into_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0xFF, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(il8.into_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0x4d, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(il8.into_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0x3f, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(il8.into_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0x3d, 0, 0, 0, 0, 0, 0, 0],
            count: 2,
        };
        assert!(il8.into_card().is_err());
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

        assert_eq!(sm.action_msg(), 0x1111800);

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
        assert_eq!(sm.action_msg(), 0x1101800);
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
        let cards = sm.action.cards.into_card().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x21211032);
        assert_eq!(trail & 0xFFFF_FFFF_FFFF_F000, cards);

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
        let cards = sm.action.cards.into_card().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x5E0073);
        assert_eq!(trail & 0xFFFF_FFFF_FFFF_F000, cards);
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
        let cards = sm.board.into_card().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x2000000000000103);
        assert_eq!(trail & 0xFFFF_FFFF_FFFF_F000, cards);
    }

    #[test]
    fn message_struct_size() {
        assert_eq!(std::mem::size_of::<Message>(), 264);
    }

    #[test]
    fn statemessage_struct_size() {
        assert_eq!(std::mem::size_of::<StateMessage>(), 224);
    }
}
