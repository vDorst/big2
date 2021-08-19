use std::collections::HashMap;

use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, StreamHandler};
use actix_web_actors::ws;
use serde_cbor::ser;
use tokio::task::JoinHandle;
use tokio::time;
use uuid::Uuid;

use big2lib::{
    messages::{GameMessage, GameState},
    players::{Move, Player},
};

use crate::messages::{
    AddressedGameMessage, CloseRoom, CreateRoom, JoinRoom, JoinedRoom, LeftRoom, SocketGameMessage,
};

/// Socket

///
pub struct RoomWs {
    room: Option<Addr<Big2Room>>,
    server: Addr<Big2Server>,
    room_key: Option<Uuid>,
    id: String,
}

impl RoomWs {
    pub fn new(server: Addr<Big2Server>, room_key: Option<Uuid>, id: String) -> RoomWs {
        RoomWs {
            room: None,
            server,
            room_key,
            id,
        }
    }
}

impl Actor for RoomWs {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        match self.room_key {
            None => {
                let msg = CreateRoom(addr);
                self.server.do_send(msg);
            }
            Some(room_key) => {
                let msg = JoinRoom { addr, room_key };
                self.server.do_send(msg);
            }
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for RoomWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let data = match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
                return;
            }
            Ok(ws::Message::Binary(data)) => data,
            Ok(ws::Message::Close(reason)) => {
                info!("Connection closed, reason: {:?}", reason);
                if let Some(room) = &self.room {
                    room.do_send(LeftRoom(ctx.address()));
                }
                return;
            }
            Ok(ws::Message::Text(_)) => {
                ctx.text("pong");
                return;
            }
            msg => {
                warn!("Received unexpected msg: {:?}", msg);
                return;
            }
        };
        let msg: GameMessage = match serde_cbor::from_slice(data.as_ref()) {
            Ok(msg) => msg,
            Err(err) => {
                warn!("Error deserializing player message: {:?}", err);
                return;
            }
        };
        let room = match &self.room {
            Some(room) => room,
            None => {
                warn!("Message sent too early");
                ctx.text("Error: Message too early");
                return;
            }
        };
        let msg = AddressedGameMessage {
            sender: ctx.address(),
            msg,
        };
        room.do_send(msg);
    }
}

impl Handler<JoinedRoom> for RoomWs {
    type Result = ();
    fn handle(&mut self, msg: JoinedRoom, ctx: &mut Self::Context) {
        let msg = match msg {
            JoinedRoom::Error { message } => GameMessage::Error { message },
            JoinedRoom::Success {
                addr,
                room_key,
                player,
                // state,
            } => {
                info!("Joined room {} as {:?}: {}", room_key, player, self.id);
                self.room = Some(addr);
                self.room_key = Some(room_key);
                GameMessage::Initialize {
                    // state,
                    room_id: room_key.to_string(),
                    player,
                    waiting: false,
                }
            }
        };
        let msg = ser::to_vec(&msg).expect("failed to serialize initialize message");
        ctx.binary(msg);
    }
}

impl Handler<SocketGameMessage> for RoomWs {
    type Result = ();
    fn handle(&mut self, msg: SocketGameMessage, ctx: &mut Self::Context) {
        let SocketGameMessage(msg) = msg;
        let data = ser::to_vec(&msg).expect("Failed to serialize message");
        ctx.binary(data);
    }
}

/// Room
///
pub struct Big2Room {
    game_state: GameState,
    players: Vec<Addr<RoomWs>>,
    key: Uuid,
    requested_rematch: Option<Player>,
    close_room_handle: Option<JoinHandle<()>>,
}

impl Big2Room {
    pub fn new() -> Big2Room {
        Big2Room {
            game_state: GameState::new(),
            players: Vec::with_capacity(4),
            key: Uuid::new_v4(),
            requested_rematch: None,
            close_room_handle: None,
        }
    }
}

