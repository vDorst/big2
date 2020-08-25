use crate::big2rules;
use serde::{Deserialize, Serialize};

use std::{
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

pub mod muon {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct String16 {
        pub data: [u8; 16],
        pub count: i32,
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

    pub fn inline16_to_card(hand: &InlineList16) -> u64 {
        let mut cards: u64 = 0;
        if hand.count > 0 && hand.count < 14 {
            for c in 0..hand.count as usize {
                let card = hand.data[c];
                cards |= card_from_byte(card);
            }
        }
        return cards;
    }
    pub fn inline8_to_card(hand: &InlineList8) -> u64 {
        let mut cards: u64 = 0;
        if hand.count > 0 && hand.count < 6 {
            for c in 0..hand.count as usize {
                let card = hand.data[c];
                cards |= card_from_byte(card);
            }
        }
        return cards;
    }

    // 02 00 00 00 14 00 00 00 06 26 36 00 00 00 00 00 03 00 00 00

    pub fn inline8_from_card(hand: u64) -> InlineList8 {
        let mut cards = InlineList8 {
            data: [0; 8],
            count: 0,
        };
        let num_cards = hand.count_ones();
        if num_cards > 5 || num_cards == 4 {
            return cards;
        };

        cards.count = num_cards as i32;

        let mut p: usize = 0;
        for bit in 12..64 {
            let mask = 1 << bit;
            let card = hand & mask;
            if card != 0 {
                cards.data[p] = cards_to_byte(card);
                p += 1;
            }
        }
        return cards;
    }

    pub fn inline8_from_card_fast(hand: u64) -> InlineList8 {
        let mut cards = InlineList8 {
            data: [0; 8],
            count: 0,
        };
        let num_cards = hand.count_ones();
        if num_cards > 5 || num_cards == 4 {
            return cards;
        };

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
        return cards;
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

pub mod client {
    use super::*;

    pub const PORT: u16 = 27191;
    pub const VERSION: u32 = 5;
    pub const MAGICNUMBER: u32 = 0x3267_6962;
    pub const BUFSIZE: usize = 512;

    pub struct TcpClient {
        id: Option<thread::JoinHandle<()>>,
        rx: Receiver<Vec<u8>>,
        tx: Sender<Vec<u8>>,
    }

    fn thread_tcp(mut ts: TcpStream, tx: Sender<Vec<u8>>, rx: Receiver<Vec<u8>>) {
        let mut buffer = [0; BUFSIZE];

        loop {
            let tx_data = rx.try_recv();
            if let Err(e) = tx_data {
                if e == std::sync::mpsc::TryRecvError::Disconnected {
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

            if bytes < mem::size_of::<client::DetectMessage>() {
                error!("TCP: Packet too small {}", bytes);
                thread::sleep(Duration::from_millis(1000));
                continue;
            }

            let dm: client::DetectMessage = bincode::deserialize(&buffer).unwrap();

            // print!("TCP: Message Kind {} Size {}\r", dm.kind, dm.size);

            // Update
            if dm.kind == 5 && dm.size as usize == mem::size_of::<StateMessage>() {
                trace!("TCP: <T>SM: {:x?}", &buffer[0..dm.size as usize]);
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
            let empty_buffer = &[0u8; mem::size_of::<StateMessage>()];
            let mut sm: StateMessage = bincode::deserialize(empty_buffer).unwrap();
            sm.size = mem::size_of::<StateMessage>() as u32;
            sm.kind = 3;
            let byte_buf = bincode::serialize(&sm).unwrap();
            // println!("{:?}", byte_buf);
            let ret = self.tx.send(byte_buf);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn action_ready(&mut self) -> Result<usize, io::Error> {
            let empty_buffer = &[0u8; mem::size_of::<StateMessage>()];
            let mut sm: StateMessage = bincode::deserialize(empty_buffer).unwrap();
            sm.size = mem::size_of::<StateMessage>() as u32;
            sm.kind = 4;
            let byte_buf = bincode::serialize(&sm).unwrap();
            let ret = self.tx.send(byte_buf);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn action_play(&mut self, cards: &muon::InlineList8) -> Result<usize, io::Error> {
            let sm = PlayMessage {
                kind: 2,
                size: mem::size_of::<PlayMessage>() as u32,
                cards: cards.clone(),
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
            let mut name_bytes: [u8; 16] = [0; 16];
            let str_size = std::cmp::min(name.len(), 16);
            name_bytes[..str_size].clone_from_slice(&name.as_bytes()[..str_size]);
            let jm = JoinMessage {
                kind: 1,
                size: mem::size_of::<JoinMessage>() as u32,
                magicnumber: MAGICNUMBER,
                version: VERSION,
                name: muon::String16 {
                    data: name_bytes,
                    count: str_size as i32,
                },
            };

            // Send Join Message.
            let jmb = bincode::serialize(&jm).unwrap();
            let ret = self.tx.send(jmb);
            if ret.is_err() {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Thread died!"));
            }
            Ok(0)
        }

        pub fn check_buffer(&mut self, sm: &mut StateMessage) -> Result<usize, io::Error> {
            let buffer = self.rx.try_recv();

            if let Err(e) = buffer {
                if e == std::sync::mpsc::TryRecvError::Empty {
                    return Ok(0);
                }
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("check_buffer: Channel Disconnected {:?}", e),
                ));
            }

            let buffer = buffer.unwrap();

            let bytes = buffer.len();

            if bytes < mem::size_of::<client::DetectMessage>() {
                error!("Packet size to low {}", bytes);
                return Ok(0);
            }

            let dm: client::DetectMessage = bincode::deserialize(&buffer).unwrap();

            if dm.kind > 6 || dm.size as usize > buffer.len() {
                error!("Unknown packet drop {}", bytes);
                return Ok(0);
            }

            if dm.kind == 5 && dm.size as usize == mem::size_of::<StateMessage>() {
                let sm_new: StateMessage = bincode::deserialize(&buffer).unwrap();
                *sm = sm_new;
                return Ok(1);
            }

            return Ok(0);
        }
    }
}

pub mod server {
    use super::*;

    pub const PORT: u16 = 27191;
    pub const VERSION: u32 = 5;
    pub const MAGICNUMBER: u32 = 0x3267_6962;
    pub const BUFSIZE: usize = 512;

    pub struct TcpServer {
        id: Option<thread::JoinHandle<()>>,
        rx: Receiver<Vec<u8>>,
        tx: Sender<Vec<u8>>,
    }

    fn thread_tcp(mut ts: TcpStream, tx: Sender<Vec<u8>>, rx: Receiver<Vec<u8>>) {
        let mut buffer = [0; BUFSIZE];

        loop {
            let tx_data = rx.try_recv();
            if let Err(e) = tx_data {
                if e == std::sync::mpsc::TryRecvError::Disconnected {
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

            if bytes < mem::size_of::<DetectMessage>() {
                error!("TCP: Packet too small {}", bytes);
                thread::sleep(Duration::from_millis(1000));
                continue;
            }

            let dm: DetectMessage = bincode::deserialize(&buffer).unwrap();

            // print!("TCP: Message Kind {} Size {}\r", dm.kind, dm.size);

            // Update
            if dm.kind == 5 && dm.size as usize == mem::size_of::<StateMessage>() {
                trace!("TCP: <T>SM: {:x?}", &buffer[0..dm.size as usize]);
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

    pub fn disconnect(mut tc: TcpServer) {
        debug!("Shutdown tcp thread!");
        drop(tc.tx);
        if let Some(thread) = tc.id.take() {
            thread.join().unwrap();
        }
    }

    impl TcpServer {
        pub fn connect(remote_addr: String) -> Result<TcpServer, io::Error> {
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

                        return Ok(TcpServer {
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

        pub fn check_buffer(&mut self, sm: &mut StateMessage) -> Result<usize, io::Error> {
            let buffer = self.rx.try_recv();

            if let Err(e) = buffer {
                if e == std::sync::mpsc::TryRecvError::Empty {
                    return Ok(0);
                }
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("check_buffer: Channel Disconnected {:?}", e),
                ));
            }

            let buffer = buffer.unwrap();

            let bytes = buffer.len();

            if bytes < mem::size_of::<DetectMessage>() {
                error!("Packet size to low {}", bytes);
                return Ok(0);
            }

            let dm: DetectMessage = bincode::deserialize(&buffer).unwrap();

            if dm.kind > 6 || dm.size as usize > buffer.len() {
                error!("Unknown packet drop {}", bytes);
                return Ok(0);
            }

            if dm.kind == 5 && dm.size as usize == mem::size_of::<StateMessage>() {
                let sm_new: StateMessage = bincode::deserialize(&buffer).unwrap();
                *sm = sm_new;
                return Ok(1);
            }

            return Ok(0);
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
        let &buffer: &[u8; 224] = &[
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
        assert_eq!(buffer.len(), std::mem::size_of::<StateMessage>());
        let sm: StateMessage = bincode::deserialize(&buffer).unwrap();

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
            cards: muon::inline8_from_card(cards).clone(),
        };
        let byte_buf = bincode::serialize(&sm).unwrap();
        println!("action_play: {:x?}", byte_buf);
    }

    #[test]
    fn a_rules_sizes() {
        let hand: u64 = 0;
        let muon_hand = muon::inline8_from_card(hand);
        let muon_hand_fast = muon::inline8_from_card_fast(hand);
        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        assert_eq!(muon_hand, muon_hand_fast);
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon::inline8_to_card(&muon_hand));

        let hand: u64 = 0x1000;
        let il8 = muon::InlineList8 {
            data: [0x3, 0, 0, 0, 0, 0, 0, 0],
            count: 1,
        };
        let muon_hand = muon::inline8_from_card(hand);
        let muon_hand_fast = muon::inline8_from_card_fast(hand);
        assert_eq!(muon_hand, muon_hand_fast);
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon::inline8_to_card(&muon_hand));

        let hand: u64 = 0xF000;
        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        let muon_hand = muon::inline8_from_card(hand);
        let muon_hand_fast = muon::inline8_from_card_fast(hand);
        assert_eq!(muon_hand, muon_hand_fast);
        assert_eq!(muon_hand, il8);
        assert!(muon::inline8_to_card(&muon_hand) == 0);

        let hand: u64 = 0x1F000;
        let il8 = muon::InlineList8 {
            data: [3, 19, 35, 51, 4, 0, 0, 0],
            count: 5,
        };
        let muon_hand = muon::inline8_from_card(hand);
        let muon_hand_fast = muon::inline8_from_card_fast(hand);
        assert_eq!(muon_hand, muon_hand_fast);
        assert_eq!(muon_hand, il8);
        assert_eq!(hand, muon::inline8_to_card(&muon_hand));

        let hand: u64 = 0xFF000;
        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        let muon_hand = muon::inline8_from_card(hand);
        let muon_hand_fast = muon::inline8_from_card_fast(hand);
        assert_eq!(muon_hand, muon_hand_fast);
        assert_eq!(muon_hand, il8);
        assert!(muon::inline8_to_card(&muon_hand) == 0);

        let hand: u64 = 0xFFF000;
        let il8 = muon::InlineList8 {
            data: [0; 8],
            count: 0,
        };
        let muon_hand = muon::inline8_from_card(hand);
        let muon_hand_fast = muon::inline8_from_card_fast(hand);
        assert_eq!(muon_hand, muon_hand_fast);
        assert_eq!(muon_hand, il8);
        assert!(muon::inline8_to_card(&muon_hand) == 0);
    }
}
