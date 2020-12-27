#![allow(dead_code)]
#![allow(unused_variables)]

use crate::big2rules;
use crate::server;
//use log::{debug, error, info, trace};
use log::{trace};
use serde::{Deserialize, Serialize};
use bincode;


use std::{
    convert::TryFrom,
    mem,
};

impl server::GameServerState {
    pub fn to_statemessage(&self, player: usize) -> Vec<u8> {
        let mut sm = StateMessage::new(None);

        sm.turn = self.gs.turn;
        sm.round = self.gs.round as u32;
        sm.num_rounds = self.gs.rounds as u32;
        sm.board = InlineList8::try_from(self.gs.last_action & 0xFFFF_FFFF_FFFF_F000).unwrap();

        for p in 0..=3 {
            sm.players[p].score = self.gs.score[p] as i32;
            sm.players[p].num_cards = self.gs.card_cnt[p] as i32;
            sm.players[p].name = if self.clients[p].addr.is_none() { String16::from_str("") } else { String16::from_vec(self.clients[p].name.to_vec()) };
            let mask = 1 << p;
            if self.gs.turn == -1 {
                sm.players[p].is_ready = self.gs.has_passed & mask != 0;
                sm.players[p].has_passed_this_cycle = false;
            } else {
                sm.players[p].is_ready = true;
                sm.players[p].has_passed_this_cycle = self.gs.has_passed & mask != 0;
            }
        }

        sm.your_index = player as i32;
        let hand =InlineList16::try_from(self.gs.cards[player]);
        if hand.is_err() {
            println!("{:?}", self.gs);
        } else {
            sm.your_hand = hand.unwrap()
        }

        if self.gs.last_action & big2rules::SrvGameState::ACTION_MASK == big2rules::SrvGameState::ACTION_PLAY {
            sm.action.action_type = StateMessageActionType::PLAY;
            sm.action.player = (self.gs.last_action & 0x03) as i32;
            sm.action.cards = InlineList8::try_from(self.gs.last_action & 0xFFFF_FFFF_FFFF_F000).unwrap();
        }

        if self.gs.last_action & big2rules::SrvGameState::ACTION_MASK == big2rules::SrvGameState::ACTION_PASS {
            sm.action.action_type = StateMessageActionType::PASS;
            sm.action.player = (self.gs.last_action & 0x03) as i32;
        }

        if self.gs.last_action & big2rules::SrvGameState::ACTION_MASK == big2rules::SrvGameState::ACTION_DEAL {
            sm.action.action_type = StateMessageActionType::DEAL;
        }

        if self.gs.last_action & big2rules::SrvGameState::ACTION_MASK != 0 {
            sm.action.is_end_of_cycle = self.gs.last_action & big2rules::SrvGameState::END_OF_CYCLE == big2rules::SrvGameState::END_OF_CYCLE;
        }

        bincode::serialize(&sm).unwrap()
    }
}

