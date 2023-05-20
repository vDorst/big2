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

#[non_exhaustive]
struct PlayerID(pub u8);

impl PlayerID {
    pub fn try_from<T>(val: T) -> Option<Self>
    where
        u8: TryFrom<T>,
    {
        u8::try_from(val).ok().and_then(|v| {
            if (0..=3).contains(&v) {
                Some(Self(v))
            } else {
                None
            }
        })
    }

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
        sm.size = mem::size_of::<StateMessage>() as u32;
        sm
    }
    #[must_use]
    pub fn current_player(&self) -> Option<usize> {
        PlayerID::try_from(self.turn).map(|p| p.in_to())
    }
    #[must_use]
    pub fn current_player_name(&self) -> Option<&str> {
        self.current_player().map(|p| self.players[p].name.as_str())
    }
    #[must_use]
    pub fn player_name(&self, p: i32) -> Option<&str> {
        PlayerID::try_from(p).map(|p| self.players[p.in_to::<usize>()].name.as_str())
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
        p |= ((self.turn as u64) & 0x7) << 4;

        match self.action.action_type {
            StateMessageActionType::Play => {
                let mut cards = self.action.cards.as_card().expect("invalid cards");
                cards |= p;
                cards
            }
            StateMessageActionType::Pass => {
                let mut cards = self.board.as_card().expect("invalid cards");
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
                let mut cards = self.your_hand.to_card();
                cards |= 0x400;
                cards |= PlayerID::try_from(self.your_index)
                    .expect("Shoud fit")
                    .in_to::<u64>()
                    & 0x7;
                cards |= ((self.turn as u64) & 0x7) << 4;
                cards
            }
        }
    }
}

pub mod muon {
    use super::{big2rules, Deserialize, Serialize, TryFrom};

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub struct String16 {
        data: [u8; 16],
        count: u32,
    }

