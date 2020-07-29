use std::io::prelude::*;
use std::mem;
use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize)]
enum StateMessage_ActionType {
	update,
	deal,
	play,
	pass,
}

#[derive(Serialize, Deserialize)]
pub struct JoinMessage {
	kind: u32,
	size: u32,
	magicnumber: u32,
	version: u32,
	name: [u8; 16],
	name_size: u32,
}

#[derive(Serialize, Deserialize)]
pub struct StateMessage_Player {
	name: [u8; 16],
	name_size: u32,
	score: i32,
	numCards: i32,
	deltaScore: i32,
	isReady: bool,
	hasPassedThisCycle: bool,
}

#[derive(Serialize, Deserialize)]
pub struct StateMessage {
	kind: u32,
	size: u32,
	round: u32,
	numRounds: u32,
	turn: i32, // -1 means in between rounds
	yourIndex: u32,
	yourHand: [u16; 8],
	players: [StateMessage_Player; 4],
	board: [u8; 8],
	action: StateMessage_ActionType,
}

mod client {
	use super::*;

	pub const PORT: u16 = 27191;
	pub const VERSION: u32 = 4;
	pub const MAGICNUMBER: u32 = 0x3267_6962;

//	01 00 00 00
//	24 00 00 00
//	62 69 67 32
//	04 00 00 00
//	62 6C 61 00
//	00 00 00 00
//	00 00 00 00
//	00 00 00 00
//	03 00 00 00

	pub fn joinMessage (name: String) -> JoinMessage {
		let mut name_bytes: [u8; 16] = [0; 16];
		let str_size = std::cmp::min(name.len(),16);
		name_bytes[..str_size].clone_from_slice(&name.as_bytes()[..str_size]);
		return JoinMessage {
			kind: 1,
			size: std::mem::size_of::<JoinMessage>() as u32,
			magicnumber: MAGICNUMBER,
			version: VERSION,
			name: name_bytes,
			name_size: str_size as u32,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::str::from_utf8;
	use std::mem::MaybeUninit;

	#[test]
	fn a_connect() {
		let JM: JoinMessage = client::joinMessage("René".to_string());
		let JMB = bincode::serialize(&JM).unwrap();
		println!("{:x?}", JMB);
		//                                         12356789T123456
		let JM: JoinMessage = client::joinMessage("René to long to5123123".to_string());
		let JMB = bincode::serialize(&JM).unwrap();
		println!("{:x?}", JMB);
	}
	#[test]
	fn b_connect() {
		let sm_size = std::mem::size_of::<StateMessage>();
		let eb = &[0u8; std::mem::size_of::<StateMessage>()];
		let mut SM: StateMessage = bincode::deserialize(eb).unwrap();
		SM.size = sm_size as u32;
		SM.action = StateMessage_ActionType::play;

		let SMB = bincode::serialize(&SM).unwrap();
		println!("{:x?}", SMB);
	}

}
