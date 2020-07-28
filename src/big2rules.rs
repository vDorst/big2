pub const SUITS: [u8; 4]  = [0x0, 0x1, 0x2, 0x3];
pub const RANKS: [u8; 13] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

pub mod rules {
	pub fn get_numbers(hand: u64) {
		let mut ranks: [u32; 16] = [0; 16];
		let mut straigth: u64 = 0;
		let mut tripps: u32 = 0;
		let mut quads:  u32 = 0;
		let mut straigths: u32 = 0;
		let mut doubles: u32 = 0;
		
		for r in crate::big2rules::RANKS.iter() {
			let idx: usize = (*r).into();
			let bitmask = 0xF << (r << 2);
			let rankmask = hand & bitmask;
			ranks[idx] = rankmask.count_ones();
			if ranks[idx] != 0 { straigth |= 1 << r; }
			if ranks[idx] == 2 { doubles += 1; }	
			if ranks[idx] == 3 { tripps += 1; }
			if ranks[idx] == 4 { quads += 1; }
		}
		let mut mask = 0b11111;
		for _ in 4..16 {
			if straigth & mask == mask { straigths += 1; }
			mask <<= 1;
		}
		// A2345
		mask = 0b1100_0000_0011_1000;
		if straigth & mask == mask { straigths += 1;};
		// 23456
		mask = 0b1000_0000_0111_1000;
		if straigth & mask == mask { straigths += 1;};
		
		let mut flushs:u32 = 0;
		//       2AKQ JT98 7654 3xxx
		mask = 0x1111_1111_1111_1000;
		for _ in 0..4 {
			let cnt = (hand & mask).count_ones();
			// println!("{:64b} {} {}", hand & mask, cnt, cnt / 5);
			flushs += cnt / 5;
			mask <<= 1;
		}

		let fullhouse = std::cmp::min(doubles, tripps) + std::cmp::min(doubles, quads) + std::cmp::min(tripps, quads);
		println!("R{:x?} S{:16b} {:x} D{:x} T{:x} Q{:x} FH{:x} FL{:x}", ranks, straigth, straigths, doubles, tripps, quads, fullhouse, flushs);
	}
	pub fn is_valid_hand(hand: u64) -> bool {
		// Check cards range. Only the upper 52 bits are used.
		if hand & 0xFFF != 0 { return false; }

		// Check number of cards played.
		let cardcount = hand.count_ones();
		// println!("Card count: {}", cardcount);
		if cardcount != 1 && cardcount != 2 &&
		   cardcount != 3 && cardcount != 5 {
			return false;
		}

		return true;
	}
	pub fn beter_hand(board: u64, hand: u64) -> bool {
		if is_valid_hand(hand) == false { return false; }

		let card_cnt_hand = hand.count_ones();
		let card_cnt_board = board.count_ones();

		// Board and hand count must match.
		// Board count 0 means new turn.
		if card_cnt_board != 0 &&
		   card_cnt_board != card_cnt_hand {
			return false;
		}

		return true;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rules_sizes() {
	        assert!(rules::is_valid_hand(0b1          ) == false, "1 invalid card");
	        assert!(rules::is_valid_hand(0b1     << 12) == true);
	        assert!(rules::is_valid_hand(0b11    << 12) == true);
	        assert!(rules::is_valid_hand(0b111   << 12) == true);
	        assert!(rules::is_valid_hand(0b1111  << 12) == false, "4 cards");
	        assert!(rules::is_valid_hand(0b11111 << 12) == true);
		assert!(rules::is_valid_hand(0b111111<< 12) == false, "6 cards");
	}
	#[test]
	fn rules_board_hand_new_turn() {
		assert!(rules::beter_hand(0, 0b1      << 12));
		assert!(rules::beter_hand(0, 0b11     << 12));
		assert!(rules::beter_hand(0, 0b111    << 12));
		assert!(rules::beter_hand(0, 0b1111   << 12) == false);
		assert!(rules::beter_hand(0, 0b11111  << 12));
	}
	#[test]
	fn rules_board_hand_one_pair() {
		assert!(rules::beter_hand(0b1 << 12, 0b1      << 12));
		assert!(rules::beter_hand(0b1 << 12, 0b11     << 12) == false);
		assert!(rules::beter_hand(0b1 << 12, 0b111    << 12) == false);
		assert!(rules::beter_hand(0b1 << 12, 0b11111  << 12) == false);
	}
	#[test]
	fn rules_board_hand_two_pair() {
		assert!(rules::beter_hand(0b11 << 12, 0b1      << 12) == false);
		assert!(rules::beter_hand(0b11 << 12, 0b11     << 12));
		assert!(rules::beter_hand(0b11 << 12, 0b111    << 12) == false);
		assert!(rules::beter_hand(0b11 << 12, 0b11111  << 12) == false);
	}
	#[test]
	fn rules_board_hand_three_of_kind() {
		assert!(rules::beter_hand(0b111 << 12, 0b1      << 12) == false);
		assert!(rules::beter_hand(0b111 << 12, 0b11     << 12) == false);
		assert!(rules::beter_hand(0b111 << 12, 0b111    << 12) == true);
		assert!(rules::beter_hand(0b111 << 12, 0b11111  << 12) == false);
	}
	#[test]
	fn rules_board_hand_fivecards() {
		assert!(rules::beter_hand(0b11111 << 12, 0b1      << 12) == false);
		assert!(rules::beter_hand(0b11111 << 12, 0b11     << 12) == false);
		assert!(rules::beter_hand(0b11111 << 12, 0b111    << 12) == false);
		assert!(rules::beter_hand(0b11111 << 12, 0b11111  << 12));
	}
}