impl Actor for Big2Room {
    type Context = Context<Self>;
}

impl Big2Room {
    fn send_to_player(&self, player: Player, msg: GameMessage) {
        let msg = SocketGameMessage(msg);
        let p_idx = player.to_idx() as usize;
        for (idx, sock) in self.players.iter().enumerate() {
            if idx == p_idx {
                sock.do_send(msg);
                break;
            }
        }
    }
    fn send_to_other_players(&self, player: Player, msg: GameMessage) {
        let p_idx = player.to_idx() as usize;
        for (idx, sock) in self.players.iter().enumerate() {
            if idx == p_idx {
                continue;
            }
            let msg = SocketGameMessage(msg.clone());
            sock.do_send(msg);
        }
    }
    fn player_from_addr(&self, addr: &Addr<RoomWs>) -> Option<Player> {
        for (idx, sock) in self.players.iter().enumerate() {
            if sock == addr {
                return Player::from_idx(idx as u8);
            }
        }
        return None;
    }
    fn broadcast(&self, msg: GameMessage) {
        for sock in self.players.iter() {
            let msg = SocketGameMessage(msg.clone());
            sock.do_send(msg);
        }
    }
}

impl Handler<JoinRoom> for Big2Room {
    type Result = ();
    fn handle(&mut self, msg: JoinRoom, ctx: &mut Self::Context) {
        let socket = msg.addr;

        let full = self.players.len() == 4;
        if full {
            let message = "Room is full".to_string();
            let msg = JoinedRoom::Error { message };
            socket.do_send(msg);
            return;
        }

        match &self.close_room_handle {
            None => {}
            Some(handle) => {
                info!("Canceled room close: {}", self.key);
                handle.abort();
                self.close_room_handle = None;
            }
        };
        let addr = socket.clone();

        self.players.push(addr);

        let player = Player::from_idx(self.players.len() as u8 - 1).unwrap();

        let waiting = self.players.len() != 4;

        let addr = ctx.address();
        let room_key = self.key;
        let msg = JoinedRoom::Success {
            addr,
            room_key,
            player,
        };
        socket.do_send(msg);
        // Send join message
        self.send_to_other_players(player, GameMessage::Joined);
    }
}

async fn delay_exit(addr: Addr<Big2Room>) {
    time::sleep(time::Duration::from_secs(15)).await;
    addr.do_send(CloseRoom {});
}

impl Handler<LeftRoom> for Big2Room {
    type Result = ();
    fn handle(&mut self, msg: LeftRoom, ctx: &mut Self::Context) {
        info!("Player left room: {}", self.key);
        let LeftRoom(addr) = msg;
        self.players.retain(|a| a != &addr);
        if self.players.is_empty() {
            info!("Room Empty: {}", self.key.clone());
            let addr = ctx.address();
            let handle = tokio::spawn(delay_exit(addr));
            self.close_room_handle = Some(handle);
        } else {
            self.broadcast(GameMessage::Disconnected);
        }
    }
}

impl Handler<CloseRoom> for Big2Room {
    type Result = ();
    fn handle(&mut self, _msg: CloseRoom, ctx: &mut Self::Context) {
        if self.players.is_empty() {
            info!("Room Closing: {}", self.key.clone());
            ctx.stop();
        } else {
            error!("Almost closed running room ({}), this is a bug", self.key);
        }
    }
}

