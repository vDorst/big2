use pyo3::prelude::*;
use pyo3::{exceptions::PyValueError, wrap_pyfunction};

use big2::big2rules::{rules, SrvGameError, SrvGameState};

/// Formats the sum of two numbers as string.
#[pyfunction]
fn score_hand(hand: u64) -> PyResult<u64> {
    Ok(rules::score_hand(hand))
}

/// A Python module implemented in Rust.
#[pymodule]
fn pybig2(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<GameServer>()?;
    m.add_function(wrap_pyfunction!(score_hand, m)?)?;

    Ok(())
}

#[pyclass]
struct GameServer {
    pub gs: SrvGameState,
}

#[pymethods]
impl GameServer {
    #[new]
    fn new(rounds: u8) -> Self {
        GameServer {
            gs: SrvGameState::new(rounds),
        }
    }
    pub fn action_play(&mut self, p: i32, hand: u64) -> PyResult<()> {
        let ret = self.gs.play(p, hand);
        match ret {
            Ok(_) => Ok(()),
            Err(SrvGameError::PlayerPlayedIllegalCard(_)) => {
                Err(PyValueError::new_err("Playing Illegal Cards!"))
            }
            Err(SrvGameError::InvalidHand) => Err(PyValueError::new_err("InvalidHand")),
            Err(SrvGameError::NotPlayersTurn) => Err(PyValueError::new_err("It is not your turn")),
            _ => Err(PyValueError::new_err("Unknown")),
        }
    }
    pub fn action_pass(&mut self, p: i32) -> PyResult<()> {
        let ret = self.gs.pass(p);
        match ret {
            Ok(_) => Ok(()),
            Err(SrvGameError::AllreadyPassed) => Err(PyValueError::new_err("AllreadyPassed")),
            Err(SrvGameError::NotPlayersTurn) => Err(PyValueError::new_err("It is not your turn")),
            _ => Err(PyValueError::new_err("Unknown")),
        }
    }
    pub fn deal(&mut self, cards: Option< Vec::<u64> >) -> PyResult<()> {
        if cards.is_none() {
            self.gs.deal(None)
        } else {
    	    let cl = cards.unwrap();
    	    if cl.len() != 4 {
    		return Err(PyValueError::new_err("List must be 4 items!"));
    	    }
    	    self.gs.deal( Some(&cl) );
        }
        Ok(())
    }
    pub fn turn(&self) -> PyResult<i32> {
        Ok(self.gs.turn)
    }
    pub fn board(&self) -> PyResult<u64> {
        Ok(self.gs.last_action & 0xFFFF_FFFF_FFFF_F000)
    }
    pub fn board_score(&self) -> PyResult<u64> {
        Ok(self.gs.board_score)
    }
}