impl StateMessage {
    pub fn new(init_buffer: Option<&[u8]>) -> Self {
        let buf: &[u8];
        if let Some(b) = init_buffer {
            // assert!(b.len() < std::mem::size_of::<Self>());
            buf = b;
        } else {
            buf = &[0; std::mem::size_of::<StateMessage>()];
        }
        let mut sm: StateMessage = bincode::deserialize(&buf).unwrap();
        sm.size = mem::size_of::<StateMessage>() as u32;
        sm.kind = MT_STATE;
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum StateMessageActionType {
    UPDATE = 0,
    DEAL = 1,
    PLAY = 2,
    PASS = 3,
}

#[derive(Serialize, Deserialize, Debug)]
struct JoinMessage {
    kind: u32,
    size: u32,
    magicnumber: u32,
    version: u32,
    name: String16,
}

#[derive(Serialize, Deserialize, Debug)]
struct StateMessagePlayer {
    name: String16,
    score: i32,
    num_cards: i32,
    delta_score: i32,
    is_ready: bool,
    has_passed_this_cycle: bool,
    padding: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct DetectMessage {
    kind: u32,
    size: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct PlayMessage {
    kind: u32,
    size: u32,
    cards: InlineList8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct StateMessageAction {
    pub action_type: StateMessageActionType,
    pub player: i32,
    pub cards: InlineList8,
    pub is_end_of_cycle: bool,
    pub padding: [u8; 3],
}

#[derive(Serialize, Deserialize, Debug)]
struct StateMessage {
    pub kind: u32,
    pub size: u32,
    pub round: u32,
    pub num_rounds: u32,
    pub turn: i32, // -1 means in between rounds
    pub your_index: i32,
    pub your_hand: InlineList16,
    pub players: [StateMessagePlayer; 4],
    pub board: InlineList8,
    pub action: StateMessageAction,
}

#[derive(Serialize, Deserialize, Debug)]
struct String16 {
    pub data: [u8; 16],
    pub count: i32,
}

impl String16 {
    fn to_string(&self) -> String {
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

    pub fn to_vec(&self) -> Vec<u8> {
        let cnt: usize = self.count as usize;
        self.data[..cnt].to_vec()
    }

    fn try_to_string(&self) -> Result<String,()> {
        if self.count < 0 || self.count > 16 {
            return Err(());
        }

        let cnt: usize = self.count as usize;
        let name_bytes = self.data[0..cnt].to_vec();
        let s_ret = String::from_utf8(name_bytes);
        match s_ret {
            Err(_) => Err(()),
            Ok(st) => Ok(st),
        }
    }

    fn from_str(name: &str) -> Self {
        let str_size = std::cmp::min(name.len(), 16);
        let mut name_bytes: [u8; 16] = [0; 16];
        let nb = &name.as_bytes()[..str_size];
        name_bytes[..str_size].clone_from_slice(nb);
        String16 {
            count: str_size as i32,
            data: name_bytes,
        }
    }
    fn from_vec(name: Vec<u8>) -> Self {
        let str_size = std::cmp::min(name.len(), 16);
        let mut name_bytes: [u8; 16] = [0; 16];
        name_bytes[..str_size].clone_from_slice(&name[..str_size]);
        String16 {
            count: str_size as i32,
            data: name_bytes,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct InlineList16 {
    pub data: [u8; 16],
    pub count: i32,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
struct InlineList8 {
    pub data: [u8; 8],
    pub count: i32,
}

impl InlineList16 {
    fn to_card(&self) -> u64 {
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

impl TryFrom<u64> for InlineList16 {
    type Error = &'static str;

    fn try_from(hand: u64) -> Result<Self, Self::Error> {
        let mut cards = InlineList16 {
            data: [0; 16],
            count: 0,
        };

        let num_cards = hand.count_ones();
        if num_cards > 13 || hand & 0xFFF != 0 {
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
    fn into_card(&self) -> Result<u64, &'static str> {
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

fn card_from_byte(byte: u8) -> u64 {
    let card = byte as u64;
    let suit = 1 << ((card & 0x30) >> 4);
    let mut rank = card & 0xF;
    if rank == 2 {
        rank = 15
    }
    return suit << (rank << 2);
}

fn cards_to_byte(card: u64) -> u8 {
    let mut rank = big2rules::cards::has_rank_idx(card);
    if rank == big2rules::cards::Rank::TWO {
        rank = 2;
    }
    let suit = (big2rules::cards::card_selected(card) & 0x3) << 4;
    return (rank | suit) as u8;
}

#[derive(Debug)]
pub enum StateMessageActions {
    Join(big2rules::Name),
    Play(u64),
    Pass,
    Ready,
    Heartbeat,
}
#[derive(Debug)]
pub enum StateMessageError {
    InvalidSize,
    NameToLong,
    NameToShort,
    NameInvalidUTF8,
    PacketInvalid,
    InvalidKind,
}

const PORT: u16 = 27191;
const VERSION: u32 = 6;
const MAGICNUMBER: u32 = 0x3267_6962;

const MT_JOIN: u32 = 1;
const MT_PLAY: u32 = 2;
const MT_PASS: u32 = 3;
const MT_READY: u32 = 4;
const MT_STATE: u32 = 5;
const MT_HEARTBEAT: u32 = 6;

pub fn create_join_msg(name: &str) -> Vec<u8> {
    let jm = JoinMessage {
        kind: MT_JOIN,
        size: mem::size_of::<JoinMessage>() as u32,
        magicnumber: MAGICNUMBER,
        version: VERSION,
        name: String16::from_str(name),
    };

    // Send Join Message.
    return bincode::serialize(&jm).unwrap();
}

pub fn create_heartbeat_msg() -> Vec<u8> {
    let mut hb = [0u8; 264];
    let size: u16 = hb.len() as u16;
    hb[0] = MT_HEARTBEAT as u8;
    hb[4] = (size & 0xFF) as u8;
    hb[5] = ((size >> 8) & 0xFF) as u8;
    hb.to_vec()
}


pub fn parse_packet(bytes: usize, buffer: &[u8]) -> Result<StateMessageActions, StateMessageError> {

    if bytes < mem::size_of::<DetectMessage>() {
        return Err(StateMessageError::PacketInvalid);
    }

    let dm: DetectMessage = bincode::deserialize(&buffer).unwrap();

    if dm.kind == MT_JOIN {
        if dm.size as usize > bytes || dm.size as usize != mem::size_of::<JoinMessage>() {
            println!("Invalid size");
            return Err(StateMessageError::PacketInvalid);
        }
        let jm: JoinMessage = bincode::deserialize(&buffer).unwrap();

        if jm.magicnumber != MAGICNUMBER {
            println!("Invalid magixnumber");
            return Err(StateMessageError::PacketInvalid);
        }

        if jm.version != VERSION {
            println!("Invalid version");
            return Err(StateMessageError::PacketInvalid);
        }

        let name = jm.name.try_to_string();
        if name.is_err() {
            println!("Invalid name");
            return Err(StateMessageError::PacketInvalid);
        }
        let name = big2rules::Name::from_str(name.unwrap().as_str());
        return Ok(StateMessageActions::Join(name));
    }

    if dm.kind == MT_PLAY {
        if dm.size as usize > bytes || dm.size as usize != mem::size_of::<PlayMessage>() {
            println!("Invalid size");
            return Err(StateMessageError::PacketInvalid);
        }
        let pm: PlayMessage = bincode::deserialize(&buffer).unwrap();

        let cards = pm.cards.into_card();

        match cards {
            Err(e) => {
                println!("Error {}", e);
                return Err(StateMessageError::PacketInvalid);
            },
            Ok(cards) => return Ok(StateMessageActions::Play(cards)),
        }
    }

    if dm.kind == MT_PASS {
        if dm.size as usize > bytes || dm.size as usize != mem::size_of::<StateMessage>() {
            println!("Invalid size");
            return Err(StateMessageError::PacketInvalid);
        }

        return Ok(StateMessageActions::Pass);
    }

    if dm.kind == MT_READY {
        if dm.size as usize > bytes || dm.size as usize != mem::size_of::<StateMessage>() {
            println!("Invalid size");
            return Err(StateMessageError::PacketInvalid);
        }

        return Ok(StateMessageActions::Ready);
    }

    if dm.kind == MT_HEARTBEAT && dm.size == 264 {
        return Ok(StateMessageActions::Heartbeat);
    }

    println!("Unknown packet: Kind {}, Size {}", dm.kind, dm.size);

    Err(StateMessageError::InvalidKind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_join_valid() {
        let jmb: &[u8] = &[1, 0, 0, 0, 36, 0, 0, 0, 98, 105, 103, 50, 6, 0, 0, 0, 82, 101, 110, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let ret = parse_packet(jmb.len(), &jmb);
        println!("{:?}", ret);
        match ret {
            Ok(StateMessageActions::Join(name)) => assert_eq!(name.to_vec(), big2rules::Name::from_str("Rene").to_vec()),
            _ => assert!(false),
        }
    }

    #[test]
    fn parse_join_invalid_magicnumber() {
        let jmb: &[u8] = &[1, 0, 0, 0, 36, 0, 0, 0, 99, 105, 103, 50, 6, 0, 0, 0, 82, 101, 110, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let ret = parse_packet(jmb.len(), &jmb);
        println!("{:?}", ret);
        match ret {
            Err(StateMessageError::PacketInvalid) => assert!(true),
            _ => assert!(false),
        }
    }
    #[test]
    fn create_and_parse_heartbeat_msg() {
        let hb_vector = vec![6u8, 0, 0, 0, 8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let hb_msg = create_heartbeat_msg();
        assert_eq!(hb_vector, hb_msg);
        let ret = parse_packet(hb_vector.len(), &hb_vector);
        match ret {
            Ok(StateMessageActions::Heartbeat) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_state_message_action_type() {
        let at = StateMessageActionType::UPDATE;
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [0,0,0,0]);

        let at = StateMessageActionType::DEAL;
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [1,0,0,0]);

        let at = StateMessageActionType::PLAY;
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [2,0,0,0]);

        let at = StateMessageActionType::PASS;
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [3,0,0,0]);
    }


    #[test]
    fn test_state_message_action() {
        let at = StateMessageAction {
            action_type: StateMessageActionType::UPDATE,
            cards: InlineList8::try_from(0).unwrap(),
            player: 0,
            is_end_of_cycle: false,
            padding: [0; 3],
        };
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let at = StateMessageAction {
            action_type: StateMessageActionType::DEAL,
            player: 0,
            cards: InlineList8::try_from(0).unwrap(),
            is_end_of_cycle: false,
            padding: [0; 3],
        };
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let at = StateMessageAction {
            action_type: StateMessageActionType::PLAY,
            player: 3,
            cards: InlineList8::try_from(0x1000).unwrap(),
            is_end_of_cycle: true,
            padding: [0; 3],
        };
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [2, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]);

        let at2: StateMessageAction = bincode::deserialize(&b).unwrap();
        assert_eq!(at, at2);

        let at = StateMessageAction {
            action_type: StateMessageActionType::PLAY,
            player: 2,
            cards: InlineList8::try_from(0xF000_1000).unwrap(),
            is_end_of_cycle: false,
            padding: [0; 3],
        };
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [2, 0, 0, 0, 2, 0, 0, 0, 3, 7, 23, 39, 55, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0]);

        let at = StateMessageAction {
            action_type: StateMessageActionType::PASS,
            player: 3,
            cards: InlineList8::try_from(0).unwrap(),
            is_end_of_cycle: false,
            padding: [0; 3],
        };
        let b = bincode::serialize(&at).unwrap();
        assert_eq!(b, [3, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_string_to_muon_string16_valid() {
        let name_vec = vec![65u8, 66, 67, 68];
        let name_str16 = String16::from_vec(name_vec);
        assert_eq!(name_str16.data, [65u8, 66, 67, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(name_str16.count, 4);

        let name_str = "test";
        let name_str16 = String16::from_str(name_str);
        assert_eq!(name_str16.data, [116, 101, 115, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(name_str16.count, 4);

        assert_eq!(name_str16.to_string(), String::from("test"));
    }
}