impl Big2Room {
    fn handle_move(&mut self, game_move: Move, player: Player) {
        // let board = match self.game_state {
        //     GameState::Playing { board } => board,
        //     GameState::Finished { .. } => {
        //         info!("Attempted move on finished game");
        //         return;
        //     }
        // };
        // if board.turn != player {
        //     error!("Not player's turn");
        //     return;
        // }
        // let new_state = match board.try_move(game_move) {
        //     Ok(new_state) => new_state,
        //     Err(err) => {
        //         error!("Player played illegal move: {:?}", err);
        //         return;
        //     }
        // };
        // self.game_state = new_state;
        // let next_player = player.invert();
        // let msg = GameMessage::Move { game_move };
        // self.send_to_player(next_player, msg);
    }
    fn handle_rematch_request(&mut self, player: Player) {
        //     let requested_player = match self.requested_rematch {
        //         None => {
        //             self.requested_rematch = Some(player);
        //             let msg = GameMessage::RequestRematch;
        //             self_send_to_other_players(player, msg);
        //             self.send_to_player(other_player, msg);
        //             return;
        //         }
        //         Some(requested_player) => requested_player,
        //     };
        //     if requested_player == player {
        //         info!("Player requsted rematch multiple times");
        //     } else {
        //         self.requested_rematch = None;
        //         let state = GameState::new();
        //         self.game_state = state;
        //         self.send_to_player(
        //             Player::Red,
        //             GameMessage::Initialize {
        //                 state,
        //                 room_id: self.key.to_string(),
        //                 waiting: false,
        //             },
        //         );
        //         self.send_to_player(
        //             Player::Blue,
        //             GameMessage::Initialize {
        //                 state,
        //                 room_id: self.key.to_string(),
        //                 waiting: false,
        //             },
        //         );
        //     }
    }
}

impl Handler<AddressedGameMessage> for Big2Room {
    type Result = ();
    fn handle(&mut self, msg: AddressedGameMessage, _ctx: &mut Self::Context) {
        let AddressedGameMessage { sender, msg } = msg;
        let player = match self.player_from_addr(&sender) {
            Some(player) => player,
            None => {
                error!("Received game message from socket that's not in game");
                let msg = GameMessage::Error {
                    message: "Player not in game".to_string(),
                };
                sender.do_send(SocketGameMessage(msg));
                return;
            }
        };
        match msg {
            // GameMessage::Move { game_move } => {
            //     self.handle_move(game_move, player);
            // }
            // GameMessage::RequestRematch => {
            //     self.handle_rematch_request(player);
            // }
            // GameMessage::Error { message } => {
            //     error!("Received error from client: {}", message);
            //     return;
            // }
            msg => {
                info!("Unexpected msg: {:?}", msg);
                return;
            }
        };
    }
}

/// Server
///
pub struct Big2Server {
    rooms: HashMap<Uuid, Addr<Big2Room>>,
}

impl Big2Server {
    pub fn new() -> Big2Server {
        Big2Server {
            rooms: HashMap::new(),
        }
    }
}

impl Actor for Big2Server {
    type Context = Context<Self>;
}

impl Handler<JoinRoom> for Big2Server {
    type Result = ();
    fn handle(&mut self, msg: JoinRoom, _: &mut Self::Context) {
        let room_key = msg.room_key.clone();
        let addr = msg.addr.clone();
        let room = match self.rooms.get(&room_key) {
            None => {
                warn!("Player attempted to join non-existent room: {}", &room_key);
                let err_resp = GameMessage::Error {
                    message: "Requested room doesn't exist".to_string(),
                };
                addr.do_send(SocketGameMessage(err_resp));
                return;
            }
            Some(room) => room,
        };
        match room.try_send(msg) {
            Ok(_) => {}
            Err(_) => {
                warn!("Player attempted to join closed room: {}", &room_key);
                let err_resp = GameMessage::Error {
                    message: "Requested room was closed due to being empty too long".to_string(),
                };
                addr.do_send(SocketGameMessage(err_resp));
            }
        };
    }
}

impl Handler<CreateRoom> for Big2Server {
    type Result = ();
    fn handle(&mut self, msg: CreateRoom, _: &mut Self::Context) {
        println!("Server received create room request");
        let room = Big2Room::new();
        let room_key = room.key;
        let room = room.start();
        self.rooms.insert(room_key, room.clone());
        let msg = JoinRoom {
            addr: msg.0,
            room_key,
        };
        room.do_send(msg);
    }
}
