pub const RANKS: [u8; 13] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

pub mod deck {
	use rand::thread_rng;
	use rand::seq::SliceRandom;
	use super::*;

	pub const NUMBER_OF_CARDS: u8 = 52;
	pub const START_BIT: u8 = 12;

	pub fn deal() -> [u64; 4] {
		// Create and shulle deck of cards
		let deck = {
				let mut deck = Vec::<u8>::with_capacity(52);

				for s in 0..deck::NUMBER_OF_CARDS {
					let card_bit: u8 = s as u8 + deck::START_BIT;
					deck.push(card_bit);
				}

				// Randomize/shuffle the cards
				for _ in 0..256 {
					deck.shuffle(&mut thread_rng());
				}
				deck
			};
		return deal_cards(deck);
	}
	fn deal_cards(cards: Vec<u8>) -> [u64; 4] {
		let mut players_hand: [u64; 4] = [0,0,0,0];
		let mut p: usize = 0;
		let mut c: usize = 0;

		for r in cards {
			let card_bit = 1 << r;
			players_hand[p] |= card_bit;
			c += 1;
			if c == 13 {
				// println!("p{:x} {:#08x?}", p, player_cards[p]);
				assert!(players_hand[p].count_ones() == 13);
				c = 0;
				p += 1;
			}
		}
		assert!((players_hand[0] | players_hand[1] |
			 players_hand[2] | players_hand[3]) == 0xFFFF_FFFF_FFFF_F000u64);
		return players_hand;
	}
}

pub mod cards {
	#[non_exhaustive]
	pub struct Kind;
	#[non_exhaustive]
	pub struct Rank;

	#[allow(dead_code)]
	impl Kind {
		pub const ONE: u64		= 0x100;
		pub const PAIR: u64		= 0x200;
		pub const SET: u64		= 0x300;
		pub const FIVECARD: u64		= 0x800;
		pub const STRAIGHT: u64		= Kind::FIVECARD | 0x100;
		pub const FLUSH: u64		= Kind::FIVECARD | 0x200;
		pub const FULLHOUSE: u64	= Kind::FIVECARD | 0x300;
		pub const QUADS: u64		= Kind::FIVECARD | 0x400;
		pub const STRAIGHTFLUSH: u64	= Kind::FIVECARD | 0x500;

		pub const SPADES: u64		= 0b1000;
		pub const HEARTS: u64		= 0b0100;
		pub const CLUBS: u64		= 0b0010;
		pub const DIAMONDS: u64		= 0b0001;
		pub const SUITMASK: u64		= 0b1111;

		pub const HIGHEST: u64		= 0x3f;
		pub const LOWEST: u64		= 12;
	}

	#[allow(dead_code)]
	impl Rank {
		pub const THREE: u64		= 3;
		pub const FOUR: u64		= 4;
		pub const FIVE: u64		= 5;
		pub const SIX: u64		= 6;
		pub const SEVEN: u64		= 7;
		pub const EIGTH: u64		= 8;
		pub const NINE: u64		= 9;
		pub const TEN: u64		= 10;
		pub const JACK: u64		= 11;
		pub const QUEEN: u64		= 12;
		pub const KING: u64		= 13;
		pub const ACE: u64		= 14;
		pub const TWO: u64		= 15;
	}

	pub fn has_rank(hand: u64, rank: u64) -> u64 {
		let mask = Kind::SUITMASK << (rank << 2);
		return hand & mask;
	}

	pub fn cnt_rank(hand: u64, rank: u64) -> u64 {
		return has_rank(hand, rank).count_ones() as u64;
	}

	pub fn card_selected(card: u64) -> u64 {
		return card.trailing_zeros() as u64;
	}

	pub fn has_rank_idx(card: u64) -> u64 {
		return card_selected(card) >> 2;
	}

	pub fn has_suit(card: u64) -> u64 {
		return 1 << (card_selected(card) & 0x3);
	}
}

pub mod rules {
	use super::*;

