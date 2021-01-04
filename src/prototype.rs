#![allow(dead_code)]

use std::{
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

#[derive(Debug, PartialEq)]
enum BIG2Errors {
    NotYourTurn,
    InValidMove,
}

// Needed to get current state of the game after rejoin
#[derive(Debug, PartialEq)]
struct BIG2Game {
    pub uid: [u64; 4],
    pub is_ready_passed: [bool; 4],
    pub game_state: u8, // 1:0, player to act/assisted, 2 end of the game, 3 assisted?.
    pub round: u8,
    pub rounds: u8,
    pub yourcards: u64,
    pub board: u64,
    // round = 255 max, max rounds 255 * cards 13 * multiplier 3 * players 3 = 29835 -> fits in 14 bits
    pub score: [i16; 4],
    // 0-13 -> 4bits, all fit in u32
    pub card_cnt: [u8; 4],
}

#[derive(Debug, PartialEq)]
struct BIG2Users {
    pub uid: u64,
    pub name: [u8; 16],   // Null terminated string like C
    pub tables: Vec<u64>, // table id
}

#[derive(Debug, PartialEq)]
enum BIG2ServerActions {
    PASS,
    PLAY(u64),
    READY,
    LEAVE,
    JOIN(u64),
    STATUS(BIG2Game),
    ERROR(BIG2Errors),
    UIDINFO(BIG2Users),
}

#[derive(Debug, PartialEq)]
enum BIG2ClientActions {
    UID(u64),
    PASS,
    PLAY(u64),
    READY,
    LEAVE,
}

struct BIG2ClientAPI {
    // Host Output Client Input
    hoci: Sender<BIG2ServerActions>,
    // Host Input Client Output
    hico: Receiver<BIG2ClientActions>,
}

impl BIG2ClientAPI {
    pub fn new() -> Result<
        (
            Self,
            (Sender<BIG2ClientActions>, Receiver<BIG2ServerActions>),
        ),
        (),
    > {
        // Transmit to other side
        let (hoci, ciho) = std::sync::mpsc::channel();
        let (cohi, hico) = std::sync::mpsc::channel();

        Ok((Self { hoci, hico }, (cohi, ciho)))
    }

    pub fn push(&mut self, action: BIG2ServerActions) -> Result<(), ()> {
        let ret = self.hoci.send(action);
        if ret.is_err() {
            return Err(());
        }
        Ok(())
    }

    pub fn pull(&mut self) -> Result<BIG2ClientActions, ()> {
        let timeout = Duration::from_millis(10);
        let ret = self.hico.recv_timeout(timeout);
        if ret.is_err() {
            return Err(());
        }
        Ok(ret.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_big2client_api() {
        let b2c = BIG2ClientAPI::new();
        assert!(b2c.is_ok());
        let (mut client1, (cohi, ciho)) = b2c.unwrap();

        assert!(client1.push(BIG2ServerActions::PASS).is_ok());

        let host_data = ciho.recv();
        assert!(host_data.is_ok());

        let host_action = host_data.unwrap();
        assert_eq!(host_action, BIG2ServerActions::PASS);

        assert!(client1.push(BIG2ServerActions::PLAY(1)).is_ok());
        let host_data = ciho.recv();
        assert!(host_data.is_ok());

        let host_action = host_data.unwrap();
        assert_eq!(host_action, BIG2ServerActions::PLAY(1));
    }
    #[test]
    fn create_big2client_api_client() {
        let b2c = BIG2ClientAPI::new();
        assert!(b2c.is_ok());
        let (mut client1, (cohi, ciho)) = b2c.unwrap();

        let ret = cohi.send(BIG2ClientActions::PASS);
        assert!(ret.is_ok());

        let ret = client1.pull();

        assert!(ret.is_ok());

        let client_data = ret.unwrap();
        assert_eq!(BIG2ClientActions::PASS, client_data);
    }

    #[test]
    fn create_big2client_api_client_err() {
        let b2c = BIG2ClientAPI::new();
        assert!(b2c.is_ok());
        let (mut client1, (cohi, ciho)) = b2c.unwrap();

        // Succes full action
        let ret = cohi.send(BIG2ClientActions::PASS);
        assert!(ret.is_ok());

        let ret = client1.pull();

        assert!(ret.is_ok());

        let client_data = ret.unwrap();
        assert_eq!(BIG2ClientActions::PASS, client_data);

        // Action fall
        std::mem::drop(cohi);
        let ret = client1.pull();

        assert!(ret.is_err());
    }
}