    impl String16 {
        #[must_use]
        pub fn as_str(&self) -> &str {
            let Some(cnt) = usize::try_from(self.count).ok().and_then(|v| if (0..=16).contains(&v) { Some(v) } else { None } ) else {
                return "Invalid string";
            };

            match core::str::from_utf8(&self.data[..cnt]) {
                Err(_) => "Can't convert",
                Ok(st) => st,
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

        pub fn is_empty(&self) -> bool {
            self.count == 0
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
        #[must_use]
        pub fn to_card(&self) -> u64 {
            let mut cards: u64 = 0;
            if self.count > 0 && self.count < 14 {
                if let Ok(count) = usize::try_from(self.count) {
                    for c in 0..count {
                        let card = self.data[c];
                        cards |= card_from_byte(card);
                    }
                }
            }
            cards
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

            cards.count = i32::try_from(num_cards).expect("Shoud fit");

            let mut hand = hand;
            let mut p: usize = 0;
            while hand != 0 {
                let zeros = u64::from(hand.trailing_zeros());

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
        pub fn as_card(&self) -> Result<u64, &'static str> {
            let Some(count) = usize::try_from(self.count).ok().and_then(|v| if (0..=self.data.len()).contains(&v) { Some(v)} else { None }) else {
                return Err("Count out-of-range!");
            };
            let mut cards: u64 = 0;
            for &card in &self.data[0..count] {
                let c = card & 0b1100_1111;
                if !(2..=14).contains(&c) {
                    return Err("Card value out-of-range!");
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
    pub fn cards_to_byte(card: u64) -> u8 {
        let mut rank = big2rules::cards::has_rank_idx(card);
        if rank == big2rules::cards::Rank::TWO {
            rank = 2;
        }
        let suit = (big2rules::cards::card_selected(card) & 0x3) << 4;
        u8::try_from(rank | suit).expect("Should fit u8!")
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
    use super::{
        client, common, debug, error, info, io, mem, muon, thread, trace, DetectMessage, Duration,
        JoinMessage, Message, PlayMessage, Read, Receiver, Sender, StateMessage, TcpStream,
        ToSocketAddrs, TryFrom, Write,
    };

    const DM_SIZE: usize = mem::size_of::<client::DetectMessage>();
    const M_SIZE: usize = mem::size_of::<Message>();
    const SM_SIZE: usize = mem::size_of::<StateMessage>();

    pub struct TcpClient {
        id: Option<thread::JoinHandle<()>>,
        rx: Receiver<Vec<u8>>,
        tx: Sender<Vec<u8>>,
    }

    fn thread_tcp(mut ts: TcpStream, tx: &Sender<Vec<u8>>, rx: &Receiver<Vec<u8>>) {
        let mut buffer = [0; common::BUFSIZE];

        'tcp_loop: loop {
            let tx_data = rx.try_recv();
            match tx_data {
                Err(std::sync::mpsc::TryRecvError::Empty) => (),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    error!("TCP: TX channel disconnected");
                    break;
                }
                Ok(data) => {
                    trace!("TCP: PUSH: {:x?}", data);
                    let ret = ts.write(&data);
                    if let Err(e) = ret {
                        error!("TCP: Error write. {}", e);
                    }
                }
            }

            let mut n_bytes = match ts.read(&mut buffer) {
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
                    if e.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    error!("TCP: RX error {:?}", e);
                    break;
                }
            };

            info!("TCP: Got Bytes {}", n_bytes);

            let mut pos: usize = 0;

            if n_bytes > 264 {
                trace!("SM {:?}", &buffer[0..n_bytes]);
            }

            while n_bytes >= DM_SIZE {
                let dm: client::DetectMessage = bincode::deserialize(&buffer[pos..]).unwrap();

                // Update

                let msg_size = usize::try_from(dm.size).unwrap();

                if dm.kind == 5 && msg_size == SM_SIZE {
                    if n_bytes < SM_SIZE {
                        continue 'tcp_loop;
                    }

                    let buf = buffer[pos..pos + SM_SIZE].to_vec();

                    info!("TCP: P{} B{} SM: {:?}", pos, n_bytes, buf);
                    let ret = tx.send(buf);
                    if ret.is_err() {
                        error!("TCP: MPSC TX ERROR {:?}", ret.unwrap_err());
                        break;
                    }
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
                continue 'tcp_loop;
            }
        }
    }

    pub fn disconnect(mut tc: TcpClient) {
        debug!("Shutdown tcp thread!");
        drop(tc.tx);
        if let Some(thread) = tc.id.take() {
            if let Err(e) = thread.join() {
                eprintln!("Thread shutdown issue: {e:?}");
            };
        }
    }

    impl TcpClient {
        pub fn connect(remote_addr: &str) -> Result<TcpClient, io::Error> {
            let server_list = remote_addr.to_socket_addrs();
            if let Err(_e) = server_list {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "DNS Name not found!",
                ));
            }
            let mut servers = server_list.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            loop {
                let server = servers.next();
                let Some(l) = server else { break };
                info!("Connecting to {:?}", l);
                let ret = TcpStream::connect_timeout(&l, Duration::from_secs(1));
                match ret {
                    Err(_) => continue,
                    Ok(s) => {
                        s.set_read_timeout(Some(Duration::from_millis(10)))?;
                        let (tx, rx) = std::sync::mpsc::channel();
                        let (tx1, rx1) = std::sync::mpsc::channel();

                        info!("Connected to {:?}!", s.peer_addr());

                        let tcp_thread = thread::Builder::new().name("big2_tcp".into());
                        let id = tcp_thread.spawn(move || {
                            thread_tcp(s, &tx, &rx1);
                        })?;

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

        pub fn action_pass(&mut self) -> Result<usize, io::Error> {
            let sm = Message::new(3);
            let byte_buf =
                bincode::serialize(&sm).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            // println!("{:?}", byte_buf);
            let ret = self.tx.send(byte_buf);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn action_ready(&mut self) -> Result<usize, io::Error> {
            let sm = Message::new(4);
            let byte_buf =
                bincode::serialize(&sm).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let ret = self.tx.send(byte_buf);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn action_play(&mut self, cards: u64) -> Result<usize, io::Error> {
            let sm = PlayMessage {
                kind: 2,
                size: u32::try_from(mem::size_of::<PlayMessage>()).expect("Should fit u32!"),
                cards: muon::InlineList8::try_from(cards)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
            };
            let byte_buf =
                bincode::serialize(&sm).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            // println!("action_play: {:x?}", byte_buf);
            let ret = self.tx.send(byte_buf);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn send_join_msg(&mut self, name: &str) -> Result<usize, io::Error> {
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
            let ret = self.tx.send(jmb);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn check_buffer(&mut self) -> Result<Option<StateMessage>, io::Error> {
            let buffer = self.rx.try_recv();

            match buffer {
                Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
                Err(e) => Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("check_buffer: Channel Disconnected {e:?}"),
                )),
                Ok(buffer) => {
                    let bytes = buffer.len();

                    if bytes < mem::size_of::<client::DetectMessage>() {
                        error!("Packet size to low {}", bytes);
                        return Ok(None);
                    }

                    let dm: client::DetectMessage = bincode::deserialize(&buffer)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                    let msg_size = usize::try_from(dm.size).unwrap();

                    if dm.kind == 5 && msg_size == mem::size_of::<StateMessage>() {
                        return Ok(Some(StateMessage::new(Some(&buffer))));
                    }

                    if dm.kind > 6 || msg_size > bytes {
                        error!("Unknown packet drop {}", bytes);
                    }

                    Ok(None)
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
            let card = u64::from(sm.your_hand.data[c]);
            let suit = 1 << ((card & 0x30) >> 4);
            let mut rank = card & 0xF;
            if rank == 2 {
                rank = 15
            }
            mycards |= suit << (rank << 2);
        }

        assert_eq!(mycards, 0x10a4_c18c_9020_0000);

        let mut mycards: u64 = 0;
        for c in 0..sm.your_hand.count as usize {
            mycards |= muon::card_from_byte(sm.your_hand.data[c]);
        }
        assert_eq!(mycards, 0x10a4_c18c_9020_0000);
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
        let cards = il8.as_card();
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
        let cards = il8.as_card();
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
        let cards = il8.as_card();
        assert!(cards.is_ok());
        assert_eq!(hand, cards.unwrap());

        let hand: u64 = 0xF100_0000_0000_0000;
        let il8 = muon::InlineList8 {
            data: [14, 2, 18, 34, 50, 0, 0, 0],
            count: 5,
        };
        let muon_hand = muon::InlineList8::try_from(hand).unwrap();
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon_hand.as_card().unwrap());

        let hand: u64 = 0x1F000;
        let il8 = muon::InlineList8 {
            data: [3, 19, 35, 51, 4, 0, 0, 0],
            count: 5,
        };
        let muon_hand = muon::InlineList8::try_from(hand).unwrap();
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon_hand.as_card().unwrap());

        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        assert!(il8.as_card().unwrap() == 0);
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
        assert!(il8.as_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 9,
        };
        assert!(il8.as_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: -1,
        };
        assert!(il8.as_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0xFF, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(il8.as_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0x4d, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(il8.as_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0x3f, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        assert!(il8.as_card().is_err());

        let il8 = muon::InlineList8 {
            data: [0x3d, 0, 0, 0, 0, 0, 0, 0],
            count: 2,
        };
        assert!(il8.as_card().is_err());
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
        let cards = sm.action.cards.as_card().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x2121_1032);
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
        let cards = sm.action.cards.as_card().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x005E_0073);
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
        let cards = sm.board.as_card().unwrap();
        let trail = sm.action_msg();
        assert_eq!(trail, 0x2000_0000_0000_0103);
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

        const DM_SIZE: usize = mem::size_of::<self::DetectMessage>();
        while n_bytes >= DM_SIZE {
            println!("Bytes {n_bytes} Pos {pos}");

            let dm: self::DetectMessage =
                bincode::deserialize(&buffer[pos..pos + DM_SIZE]).unwrap();

            let msg_size = usize::try_from(dm.size).unwrap();

            // Update
            const SM_SIZE: usize = mem::size_of::<StateMessage>();
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
            const M_SIZE: usize = mem::size_of::<Message>();
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
}