	pub fn get_numbers(hand: u64) {
		let mut ranks: [u32; 16] = [0; 16];
		let mut straigth: u64 = 0;
		let mut tripps: u32 = 0;
		let mut quads:  u32 = 0;
		let mut straigths: u32 = 0;
		let mut doubles: u32 = 0;

		for r in RANKS.iter() {
			let idx: usize = (*r).into();
			ranks[idx] = cards::cnt_rank(hand, idx as u64) as u32;
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
	pub fn is_flush(hand: u64) -> bool {
		let mut mask: u64 = 0x1111_1111_1111_1000;
		for _ in 0..4 {
			if (hand & mask).count_ones() == 5 { return true; }
			mask <<= 1;
		}
		return false;
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
		// Score:
		//	0xKNN = One, Pair and Straigth, Flush
		// 	  |++- highest card: bit nummer of the highest card
		//        +--- Kind: Kind::ONE or Kind::TWO

		//	0xK0R = Set, Quad, FullHouse
		// 	  ||+- Rank: Only the RANK. Because only one RANK of each can exists.
		//	  |+-- Zero
		//        +--- Kind: Kind::QUADS or Kind::SET or Kind::FULLHOUSE

		if is_valid_hand(hand) == false { return 0; }
		let card_cnt_hand: u64 = hand.count_ones().into();

		// find the highest card and calc the rank.
		let highest_card: u64 = 63 - hand.leading_zeros() as u64;
		let rank: u64 = highest_card >> 2;

		// Get the played suit of that rank.
		let suitmask = hand >> (rank << 2);
		// Count number of cards based on the suit
		let cnt: u64 = suitmask.count_ones() as u64;

		if card_cnt_hand <= 3 {
			// If cnt doesn't match the card_cnt then it is invalid hand.
			if cnt != card_cnt_hand { return 0; }

			if card_cnt_hand == 1 { return cards::Kind::ONE | highest_card; }
			if card_cnt_hand == 2 { return cards::Kind::PAIR | highest_card; }
			if card_cnt_hand == 3 { return cards::Kind::SET | rank; }

			return 0;
		}

		let lowest_card: u64 = hand.trailing_zeros() as u64;
		let low_rank: u64 = lowest_card >> 2;
		// Get the played suit of that rank.
		let low_suitmask = hand >> (low_rank << 2) & cards::Kind::SUITMASK;
		// Count number of cards based on the suit
		let low_cnt: u64 = low_suitmask.count_ones() as u64;

		// Quad
		if cnt == 4 { return cards::Kind::QUADS | rank; }
		if low_cnt == 4 { return cards::Kind::QUADS | low_rank; }

		// Full House
		if cnt == 3 && low_cnt == 2 { return cards::Kind::FULLHOUSE | rank; }
		if cnt == 2 && low_cnt == 3 { return cards::Kind::FULLHOUSE | low_rank; }

		// Flush
		let is_flush: bool = is_flush(hand);

		// Straigth detection
		let mut is_straight: bool = rank - low_rank == 4 || rank - low_rank == 12;

		if is_straight {
			let mut straigth_score: u64 = 0;
			if rank - low_rank == 12 {
				is_straight =	cards::has_rank(hand, cards::Rank::THREE) != 0 &&
						cards::has_rank(hand, cards::Rank::FOUR)  != 0 &&
						cards::has_rank(hand, cards::Rank::FIVE)  != 0 &&
						cards::has_rank(hand, cards::Rank::TWO)   != 0;
				// Straight 23456
				if is_straight && cards::has_rank(hand, cards::Rank::SIX) != 0  { straigth_score |= highest_card | 0x40; }
				// Straight A2345
				if is_straight && cards::has_rank(hand, cards::Rank::ACE) != 0 { straigth_score |= highest_card | 0x80; }
			} else {
				is_straight =   cards::has_rank(hand,  low_rank)     != 0 &&
						cards::has_rank(hand,  low_rank + 1) != 0 &&
						cards::has_rank(hand,  low_rank + 2) != 0 &&
						cards::has_rank(hand,  low_rank + 3) != 0 &&
						cards::has_rank(hand,  low_rank + 4) != 0;
				if is_straight { straigth_score = highest_card; }
			}

			is_straight = straigth_score != 0;

			if is_straight {
				if is_flush { return cards::Kind::STRAIGHTFLUSH | straigth_score; }
				return cards::Kind::STRAIGHT | straigth_score;
			}
		}

		if !is_straight && is_flush { return cards::Kind::FLUSH | highest_card; }

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
		// ONE
		assert!(rules::score_hand(0x0000_0000_0000_1000) == cards::Kind::ONE | cards::Kind::LOWEST);
		assert!(rules::score_hand(0x8000_0000_0000_0000) == cards::Kind::ONE | cards::Kind::HIGHEST);


		// PAIR
		assert!(rules::score_hand(0b11 << 12) == cards::Kind::PAIR | 13);
		// Select one 3 and one 4
		assert!(rules::score_hand(0b11000 << 12) == 0);
		assert!(rules::score_hand(0b11 << 12) < rules::score_hand(0b11 << 62));


		// SET
		assert!(rules::score_hand(0b0111 << 12) == cards::Kind::SET | cards::Rank::THREE);
		assert!(rules::score_hand(0b1110 << 12) == cards::Kind::SET | cards::Rank::THREE);
		assert!(rules::score_hand(0b1101 << 12) == cards::Kind::SET | cards::Rank::THREE);
		assert!(rules::score_hand(0b1011 << 12) == cards::Kind::SET | cards::Rank::THREE);
		assert!(rules::score_hand(0b11100 << 12) == 0);
		assert!(rules::score_hand(0b11 << 12) < rules::score_hand(0b11 << 13));


		// QUAD
		assert!(rules::score_hand(0b0001_1111_0000 << 12) == cards::Kind::QUADS | cards::Rank::FOUR);
		assert!(rules::score_hand(0b0000_1111_1000 << 12) == cards::Kind::QUADS | cards::Rank::FOUR);
		assert!(rules::score_hand(0b0001_1111_0000 << 52) == cards::Kind::QUADS | cards::Rank::ACE);
		assert!(rules::score_hand(0b0000_1111_1000 << 52) == cards::Kind::QUADS | cards::Rank::ACE);
		assert!(rules::score_hand(0b1111_0000_1000 << 52) == cards::Kind::QUADS | cards::Rank::TWO);
		assert!(rules::score_hand(0b1111_0001_1000 << 52) == 0);
		assert!(rules::score_hand(0b1111_0000_1001 << 52) == 0);


		// FULL HOUSE
		assert!(rules::score_hand(0b0011_1011_0000 << 12) == cards::Kind::FULLHOUSE | cards::Rank::FOUR);
		assert!(rules::score_hand(0b0000_1101_1001 << 12) == cards::Kind::FULLHOUSE | cards::Rank::FOUR);
		assert!(rules::score_hand(0b0000_1011_0110 << 12) == cards::Kind::FULLHOUSE | cards::Rank::FOUR);
		assert!(rules::score_hand(0b1110_1001_0000 << 52) == cards::Kind::FULLHOUSE | cards::Rank::TWO);
		assert!(rules::score_hand(0b0000_0111_1001 << 52) == cards::Kind::FULLHOUSE | cards::Rank::ACE);
		assert!(rules::score_hand(0b0000_1101_0110 << 52) == cards::Kind::FULLHOUSE | cards::Rank::ACE);


		// STRAIGHT
		assert!(rules::score_hand(0x0002_1111 << 12) == cards::Kind::STRAIGHT | 0x1d);
		assert!(rules::score_hand(0x0002_2221 << 12) == cards::Kind::STRAIGHT | 0x1d);
		assert!(rules::score_hand(0x0000_0002_2221 << 12) == cards::Kind::STRAIGHT | 0x1d);
		// 23456
		assert!(rules::score_hand(0x8000_0000_0111_1000) == cards::Kind::STRAIGHT | cards::Kind::HIGHEST | 0x40);
		// A2345
		assert!(rules::score_hand(0x8200_0000_0011_1000) == cards::Kind::STRAIGHT | cards::Kind::HIGHEST | 0x80);


		// FLUSH
		assert!(rules::score_hand(0x0011_1101 << 12) == cards::Kind::FLUSH | 32);
		assert!(rules::score_hand(0x8800_0000_0808_8000) == cards::Kind::FLUSH | cards::Kind::HIGHEST);


		// STRAIGHT FLUSH
		assert!(rules::score_hand(0x0001_1111 << 12) == cards::Kind::STRAIGHTFLUSH | 0x1c);
		assert!(rules::score_hand(0x1111_1000_0000_0000) == cards::Kind::STRAIGHTFLUSH | 0x3c);
		assert!(rules::score_hand(0x8888_8000_0000_0000) == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST);
		// 23456
		assert!(rules::score_hand(0x8000_0000_0888_8000) == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST | 0x40);
		assert!(rules::score_hand(0x1000_0000_0111_1000) == cards::Kind::STRAIGHTFLUSH | 0x3c | 0x40);
		// A2345
		assert!(rules::score_hand(0x8800_0000_0088_8000) == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST | 0x80);
		assert!(rules::score_hand(0x1100_0000_0011_1000) == cards::Kind::STRAIGHTFLUSH | 0x3c | 0x80);


		// BARBAGE
		assert!(rules::score_hand(0x0001_0311 << 12) == 0);
	}
	#[test]
	fn c_deal_hand() {
		// No cards generated
		assert!(deck::deal() != [0, 0, 0, 0]);
		// Detect shuffle is did not work at all.
		assert!(deck::deal() != [0x1111_1111_1111_1000, 0x2222_2222_2222_2000, 0x4444_4444_4444_4000, 0x8888_8888_8888_8000]);
	}
	#[test]
	fn d_cards_test() {
		// No cards generated
		let card: u64 = 0x1000;
		assert!(cards::has_rank_idx(card) == cards::Rank::THREE);
		assert!(cards::has_suit(card) == cards::Kind::DIAMONDS);
		let card: u64 = 0x20000;
		assert!(cards::has_rank_idx(card) == cards::Rank::FOUR);
		assert!(cards::has_suit(card) == cards::Kind::CLUBS);
		let card: u64 = 0x0400_0000_0000_0000;
		assert!(cards::has_rank_idx(card) == cards::Rank::ACE);
		assert!(cards::has_suit(card) == cards::Kind::HEARTS);
		let card: u64 = 0x8000_0000_0000_0000;
		assert!(cards::has_rank_idx(card) == cards::Rank::TWO);
		assert!(cards::has_suit(card) == cards::Kind::SPADES);
	}
}

