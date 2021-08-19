use actix::{Addr, Message};
use big2lib::{messages::GameMessage, players::Player};
use uuid::Uuid;

// use big2lib::{GameMessage, GameState, Player};

use crate::rooms::{Big2Room, RoomWs};

#[derive(Message)]
#[rtype(result = "()")]
pub struct JoinRoom {
    pub addr: Addr<RoomWs>,
    pub room_key: Uuid,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CreateRoom(pub Addr<RoomWs>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeftRoom(pub Addr<RoomWs>);

#[derive(Message)]
#[rtype(result = "()")]
pub enum JoinedRoom {
    Success {
        addr: Addr<Big2Room>,
        room_key: Uuid,
        player: Player,
        // state: GameState,
    },
    Error {
        message: String,
    },
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddressedGameMessage {
    pub sender: Addr<RoomWs>,
    pub msg: GameMessage,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SocketGameMessage(pub GameMessage);

#[derive(Message)]
#[rtype(result = "()")]
pub struct CloseRoom;

cfg_if::cfg_if! {
    if #[cfg(feature = "agent")] {
        use crate::agents::{AgentException, AgentWs};

        #[derive(Message)]
        #[rtype(result = "()")]
        pub struct AgentRequest {
            pub msg: GameMessage,
            pub addr: Addr<AgentWs>,
        }

        #[derive(Message)]
        #[rtype(result = "()")]
        pub struct AgentResponse {
            pub resp: Result<GameMessage,AgentException>,
        }
    }
}
