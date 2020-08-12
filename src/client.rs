use crate::big2rules;

use serde::{Deserialize, Serialize};

use std::{
    io::{self, Write, Read},
    net::{TcpStream, ToSocketAddrs},
    mem,
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
    name: MuonString16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MuonString16 {
    pub data: [u8; 16],
    pub count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MuonInlineList16 {
    pub data: [u8; 16],
    pub count: i32,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct MuonInlineList8 {
    pub data: [u8; 8],
    pub count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateMessagePlayer {
    pub name: MuonString16,
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
    pub cards: MuonInlineList8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateMessageAction {
    pub action_type: StateMessageActionType,
    pub player: i32,
    pub cards: MuonInlineList8,
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
    pub your_hand: MuonInlineList16,
    pub players: [StateMessagePlayer; 4],
    pub board: MuonInlineList8,
    pub action: StateMessageAction,
}

pub mod client {
    use super::*;

    pub const PORT: u16 = 27191;
    pub const VERSION: u32 = 5;
    pub const MAGICNUMBER: u32 = 0x3267_6962;

    pub struct TcpClient {
        ts: TcpStream,
    }

    pub fn muon_inline16_to_card(hand: &MuonInlineList16) -> u64 {
        let mut cards: u64 = 0;
        if hand.count > 0 && hand.count < 14 {
            for c in 0..hand.count as usize {
                let card = hand.data[c];
                cards |= card_from_byte(card);
            }
        }
        return cards;
    }

    pub fn muon_inline8_to_card(hand: &MuonInlineList8) -> u64 {
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



    pub fn muon_inline8_from_card(hand: u64) -> MuonInlineList8 {
        let mut cards = MuonInlineList8 { data: [0; 8], count: hand.count_ones() as i32, };
        let num_cards = hand.count_ones();
        if num_cards > 5 { return cards };

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

    pub fn card_from_byte(byte: u8) -> u64 {
        let card = byte as u64;
        let suit = 1 << ((card & 0x30) >> 4);
        let mut rank = card & 0xF;
        if rank == 2 { rank = 15 }
        return suit << (rank << 2);
    }

    pub fn cards_to_byte(card: u64) -> u8 {
        let mut rank = big2rules::cards::has_rank_idx(card);
        if rank == big2rules::cards::Rank::TWO { rank = 2; }
        let suit = (big2rules::cards::card_selected(card) & 0x3) << 4;
        return (rank | suit) as u8;
    }

    impl TcpClient {
        pub fn connect(remote_addr: String) -> Result<TcpClient, io::Error> {
            let server_list = remote_addr.to_socket_addrs();
            if let Err(_e) = server_list { return Err(io::Error::new(io::ErrorKind::NotFound, "DNS Name not found!")); }
            let mut servers = server_list.unwrap();

            loop {
                let server = servers.next();
                if server.is_none() { break; }
                let l = server.unwrap();
                print!("Trying {:?}\r\n", l);
                let ret = TcpStream::connect_timeout(&l, Duration::from_secs(1));
                match ret {
                    Err(_) => continue,
                    Ok(s) => {
                        s.set_read_timeout(Some(Duration::from_millis(100)))?;
                        return Ok( TcpClient { ts: s} );
                    },
                }
            }
            Err(io::Error::new(io::ErrorKind::TimedOut, "Can't Connect Timeout!"))
        }

        pub fn action_pass(&mut self) -> Result<usize, io::Error> {
            let empty_buffer = &[0u8; mem::size_of::<client::StateMessage>()];
            let mut sm: client::StateMessage = bincode::deserialize(empty_buffer).unwrap();
            sm.size = mem::size_of::<client::StateMessage>() as u32;
            sm.kind = 3;
            let byte_buf = bincode::serialize(&sm).unwrap();
            println!("{:?}", byte_buf);
            return self.ts.write(&byte_buf);
        }

        pub fn action_ready(&mut self) -> Result<usize, io::Error> {
            let empty_buffer = &[0u8; mem::size_of::<client::StateMessage>()];
            let mut sm: client::StateMessage = bincode::deserialize(empty_buffer).unwrap();
            sm.size = mem::size_of::<client::StateMessage>() as u32;
            sm.kind = 4;
            let byte_buf = bincode::serialize(&sm).unwrap();
            return self.ts.write(&byte_buf);
        }

        pub fn action_play(&mut self, cards: &MuonInlineList8) -> Result<usize, io::Error> {
            let sm = client::PlayMessage {
                kind: 2,
                size: mem::size_of::<client::PlayMessage>() as u32,
                cards: cards.clone(),
            };
            let byte_buf = bincode::serialize(&sm).unwrap();
            println!("action_play: {:x?}", byte_buf);
            return self.ts.write(&byte_buf);
        }

        pub fn send_join_msg(&mut self, name: &String) -> Result<usize, io::Error> {
            let mut name_bytes: [u8; 16] = [0; 16];
            let str_size = std::cmp::min(name.len(),16);
            name_bytes[..str_size].clone_from_slice(&name.as_bytes()[..str_size]);
            let jm = JoinMessage {
                kind: 1,
                size: mem::size_of::<JoinMessage>() as u32,
                magicnumber: MAGICNUMBER,
                version: VERSION,
                name: MuonString16{
                    data: name_bytes,
                    count: str_size as i32,
                },
            };

            // Send Join Message.
            let jmb = bincode::serialize(&jm).unwrap();
            return self.ts.write(&jmb);
        }

        pub fn check_buffer(&mut self, sm: &mut client::StateMessage) -> Result<usize, io::Error> {
            let mut buffer = [0; 300];
            let bytes = self.ts.peek(&mut buffer);

            match bytes {
                Err(e) => if e.kind() == io::ErrorKind::TimedOut { return Ok(0) },
                Ok(b)  => if b < mem::size_of::<client::DetectMessage>() { return Ok(0); },
            }

            let bytes = self.ts.read(&mut buffer)?;

            if bytes < mem::size_of::<client::DetectMessage>() {
                // println!("Packet size to low {}", bytes);
                return Ok(0);
            }

            let dm: client::DetectMessage = bincode::deserialize(&buffer).unwrap();

            // println!("Message Kind {} Size {}", dm.kind, dm.size);

            if dm.kind > 6 || dm.size as usize > buffer.len() {
                println!("Unknown packet drop {}", bytes);
                return Ok(0);
            }

            if dm.kind == 5 && dm.size as usize == mem::size_of::<client::StateMessage>() {
                    let sm_new: client::StateMessage = bincode::deserialize(&buffer).unwrap();
                    *sm = sm_new;
                    return Ok(1);
            }

            if dm.kind == 6 {
                // println!("HeartbeatMessage");
                return Ok(0);
            } else {
                if (dm.size as usize) < buffer.len() {
                    println!("Request: {:x?}", &buffer[0..dm.size as usize]);
                } else {
                    println!("Invalid packet!");
                }
            }

            return Ok(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::from_utf8;
    use std::mem::MaybeUninit;

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
        let &buffer: &[u8; 224] = &[5, 0, 0, 0, 0xe0, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0,
            0x15, 7, 0x37, 0x28, 0x38, 0x39, 0xa, 0x2b, 0x3b, 0x2c, 0x1d, 0x3d, 2, 0, 0,
            0, 0xd, 0, 0, 0, 0x54, 0x69, 0x6b, 0x6b, 0x69, 0x65, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0x68, 0x6f, 0x73, 0x74,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            0x52, 0x65, 0x6e, 0x65, 0x31, 0x32, 0x33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0xb,
            0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0x52, 0x65, 0x6e, 0x65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0,
            0, 0, 0, 0, 0, 0, 0xd, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0x16, 0x26, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(buffer.len(), std::mem::size_of::<StateMessage>());
        let sm: StateMessage = bincode::deserialize(&buffer).unwrap();

        let mut mycards: u64 = 0;
        for c in 0..sm.your_hand.count as usize {
            let card = sm.your_hand.data[c] as u64;
            let suit = 1 << ((card & 0x30) >> 4);
            let mut rank = card & 0xF;
            if rank == 2 { rank = 15 }
            mycards |= suit << (rank << 2);
        }

        assert_eq!(mycards, 0x10a4c18c90200000);

        let mut mycards: u64 = 0;
        for c in 0..sm.your_hand.count as usize {
            mycards |= client::card_from_byte(sm.your_hand.data[c]);
        }
        assert_eq!(mycards, 0x10a4c18c90200000);
    }
    #[test]
    fn d_statemessage_respone() {
        let cards: u64 = 0b111 << 12;
        let sm = PlayMessage {
            kind: 2,
            size: mem::size_of::<PlayMessage>() as u32,
            cards: client::muon_inline8_from_card(cards).clone(),
        };
        let byte_buf = bincode::serialize(&sm).unwrap();
        println!("action_play: {:x?}", byte_buf);
    }
}
