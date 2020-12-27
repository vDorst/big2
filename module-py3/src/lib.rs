use pyo3::prelude::*;
use pyo3::{exceptions::PyValueError, wrap_pyfunction};

use big2::big2rules::{rules, SrvGameError, SrvGameState};
use big2::network::{StateMessage, client};


/// Formats the sum of two numbers as string.
#[pyfunction]
fn score_hand(hand: u64) -> PyResult<u64> {
    Ok(rules::score_hand(hand))
}

/// A Python module implemented in Rust.
#[pymodule]
fn pybig2(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<GameClient>()?;
    m.add_function(wrap_pyfunction!(score_hand, m)?)?;

    Ok(())
}

#[pyclass]
struct GameClient {
    pub sm: StateMessage,
    pub board: u64,
    pub board_score: u64,
    pub cards_selected: u64,
    pub auto_pass: bool,
    pub i_am_ready: bool,
    pub is_valid_hand: bool,
    pub hand_score: u64,
    pub ts: Option<client::TcpClient>,
}

#[pymethods]
impl GameClient {
    #[new]
    fn new() -> Self {
        return GameClient {
            board: 0,
            board_score: 0,
            cards_selected: 0,
            auto_pass: false,
            i_am_ready: true,
            is_valid_hand: false,
            hand_score: 0,
            sm: StateMessage::new(None),
            ts: None,
        };
    }

    pub fn join(&mut self, addr: String, name: String) -> PyResult<()> {
        let client = client::TcpClient::connect(addr);

        if let Err(e) = client {
            return Err(PyValueError::new_err("Unable to connect"));
        }

        let mut ts = client.unwrap();

        if let Err(e) = ts.send_join_msg(&name) {
            return Err(PyValueError::new_err("Unable to send join msg"));
        }

        self.ts = Some(ts);

        return Ok(());
    }

    // pub fn action_play(self, hand: u64) -> PyResult<()> {
    //     let ts = self.ts.unwrap();
    //     let ret = ts.action_play(hand);
    //     if ret.is_err() == true {
    //         return Err(PyValueError::new_err("TCP error: Unable to send PLAY."));
    //     }
    //     Ok(())
    // }
    // pub fn action_pass(self) -> PyResult<()> {
    //     let ts = self.ts.unwrap();
    //     let ret = ts.action_pass();
    //     if ret.is_err() == true {
    //         return Err(PyValueError::new_err("TCP error: Unable to send PASS."));
    //     }
    //     Ok(())
    // }
    pub fn turn(&self) -> PyResult<i32> {
        Ok(self.sm.turn)
    }
    pub fn board(&self) -> PyResult<u64> {
        Ok(self.board)
    }
    pub fn board_score(&self) -> PyResult<u64> {
        Ok(self.board_score)
    }

    pub fn poll(&self) -> PyResult<Option<u64>> {
        let ts = self.ts.unwrap();
        let ret = ts.check_buffer();
        if let Err(e) = ret {
            return Err(PyValueError::new_err("TCP error: Unable POLL DATA"));
        }
        let buffer_sm = ret.unwrap();

        // Process new StateMessage
        if buffer_sm.is_some() {
            self.sm = buffer_sm.unwrap();
            return Ok(Some(self.sm.action_msg()));
        }
        return Ok(None);
    }
}
