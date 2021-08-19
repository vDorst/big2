use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Player {
    P1,
    P2,
    P3,
    P4,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PlayerStatus {
    Normal,
    ToAct,
    Passed,
    Ready,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Player {
    pub fn from_idx(idx: u8) -> Option<Player> {
        match idx {
            0 => Some(Player::P1),
            1 => Some(Player::P2),
            2 => Some(Player::P3),
            3 => Some(Player::P4),
            _ => None,
        }
    }
    pub fn to_idx(&self) -> u8 {
        match self {
            Player::P1 => 0,
            Player::P2 => 1,
            Player::P3 => 2,
            Player::P4 => 3,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(tag = "type")]
pub enum Move {
    Move,
    Discard,
}
