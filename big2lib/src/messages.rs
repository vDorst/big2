use std::ops::IndexMut;

use serde::{Deserialize, Serialize};

use crate::players::{Move, Player, PlayerStatus};

type PlayerID = String;
type RoomID = String;
type Cards = u64;
type Score = i8;
type EndScore = i16;

pub const MAX_PLAYERS: usize = 4;
pub const MAX_SPECTATORS: usize = 12;

pub struct Client {
    room: RoomID,
}

pub struct RoomInfo {
    // Inforation About the room
    pub room: RoomID,
    // List with players
    pub player: [Option<PlayerID>; MAX_PLAYERS],
    pub spectators: Vec<PlayerID>,
    pub rounds: u8,
    pub state: RoomState,
    pub update: GameUpdate,
}

pub enum RoomState {
    WaitingForPlayers,
    Full,
}

pub struct LastAction {
    player: PlayerID,
    action: Action,
    state: GameState,
}

pub enum Action {
    Deal(Option<Cards>),
    Pass,
    Play(Cards),
}

pub struct GameUpdate {
    pub state: GameState,
    pub board: Cards,
    pub round: u8,
    pub hand: Option<Cards>,
    pub players: [Players; MAX_PLAYERS],
    pub shuffle: PlayerShuffle,
}

#[derive(Clone, Copy)]
pub struct Players {
    pub state: PlayerStatus,
    pub num_cards: u8,
    pub score: Score,
    pub end_score: EndScore,
}

impl Default for Players {
    fn default() -> Self {
        Self {
            state: PlayerStatus::Normal,
            num_cards: 13,
            score: 0,
            end_score: 0,
        }
    }
}

pub struct PlayerShuffle {
    shuffle: u8,
}

impl PlayerShuffle {
    pub fn new(shuffle_id: u8) -> Self {
        // shuffle 4 players only has 24 combinations
        Self {
            shuffle: shuffle_id % 24,
        }
    }
    pub fn shuffle(&self) -> [Player; MAX_PLAYERS] {
        let mut s = 0_u8;
        let want = self.shuffle;

        for a in 0..MAX_PLAYERS {
            for b in 0..MAX_PLAYERS {
                if b == a {
                    continue;
                }
                for c in 0..MAX_PLAYERS {
                    if c == a || c == b {
                        continue;
                    }
                    for d in 0..MAX_PLAYERS {
                        if d == a || d == b || d == c {
                            continue;
                        }
                        if want == s {
                            return [
                                Player::from_idx(a as u8).unwrap(),
                                Player::from_idx(b as u8).unwrap(),
                                Player::from_idx(c as u8).unwrap(),
                                Player::from_idx(d as u8).unwrap(),
                            ];
                        };
                        s = s + 1;
                    }
                }
            }
        }
        [Player::P1, Player::P2, Player::P3, Player::P4]
    }
}

impl Default for GameUpdate {
    fn default() -> Self {
        Self {
            state: GameState::WaitingForPlayers,
            board: 0,
            round: 8,
            shuffle: PlayerShuffle::new(0),
            hand: None,
            players: [Players::default(); 4],
            //player_shuffle: PLayerShuffle::new(0),
        }
    }
}

impl RoomInfo {
    pub fn new(room_id: RoomID, rounds: Option<u8>) -> Self {
        Self {
            room: room_id,
            player: [None, None, None, None],
            spectators: Vec::with_capacity(MAX_SPECTATORS),
            rounds: rounds.unwrap_or(8_u8),
            state: RoomState::WaitingForPlayers,
            update: Default::default(),
        }
    }

    pub fn user_add(&mut self, name: PlayerID) -> Result<Player, String> {
        // Find is PlayerID already exits
        for p in self.player.iter() {
            if let Some(n) = p {
                if *n == name {
                    return Err("Error: User allready exitst".to_string());
                }
            }
        }
        for (i, p) in self.player.iter_mut().enumerate() {
            if p.is_none() {
                *p = Some(name);
                return Ok(Player::from_idx(i as u8).unwrap());
            }
        }
        Err("Room Full!".to_string())
    }
}

//#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameMessage {
    RoomAction(RoomAction),
    GameAction(GameUpdate),
    Disconnected(String),
    Initial {},
    Move { player: Player, action: Action },
    Error { message: String },
}

pub enum RoomAction {
    PlayerJoin {
        idx: Player,
        uid: PlayerID,
        state: RoomState,
    },
    PlayerLeave {
        idx: Player,
        state: RoomState,
    },
    SpectatorJoin(PlayerID),
    SpectatorLeave(PlayerID),
}

pub enum Action1 {
    Deal(Cards),
    Pass,
    Play(Cards),
    Undo,
    Accept,
}

pub enum StepBackAction {
    Request,
    Accept,
    Reject,
}

#[derive(PartialEq, Eq)]
pub enum GameState {
    WaitingForPlayers,
    StepBackRequest,
    ToAct(Player),
    Score,
    EndScore,
}

impl GameState {
    pub fn new() -> Self {
        Self::WaitingForPlayers
    }
}

#[cfg(test)]
mod tests {

    use super::{Player, PlayerShuffle};
    #[test]
    fn test_player_shuffle() {
        let p_0 = PlayerShuffle::new(0).shuffle();
        let p_24 = PlayerShuffle::new(24).shuffle();
        assert_eq!(p_0, p_24);
        assert_eq!(p_0, [Player::P1, Player::P2, Player::P3, Player::P4]);

        let p_23 = PlayerShuffle::new(23).shuffle();
        assert_eq!(p_23, [Player::P4, Player::P3, Player::P2, Player::P1]);
    }
}
