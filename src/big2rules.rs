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
		
		let flushs = has_flush(hand);

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

	pub fn is_flush(hand: u64) -> u64 {
		let mut mask: u64 = 0x1111_1111_1111_1000;
		for _ in 0..4 {
			if (hand & mask).count_ones() == 5 { return (mask >> 12) & 0xF; }
			mask <<= 1;
		}
		return 0;
	}

	pub fn has_flush(hand: u64) -> u8 {
		let mut mask: u64 = 0x1111_1111_1111_1000;
		let mut flushs: u8 = 0;
		for _ in 0..4 {
			if (hand & mask).count_ones() >= 5 { flushs += 1; }
			mask <<= 1;
		}
		return flushs;
	}

	pub fn score_hand(hand: u64) -> u64 {
		if is_valid_hand(hand) == false { return 0; }
		// Score:
		// 	
	
		// Score:
		//	0xCRS = one 0x1.. to three cards 0x3..
		// 	  ||+- Suit: Selected suit of that ranked card shifted down.
		//	  |+-- Rank: 3..F 3 = 3 and F = 2
		//        +--- Card count: 1 = single, 2 = two, 3 = three
		
		//	Five cards
		//		5/7 = straigt, 7 =straigth flush
		//		R = card score 
		// 		S = highst suit
		
		//		6 = flush
		//		R = card score
		//		S = suit
		
		//		8 = Full house
		//		R = card score
		//		S = suit
		
		//		9 = Quads
		//		R = card score
		//		S = suit
		let card_cnt_hand: u64 = hand.count_ones().into();

		if card_cnt_hand <= 3 {
			// find the highest card and calc the rank.
			let rank: u64 = (63 - hand.leading_zeros() as u64) >> 2;
			// Get the played suit of that rank.
			let suitmask = hand >> (rank << 2);
			// Count number of cards based on the suit
			let cnt: u64 = suitmask.count_ones() as u64;
			// If cnt doesn't match the card_cnt then it is invalid hand.
			if cnt != card_cnt_hand { return 0; }
			// Return score.
			return (card_cnt_hand << 8) | (rank << 4) | suitmask;
		}

		let mut straigth_bits = 0;
		let mut single: u64 = 0;
		let mut doubles: u64 = 0;
		let mut tripples: u64 = 0;
		let mut flush:	u64 = 0;

		for r in crate::big2rules::RANKS.iter() {
			let rank: u64 = (*r).into();
			let rankmask = (hand >> (rank << 2)) & 0xF;
			
			if rankmask == 0 { 
				straigth_bits = 0;
				continue; 
			}
			
			let cnt: u64 = rankmask.count_ones().into();

			// Found Quads
			if cnt == 4 { return 0x5400 | rank; }

			let rs: u64 = rankmask | (rank << 4);
			
			flush |= rankmask;
			
			if cnt == 1 { single = rs; }
			if cnt == 2 { doubles = rs; }	
			if cnt == 3 { tripples = rs; }
		}
		
		let is_flush: bool = flush.count_ones() == 1;

		if straigth_bits == 5 && is_flush {
			return 0x5600 | single;
		}

		if straigth_bits == 5 && !is_flush {
			return 0x5000 | single;
		}
		
		if is_flush {
			return 0x5500 | single;
		}

		if tripples != 0 && doubles != 0 {
			return 0x5400 | tripples;
		}

		println!("Unknown");
		return 0;
	}	
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_rules_sizes() {
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
	#[test]
	fn b_rules_score_hand() {
		assert!(rules::score_hand(0b1  << 12) == 0x131);
		assert!(rules::score_hand(0b11 << 12) == 0x233);
		// Select one 3 and one 4
		assert!(rules::score_hand(0b11000 << 12) == 0);
		assert!(rules::score_hand(0b11 << 12) < rules::score_hand(0b11 << 13));
		// Select two 3 and one 4
		assert!(rules::score_hand(0b111 << 12) == 0x337);
		assert!(rules::score_hand(0b11100 << 12) == 0);
		assert!(rules::score_hand(0b11 << 12) < rules::score_hand(0b11 << 13));
		
		// flush
		assert!(rules::score_hand(0x11111 << 12) == 0);
	}
}
