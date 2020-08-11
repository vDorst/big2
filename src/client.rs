#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use crate::big2rules;

use serde::{Deserialize, Serialize};

use std::{
    io::{self, Write, Read},
    net::{TcpStream},
    mem,
};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum StateMessage_ActionType {
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
    name: muon_String16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct muon_String16 {
    pub data: [u8; 16],
    pub count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct muon_InlineList16 {
    pub data: [u8; 16],
    pub count: i32, 
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct muon_InlineList8 {
    pub data: [u8; 8],
    pub count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateMessage_Player {
    pub name: muon_String16,
    pub score: i32,
    pub numCards: i32,
    pub deltaScore: i32,
    pub isReady: bool,
    pub hasPassedThisCycle: bool,
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
    pub cards: muon_InlineList8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateMessage_Action {
    pub action_type: StateMessage_ActionType,
    pub player: i32,
    pub cards: muon_InlineList8,
    pub isEndOfCycle: bool,
    pub padding: [u8; 3],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateMessage {
    pub kind: u32,
    pub size: u32,
    pub round: u32,
    pub numRounds: u32,
    pub turn: i32, // -1 means in between rounds
    pub yourIndex: i32,
    pub yourHand: muon_InlineList16,
    pub players: [StateMessage_Player; 4],
    pub board: muon_InlineList8,
    pub action: StateMessage_Action,
}

pub mod client {
    use super::*;

    pub const PORT: u16 = 27191;
    pub const VERSION: u32 = 5;
    pub const MAGICNUMBER: u32 = 0x3267_6962;

    pub struct TcpClient {
        ts: TcpStream,
    }

    pub fn IL16_to_card(hand: &muon_InlineList16) -> u64 {
        let mut cards: u64 = 0;
        if hand.count > 0 && hand.count < 14 {
            for c in 0..hand.count as usize {
                let card = hand.data[c];
                cards |= card_from_byte(card);
            }
        }
        return cards;
    }

    pub fn IL8_to_card(hand: &muon_InlineList8) -> u64 {
        let mut cards: u64 = 0;
        if hand.count > 0 && hand.count < 6 { 
            for c in 0..hand.count as usize {
                let card = hand.data[c];
                cards |= card_from_byte(card);
            }
        }
        return cards;
    }

    pub fn IL8_from_card(hand: u64) -> muon_InlineList8 {
        let mut cards = muon_InlineList8 { data: [0; 8], count: 0, };
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
            Ok( TcpClient {ts: TcpStream::connect(remote_addr)?})
        }

        // pub fn StateMessage_new(self, kind: u32) -> StateMessage {
        //  let empty_buffer = &[0u8; mem::size_of::<client::StateMessage>()];
        //  let mut SM: client::StateMessage = bincode::deserialize(empty_buffer).unwrap();
        //  SM.size = mem::size_of::<client::StateMessage>() as u32;
        //  SM.kind = kind;
        //  return SM;
        // }

        // fn send_buf<T: Sized>(&mut self, value: T) -> Result<usize, io::Error> {
        //  let byte_buf = bincode::serialize(&value).unwrap();
        //  return self.ts.write(&byte_buf);    
        // }

        pub fn Action_Pass(&mut self) -> Result<usize, io::Error> {
            let empty_buffer = &[0u8; mem::size_of::<client::StateMessage>()];
            let mut SM: client::StateMessage = bincode::deserialize(empty_buffer).unwrap();
            SM.size = mem::size_of::<client::StateMessage>() as u32;
            SM.kind = 3;
            let byte_buf = bincode::serialize(&SM).unwrap();
            return self.ts.write(&byte_buf);
        }

        pub fn Action_Ready(&mut self) -> Result<usize, io::Error> {
            let empty_buffer = &[0u8; mem::size_of::<client::StateMessage>()];
            let mut SM: client::StateMessage = bincode::deserialize(empty_buffer).unwrap();
            SM.size = mem::size_of::<client::StateMessage>() as u32;
            SM.kind = 4;
            let byte_buf = bincode::serialize(&SM).unwrap();
            return self.ts.write(&byte_buf);
        }

        pub fn Action_Play(&mut self, cards: Vec::<u8>) -> Result<usize, io::Error> {
            let empty_buffer = &[0u8; mem::size_of::<client::StateMessage>()];
            let mut SM: client::StateMessage = bincode::deserialize(empty_buffer).unwrap();
            SM.size = mem::size_of::<client::StateMessage>() as u32;
            SM.kind = 4;
            let byte_buf = bincode::serialize(&SM).unwrap();
            return self.ts.write(&byte_buf);
        }

        pub fn send_join_msg(&mut self, name: &String) -> Result<usize, io::Error> {
            let mut name_bytes: [u8; 16] = [0; 16];
            let str_size = std::cmp::min(name.len(),16);
            name_bytes[..str_size].clone_from_slice(&name.as_bytes()[..str_size]);
            let JM = JoinMessage {
                kind: 1,
                size: mem::size_of::<JoinMessage>() as u32,
                magicnumber: MAGICNUMBER,
                version: VERSION,
                name: muon_String16{
                    data: name_bytes,
                    count: str_size as i32,
                },
            };

            // Send Join Message.
            let JMB = bincode::serialize(&JM).unwrap();
            return self.ts.write(&JMB);
        }

        pub fn check_buffer(&mut self, SM: &mut client::StateMessage) -> Result<usize, io::Error> {
            let mut buffer = [0; 300];
            let bytes = self.ts.read(&mut buffer)?;

            if bytes < mem::size_of::<client::DetectMessage>() {
                // println!("Packet size to low {}", bytes);
                return Ok(0);
            }

            let DM: client::DetectMessage = bincode::deserialize(&buffer).unwrap();

            // println!("Message Kind {} Size {}", DM.kind, DM.size);

            if DM.kind > 6 || DM.size as usize > buffer.len() {
                println!("Unknown packet drop {}", bytes);
                return Ok(0);
            }

            if DM.kind == 5 && DM.size as usize == mem::size_of::<client::StateMessage>() { 
                    let mut SM_NEW: client::StateMessage = bincode::deserialize(&buffer).unwrap();
                    println!("Request: {:?}", &buffer[0..DM.size as usize]);
                    *SM = SM_NEW;
                    return Ok(1);
            }

            if DM.kind == 6 {
                // println!("HeartbeatMessage");
                return Ok(0);
            } else {
                if (DM.size as usize) < buffer.len() {
                    println!("Request: {:x?}", &buffer[0..DM.size as usize]);
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
        let mut SM: StateMessage = bincode::deserialize(eb).unwrap();
        SM.size = sm_size as u32;
        SM.action = StateMessage_ActionType::play;

        let SMB = bincode::serialize(&SM).unwrap();
        println!("{:x?}", SMB);
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
        let SM: StateMessage = bincode::deserialize(&buffer).unwrap();

        let mut mycards: u64 = 0;
        for c in 0..SM.yourHand.count as usize {
            let card = SM.yourHand.data[c] as u64;
            let suit = 1 << ((card & 0x30) >> 4);
            let mut rank = card & 0xF;
            if rank == 2 { rank = 15 }
            mycards |= suit << (rank << 2);
        }

        assert_eq!(mycards, 0x10a4c18c90200000);

        let mut mycards: u64 = 0;
        for c in 0..SM.yourHand.count as usize {
            mycards |= client::card_from_byte(SM.yourHand.data[c]);
        }
        assert_eq!(mycards, 0x10a4c18c90200000);
    }
}
