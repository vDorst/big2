#![allow(dead_code)]
//#![allow(unused_imports)]
//#![allow(unused_variables)]
use crate::network;

pub const RANKS: [u8; 13] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

pub mod deck {
    use super::*;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

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
        let mut players_hand: [u64; 4] = [0, 0, 0, 0];
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
        assert!(
            (players_hand[0] | players_hand[1] | players_hand[2] | players_hand[3])
                == 0xFFFF_FFFF_FFFF_F000u64
        );
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
        pub const ONE: u64 = 0x100;
        pub const PAIR: u64 = 0x200;
        pub const SET: u64 = 0x300;
        pub const FIVECARD: u64 = 0x800;
        pub const STRAIGHT: u64 = Kind::FIVECARD | 0x100;
        pub const FLUSH: u64 = Kind::FIVECARD | 0x200;
        pub const FULLHOUSE: u64 = Kind::FIVECARD | 0x300;
        pub const QUADS: u64 = Kind::FIVECARD | 0x400;
        pub const STRAIGHTFLUSH: u64 = Kind::FIVECARD | 0x500;
        pub const TYPE: u64 = 0xF00;

        pub const SPADES: u64 = 0b1000;
        pub const HEARTS: u64 = 0b0100;
        pub const CLUBS: u64 = 0b0010;
        pub const DIAMONDS: u64 = 0b0001;
        pub const SUITMASK: u64 = 0b1111;

        pub const HIGHEST: u64 = 0x3f;
        pub const LOWEST: u64 = 12;
    }

    #[allow(dead_code)]
    impl Rank {
        pub const THREE: u64 = 3;
        pub const FOUR: u64 = 4;
        pub const FIVE: u64 = 5;
        pub const SIX: u64 = 6;
        pub const SEVEN: u64 = 7;
        pub const EIGTH: u64 = 8;
        pub const NINE: u64 = 9;
        pub const TEN: u64 = 10;
        pub const JACK: u64 = 11;
        pub const QUEEN: u64 = 12;
        pub const KING: u64 = 13;
        pub const ACE: u64 = 14;
        pub const TWO: u64 = 15;
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
        let mut quads: u32 = 0;
        let mut straigths: u32 = 0;
        let mut doubles: u32 = 0;

        for r in RANKS.iter() {
            let idx: usize = (*r).into();
            ranks[idx] = cards::cnt_rank(hand, idx as u64) as u32;
            if ranks[idx] != 0 {
                straigth |= 1 << r;
            }
            if ranks[idx] == 2 {
                doubles += 1;
            }
            if ranks[idx] == 3 {
                tripps += 1;
            }
            if ranks[idx] == 4 {
                quads += 1;
            }
        }
        let mut mask = 0b11111;
        for _ in 4..16 {
            if straigth & mask == mask {
                straigths += 1;
            }
            mask <<= 1;
        }
        // A2345
        mask = 0b1100_0000_0011_1000;
        if straigth & mask == mask {
            straigths += 1;
        };
        // 23456
        mask = 0b1000_0000_0111_1000;
        if straigth & mask == mask {
            straigths += 1;
        };

        let flushs = has_flush(hand);

        let fullhouse = std::cmp::min(doubles, tripps)
            + std::cmp::min(doubles, quads)
            + std::cmp::min(tripps, quads);
        println!(
            "R{:x?} S{:16b} {:x} D{:x} T{:x} Q{:x} FH{:x} FL{:x}",
            ranks, straigth, straigths, doubles, tripps, quads, fullhouse, flushs
        );
    }
    pub fn is_valid_hand(hand: u64) -> bool {
        // Check cards range. Only the upper 52 bits are used.
        let ret: bool = (hand & 0xFFF) == 0;

        // Check number of cards played. count = 1, 2, 3 or 5 is valid.
        let cardcount = hand.count_ones();
        ret && cardcount != 4 && cardcount < 6 && cardcount != 0
    }
    pub fn beter_hand(board: u64, hand: u64) -> bool {
        if is_valid_hand(hand) == false {
            return false;
        }

        let card_cnt_hand = hand.count_ones();
        let card_cnt_board = board.count_ones();

        // Board and hand count must match.
        // Board count 0 means new turn.
        if card_cnt_board != 0 && card_cnt_board != card_cnt_hand {
            return false;
        }
        return true;
    }
    pub fn is_flush(hand: u64) -> bool {
        let mut mask: u64 = 0x1111_1111_1111_1000;
        for _ in 0..4 {
            if (hand & !mask) == 0 {
                return true;
            }
            mask <<= 1;
        }
        return false;
    }
    pub fn has_flush(hand: u64) -> u8 {
        let mut mask: u64 = 0x1111_1111_1111_1000;
        let mut flushs: u8 = 0;
        for _ in 0..4 {
            if (hand & mask).count_ones() >= 5 {
                flushs += 1;
            }
            mask <<= 1;
        }
        return flushs;
    }
    pub fn score_hand(hand: u64) -> u64 {
        // Score:
        //  0xKNN = One, Pair and Straigth, Flush
        //    |++- highest card: bit nummer of the highest card
        //        +--- Kind: Kind::ONE or Kind::TWO

        //  0xK0R = Set, Quad, FullHouse
        //    ||+- Rank: Only the RANK. Because only one RANK of each can exists.
        //    |+-- Zero
        //        +--- Kind: Kind::QUADS or Kind::SET or Kind::FULLHOUSE

        if is_valid_hand(hand) == false {
            return 0;
        }
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
            if cnt != card_cnt_hand {
                return 0;
            }

            if card_cnt_hand == 1 {
                return cards::Kind::ONE | highest_card;
            }
            if card_cnt_hand == 2 {
                return cards::Kind::PAIR | highest_card;
            }
            if card_cnt_hand == 3 {
                return cards::Kind::SET | rank;
            }

            return 0;
        }

        let lowest_card: u64 = hand.trailing_zeros() as u64;
        let low_rank: u64 = lowest_card >> 2;
        // Get the played suit of that rank.
        let low_suitmask = hand >> (low_rank << 2) & cards::Kind::SUITMASK;
        // Count number of cards based on the suit
        let low_cnt: u64 = low_suitmask.count_ones() as u64;

        // Quad
        if cnt == 4 {
            return cards::Kind::QUADS | rank;
        }
        if low_cnt == 4 {
            return cards::Kind::QUADS | low_rank;
        }

        // Full House
        if cnt == 3 && low_cnt == 2 {
            return cards::Kind::FULLHOUSE | rank;
        }
        if cnt == 2 && low_cnt == 3 {
            return cards::Kind::FULLHOUSE | low_rank;
        }

        // Flush
        let is_flush: bool = is_flush(hand);

        // Straigth detection
        let mut is_straight: bool = rank - low_rank == 4 || rank - low_rank == 12;

        if is_straight {
            let mut straigth_score: u64 = 0;
            if rank - low_rank == 12 {
                is_straight = cards::has_rank(hand, cards::Rank::THREE) != 0
                    && cards::has_rank(hand, cards::Rank::FOUR) != 0
                    && cards::has_rank(hand, cards::Rank::FIVE) != 0
                    && cards::has_rank(hand, cards::Rank::TWO) != 0;
                // Straight 23456
                if is_straight && cards::has_rank(hand, cards::Rank::SIX) != 0 {
                    straigth_score |= highest_card | 0x40;
                }
                // Straight A2345
                if is_straight && cards::has_rank(hand, cards::Rank::ACE) != 0 {
                    straigth_score |= highest_card | 0x80;
                }
            } else {
                is_straight = cards::has_rank(hand, low_rank) != 0
                    && cards::has_rank(hand, low_rank + 1) != 0
                    && cards::has_rank(hand, low_rank + 2) != 0
                    && cards::has_rank(hand, low_rank + 3) != 0
                    && cards::has_rank(hand, low_rank + 4) != 0;
                if is_straight {
                    straigth_score = highest_card;
                }
            }

            is_straight = straigth_score != 0;

            if is_straight {
                if is_flush {
                    return cards::Kind::STRAIGHTFLUSH | straigth_score;
                }
                return cards::Kind::STRAIGHT | straigth_score;
            }
        }

        if !is_straight && is_flush {
            return cards::Kind::FLUSH | highest_card;
        }

        return 0;
    }
}

pub struct GameState {
    pub sm: network::StateMessage,
    pub srn: std::io::Stdout,
    pub board: u64,
    pub board_score: u64,
    pub cards_selected: u64,
    pub auto_pass: bool,
    pub i_am_ready: bool,
    pub is_valid_hand: bool,
    pub hand_score: u64,
}

pub struct SrvGameState {
    pub last_action: u64,
    pub board_score: u64,
    pub has_passed: u8,
    pub turn: i32,
    pub round: u8,
    pub rounds: u8,
    pub cards: [u64; 4],
    pub played_cards: u64,
    pub score: [i16; 4],
    pub card_cnt: [u8; 4],
}

#[derive(Debug)]
pub enum SrvGameError {
    NotPlayersTurn,
    PlayerAlreadyPlayedCard(u64),
    PlayerPlayedIllegalCard(u64),
    InvalidHand,
    AllreadyPassed,
}

impl SrvGameState {
    pub fn new(rounds: u8) -> Self {
        SrvGameState {
            last_action: 0,
            board_score: 0,
            has_passed: 0,
            turn: -1,
            round: 0,
            rounds: rounds,
            cards: [0; 4],
            played_cards: 0,
            score: [0; 4],
            card_cnt: [13; 4],
        }
    }
    pub fn deal(&mut self, cards: Option<&[u64]>) {
        // create cards
        if let Some(cards) = cards {
            assert_eq!(cards.len(), 4);
            self.cards.copy_from_slice(cards);
        } else {
            self.cards = deck::deal();
        }

        // Setup
        self.round += 1;
        self.has_passed = 0;
        self.board_score = 0;
        self.has_passed = 0;
        self.card_cnt = [13; 4];

        let mut m: u64 = 0;
        for c in self.cards.iter() {
            m |= c;
            println!("C 0x{:16x} count {}", c, c.count_ones());
        }
        let im = !(m | 0xFFF);
        println!("! 0x{:16x} M 0x{:16x} count {}", im, m, im.count_ones());
        // assert!(m == 0xFFFF_FFFF_FFFF_F000);

        // Which player to start
        if self.round == 1 {
            self.turn = self.cards.iter().position(|&x| x & 0x1000 != 0).unwrap() as i32;
        } else {
            let p = (self.last_action & 0x3) as i32;
            println!("Last action {:16x} P{}", self.last_action, p);
            self.turn = p;
        }
    }
    pub fn play(&mut self, player: i32, hand: u64) -> Result<(), SrvGameError> {
        if player != self.turn {
            return Err(SrvGameError::NotPlayersTurn);
        }

        let p: usize = player as usize & 0x3;
        let mut pc = self.cards[p];
        let illegal_cards = (pc ^ hand) & hand;

        if illegal_cards != 0 {
            return Err(SrvGameError::PlayerPlayedIllegalCard(illegal_cards));
        }

        let score = rules::score_hand(hand);
        if score == 0 {
            return Err(SrvGameError::InvalidHand);
        }

        if self.board_score != 0 && score <= self.board_score {
            return Err(SrvGameError::InvalidHand);
        }

        self.board_score = score;
        self.last_action = hand | (p as u64);

        pc ^= hand;
        // pc &= !hand;

        self.cards[p] = pc;

        let cnt = hand.count_ones();
        self.card_cnt[p] -= cnt as u8;
        if self.card_cnt[p] == 0 {
            println!("No more cards!");
            self.turn = -1;
            return Ok(());
        }

        // if pc.count_ones() == 0 {
        //     println!("No more cards!");
        //     self.turn = -1;
        //     return Ok(());
        // }

        self.next_player();

        Ok(())
    }
    pub fn pass(&mut self, player: i32) -> Result<(), SrvGameError> {
        if player != self.turn {
            return Err(SrvGameError::NotPlayersTurn);
        }
        let b = 1 << player;
        if b & self.has_passed != 0 {
            return Err(SrvGameError::AllreadyPassed);
        }

        self.has_passed |= b;

        self.next_player();

        Ok(())
    }

    fn next_player(&mut self) {
        let mut next = self.turn;

        if self.board_score == 0x23f || self.board_score == 0x13f || self.board_score == 0x33f {
            println!(
                "Play 2s which is the highest card bs {:3x}",
                self.board_score
            );
            self.board_score = 0;
            self.has_passed = 0;
            return;
        }

        for _ in 0..3 {
            next = (next + 1) & 0x3;

            let b = 1 << next;
            println!(
                "    TURN {} NEXT {} HP {:x} B {:x} SKIP {:x}",
                self.turn,
                next,
                self.has_passed,
                b,
                self.has_passed & b
            );

            if self.has_passed & b == 0 {
                break;
            }
        }
        if self.has_passed.count_ones() == 3 {
            // everyone has passed.
            self.board_score = 0;
            self.has_passed = 0;
            println!("\tEveryone has passed bs {:3x}", self.board_score);
        }

        self.turn = next;
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rules_sizes() {
        assert!(rules::is_valid_hand(0) == false);
        assert!(rules::is_valid_hand(0x1001) == false);
        assert!(rules::is_valid_hand(0b1) == false, "1 invalid card");
        assert!(rules::is_valid_hand(0b1 << 12) == true);
        assert!(rules::is_valid_hand(0b11 << 12) == true);
        assert!(rules::is_valid_hand(0b111 << 12) == true);
        assert!(rules::is_valid_hand(0b1111 << 12) == false, "4 cards");
        assert!(rules::is_valid_hand(0b11111 << 12) == true);
        assert!(rules::is_valid_hand(0b111111 << 12) == false, "6 cards");
    }
    #[test]
    fn rules_board_hand_new_turn() {
        assert!(rules::beter_hand(0, 0b1 << 12));
        assert!(rules::beter_hand(0, 0b11 << 12));
        assert!(rules::beter_hand(0, 0b111 << 12));
        assert!(rules::beter_hand(0, 0b1111 << 12) == false);
        assert!(rules::beter_hand(0, 0b11111 << 12));
    }
    #[test]
    fn rules_board_hand_one_pair() {
        assert!(rules::beter_hand(0b1 << 12, 0b1 << 12));
        assert!(rules::beter_hand(0b1 << 12, 0b11 << 12) == false);
        assert!(rules::beter_hand(0b1 << 12, 0b111 << 12) == false);
        assert!(rules::beter_hand(0b1 << 12, 0b11111 << 12) == false);
    }
    #[test]
    fn rules_board_hand_two_pair() {
        assert!(rules::beter_hand(0b11 << 12, 0b1 << 12) == false);
        assert!(rules::beter_hand(0b11 << 12, 0b11 << 12));
        assert!(rules::beter_hand(0b11 << 12, 0b111 << 12) == false);
        assert!(rules::beter_hand(0b11 << 12, 0b11111 << 12) == false);
    }
    #[test]
    fn rules_board_hand_three_of_kind() {
        assert!(rules::beter_hand(0b111 << 12, 0b1 << 12) == false);
        assert!(rules::beter_hand(0b111 << 12, 0b11 << 12) == false);
        assert!(rules::beter_hand(0b111 << 12, 0b111 << 12) == true);
        assert!(rules::beter_hand(0b111 << 12, 0b11111 << 12) == false);
    }
    #[test]
    fn rules_board_hand_fivecards() {
        assert!(rules::beter_hand(0b11111 << 12, 0b1 << 12) == false);
        assert!(rules::beter_hand(0b11111 << 12, 0b11 << 12) == false);
        assert!(rules::beter_hand(0b11111 << 12, 0b111 << 12) == false);
        assert!(rules::beter_hand(0b11111 << 12, 0b11111 << 12));
    }
    #[test]
    fn b_rules_score_hand() {
        // ONE
        assert!(rules::score_hand(0x0000_0000_0000_1000) == cards::Kind::ONE | cards::Kind::LOWEST);
        assert!(
            rules::score_hand(0x8000_0000_0000_0000) == cards::Kind::ONE | cards::Kind::HIGHEST
        );

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
        assert!(
            rules::score_hand(0b0001_1111_0000 << 12) == cards::Kind::QUADS | cards::Rank::FOUR
        );
        assert!(
            rules::score_hand(0b0000_1111_1000 << 12) == cards::Kind::QUADS | cards::Rank::FOUR
        );
        assert!(rules::score_hand(0b0001_1111_0000 << 52) == cards::Kind::QUADS | cards::Rank::ACE);
        assert!(rules::score_hand(0b0000_1111_1000 << 52) == cards::Kind::QUADS | cards::Rank::ACE);
        assert!(rules::score_hand(0b1111_0000_1000 << 52) == cards::Kind::QUADS | cards::Rank::TWO);
        assert!(rules::score_hand(0b1111_0001_1000 << 52) == 0);
        assert!(rules::score_hand(0b1111_0000_1001 << 52) == 0);

        // FULL HOUSE
        assert!(
            rules::score_hand(0b0011_1011_0000 << 12) == cards::Kind::FULLHOUSE | cards::Rank::FOUR
        );
        assert!(
            rules::score_hand(0b0000_1101_1001 << 12) == cards::Kind::FULLHOUSE | cards::Rank::FOUR
        );
        assert!(
            rules::score_hand(0b0000_1011_0110 << 12) == cards::Kind::FULLHOUSE | cards::Rank::FOUR
        );
        assert!(
            rules::score_hand(0b1110_1001_0000 << 52) == cards::Kind::FULLHOUSE | cards::Rank::TWO
        );
        assert!(
            rules::score_hand(0b0000_0111_1001 << 52) == cards::Kind::FULLHOUSE | cards::Rank::ACE
        );
        assert!(
            rules::score_hand(0b0000_1101_0110 << 52) == cards::Kind::FULLHOUSE | cards::Rank::ACE
        );

        // STRAIGHT
        assert!(rules::score_hand(0x0002_1111 << 12) == cards::Kind::STRAIGHT | 0x1d);
        assert!(rules::score_hand(0x0002_2221 << 12) == cards::Kind::STRAIGHT | 0x1d);
        assert!(rules::score_hand(0x0000_0002_2221 << 12) == cards::Kind::STRAIGHT | 0x1d);
        // 23456
        assert!(
            rules::score_hand(0x8000_0000_0111_1000)
                == cards::Kind::STRAIGHT | cards::Kind::HIGHEST | 0x40
        );
        // A2345
        assert!(
            rules::score_hand(0x8200_0000_0011_1000)
                == cards::Kind::STRAIGHT | cards::Kind::HIGHEST | 0x80
        );

        // FLUSH
        assert!(rules::score_hand(0x0011_1101 << 12) == cards::Kind::FLUSH | 32);
        assert!(
            rules::score_hand(0x8800_0000_0808_8000) == cards::Kind::FLUSH | cards::Kind::HIGHEST
        );

        // STRAIGHT FLUSH
        assert!(rules::score_hand(0x0001_1111 << 12) == cards::Kind::STRAIGHTFLUSH | 0x1c);
        assert!(rules::score_hand(0x1111_1000_0000_0000) == cards::Kind::STRAIGHTFLUSH | 0x3c);
        assert!(
            rules::score_hand(0x8888_8000_0000_0000)
                == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST
        );
        // 23456
        assert!(
            rules::score_hand(0x8000_0000_0888_8000)
                == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST | 0x40
        );
        assert!(
            rules::score_hand(0x1000_0000_0111_1000) == cards::Kind::STRAIGHTFLUSH | 0x3c | 0x40
        );
        // A2345
        assert!(
            rules::score_hand(0x8800_0000_0088_8000)
                == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST | 0x80
        );
        assert!(
            rules::score_hand(0x1100_0000_0011_1000) == cards::Kind::STRAIGHTFLUSH | 0x3c | 0x80
        );

        // BARBAGE
        assert!(rules::score_hand(0x0001_0311 << 12) == 0);
    }
    #[test]
    fn c_deal_hand() {
        // No cards generated
        assert!(deck::deal() != [0, 0, 0, 0]);
        // Detect shuffle is did not work at all.
        assert!(
            deck::deal()
                != [
                    0x1111_1111_1111_1000,
                    0x2222_2222_2222_2000,
                    0x4444_4444_4444_4000,
                    0x8888_8888_8888_8000
                ]
        );
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

    #[test]
    fn trail_server() {
        let trail: &[u64] = &[
            0x0d00854004174430, // Deal
            0x0000000000001003,
            0x0000000000004010,
            0x0000000000008021,
            0x0000000020000032,
            0x0000000800000003,
            0x0000004000000010,
            0x0000200000000021,
            0x2000000000000032,
            0x2000000000000103,
            0x2000000000000110,
            0x2000000000000121,
            0x0000000100000032,
            0x0000400000000003,
            0x0000800000000010,
            0x0002000000000021,
            0x1000000000000032,
            0x1000000000000103,
            0x1000000000000110,
            0x1000000000000121,
            0x000000000a000032,
            0x00a0000000000003,
            0x0500000000000010,
            0x0500000000000121,
            0xc000000000000022,
            0x000c000000000032,
            0x000c000000000103,
            0x000c000000000110,
            0x000c000000000121,
            0x0050000000000032,
            0x0050000000000103,
            0x0050000000000110,
            0x0050000000000121,
            0x0000100000000072,
            0x0000000001000800,
            0x0000000001001800,
            0x0000000001101800,
            0x0000000001111800,
            0xc00ac102a1022420,
            0x0000000000090032,
            0x0000000000090103,
            0x0000c00000000010,
            0x0c00000000000021,
            0x0c00000000000102,
            0x0c00000000000110,
            0x0000000000008021,
            0x0000000000200032,
            0x0000000000800003,
            0x0000000000800110,
            0x0000000010000021,
            0x0000000800000032,
            0x0000008000000013,
            0x0000080000000021,
            0x0080000000000032,
            0x0200000000000013,
            0x1000000000000021,
            0x1000000000000132,
            0x2000000000000013,
            0x2000000000000131,
            0x0000000444440003,
            0x0000000444440110,
            0x0000000444440121,
            0x0000000444440132,
            0x0000000000005003,
            0xc000000000000000,
            0x0002000220022010,
            0x0002000220022121,
            0x0011001100100032,
            0x0011001100100103,
            0x0011001100100120,
            0x0000060000000032,
            0x0000060000000103,
            0x0000060000000110,
            0x0000300000000021,
            0x0000300000000112,
            0x0000006000000021,
            0x0000006000000132,
            0x0000006000000103,
            0x0000006000000110,
            0x000000000a000021,
            0x000000000a000132,
            0x000000000a000103,
            0x000000000a000110,
            0x0040000000000071,
            0x0000000001000800,
            0x0000000001001800,
            0x0000000001011800,
            0x0000000001111800,
            0x080e0690c0051410,
            0x0000000000200021,
            0x0000000000400032,
            0x0000000000800003,
            0x0000001000000010,
            0x0000080000000021,
            0x0000200000000032,
            0x0001000000000003,
            0x0800000000000010,
            0x0800000000000121,
            0x8000000000000022,
            0x1200000000128032,
            0x0000000e06000003,
            0x000e000000050010,
            0x000e000000050121,
            0x000e000000050132,
            0x000e000000050103,
            0x0000000000001010,
            0x0000000020000021,
            0x0000000100000032,
            0x0000002000000003,
            0x0000008000000010,
            0x0040000000000021,
            0x0040000000000132,
            0x0040000000000103,
            0x0040000000000110,
            0x0000d00000006021,
            0x0000d00000006132,
            0x0000d00000006103,
            0x0000d00000006110,
            0x6000000000000021,
            0x6000000000000132,
            0x6000000000000103,
            0x6000000000000110,
            0x0000000009000071,
            0x0000000001000800,
            0x0000000001010800,
            0x0000000001011800,
            0x0000000001111800,
            0x1019068819840410,
            0x0000000000080021,
            0x0000080000000032,
            0x0000400000000003,
            0x0010000000000010,
            0x0040000000000021,
            0x4000000000000032,
            0x4000000000000103,
            0x4000000000000110,
            0x8000000000000011,
            0x0e00005000000021,
            0x0e00005000000132,
            0x0e00005000000103,
            0x0e00005000000110,
            0x0002002220200071,
            0x0000000001000800,
            0x0000000001100800,
            0x0000000001101800,
            0x0000000001111800,
            0x061900011c20b410,
            0x1000000002184021,
            0x00007000c0000032,
            0x00007000c0000103,
            0x00007000c0000110,
            0x00007000c0000121,
            0x0000000001000032,
            0x0000008000000003,
            0x0010000000000010,
            0x0020000000000021,
            0x0080000000000032,
            0x0800000000000003,
            0x0800000000000110,
            0x0800000000000121,
            0x0800000000000132,
            0x0000000000070003,
            0x0000000000070110,
            0x00000e0000000021,
            0x00000e0000000132,
            0x00000e0000000113,
            0x0000000000800021,
            0x0000000400000032,
            0x0000010000000003,
            0x0008000000000010,
            0x0008000000000121,
            0x0100000000000032,
            0x2000000000000003,
            0x2000000000000120,
            0x2000000000000132,
            0x0000000000400003,
            0x0000000010000010,
            0x0000000800000021,
            0x0004000000000032,
            0x0040000000000003,
            0x0040000000000110,
            0x0040000000000121,
            0x8000000000000022,
            0x0000006000000072,
            0x0000000001000800,
            0x0000000001100800,
            0x0000000001101800,
            0x0000000001111800,
            0x0c503210091a0420,
            0x0000000000010032,
            0x0000000002000003,
            0x0000000008000010,
            0x0000000010000021,
            0x0000000020000032,
            0x0000000080000003,
            0x0000020000000010,
            0x4000000000000021,
            0x4000000000000132,
            0x4000000000000103,
            0x4000000000000110,
            0x0000000000001021,
            0x0000000000800032,
            0x0000000800000003,
            0x0000200000000010,
            0x0001000000000021,
            0x0004000000000032,
            0x0004000000000103,
            0x0040000000000010,
            0x0080000000000021,
            0x1000000000000002,
            0x1000000000000110,
            0x8000000000000011,
            0x0000000000040021,
            0x0000000100000032,
            0x0002000000000003,
            0x0800000000000010,
            0x0800000000000121,
            0x2000000000000032,
            0x2000000000000103,
            0x2000000000000120,
            0x000000e000006032,
            0x000000e000006103,
            0x000000e000006110,
            0x000000e000006121,
            0x0020000000000072,
            0x0000000000010800,
            0x0000000001010800,
            0x0000000001011800,
            0x0000000001111800,
            0xda24004d80800420,
            0x0000000000001032,
            0x0000000000008003,
            0x0000000000800010,
            0x0000000008000021,
            0x0000008000000032,
            0x0002000000000003,
            0x0004000000000010,
            0x0400000000000021,
            0x0400000000000132,
            0x0400000000000103,
            0x4000000000000010,
            0x4000000000000101,
            0x0000000080000010,
            0x0000080000000021,
            0x0000100000000032,
            0x0000200000000003,
            0x0020000000000010,
            0x0020000000000121,
            0x0100000000000032,
            0x0100000000000103,
            0x1000000000000020,
            0x2000000000000002,
            0x8000000000000000,
            0x0a00000d00000010,
            0x0a00000d00000121,
            0x0a00000d00000132,
            0x0a00000d00000103,
            0x0000004000000070,
            0x0000000000010800,
            0x0000000001010800,
            0x0000000001011800,
            0x0000000001111800,
            0x82c4120900590400,
            0x0000000000090010,
            0x0000000600000021,
            0x0000006000000032,
            0x0000009000000003,
            0x00c0000000000010,
            0x0c00000000000021,
            0x0c00000000000132,
            0x0c00000000000103,
            0x0c00000000000110,
            0x0000000000006021,
            0x0000000000009032,
            0x000000000a000003,
            0x0000000900000010,
            0x0000000900000121,
            0x0000000900000132,
            0x0000050000000003,
            0x0000050000000130,
            0x00000000a0000003,
            0x00000000a0000110,
            0x00000000a0000121,
            0x00000000a0000132,
            0x0000a00000000003,
            0x0000a00000000110,
            0x0000a00000000121,
            0x0000a00000000132,
            0x0000000000800003,
            0x0000020000000010,
            0x0000400000000021,
            0x0008000000000032,
            0x4000000000000003,
            0x8000000000000000,
            0x0000000000500010,
            0x0000000000500121,
            0x0000000005000032,
            0x0000000005000103,
            0x0000000005000120,
            0x2000000000000032,
            0x2000000000000103,
            0x2000000000000110,
            0x2000000000000121,
            0x0100000000000032,
            0x0100000000000103,
            0x0200000000000010,
            0x1000000000000021,
            0x1000000000000102,
            0x1000000000000110,
            0x0000000000020021,
            0x0020000000000032,
            0x0020000000000103,
            0x0020000000000110,
            0x0020000000000121,
            0x0000080000000032,
            0x0002000000000073,
        ];
        let cards: &[u64] = &[
            // First game
            0x0d00854004174000,
            0x2200000008000 | 0x201_0ab6_0100_0000,
            0xf05c10012a000000,
            0xa0400800001000 | 0x000_0000_d0e8_2000,
            // Second game
            0xC00AC102A1022000,
            0x1c4038601a008000,
            0x91061900390000 | 0x0100_0000_0000_0000,
            0x2200008444c45000 | 0x0024_0000_0000_0000,
            // Third
            0x080e0690c0051000,
            0x6040d80029206000,
            0x9200200100528000 | 0x40_1008_0000,
            0x1002e06800000 | 0x5b0_0100_0000_0000,
            // Fourth
            0x1019068819840000,
            0x8e42007220280000,
            0x4000080000000000,
            0x400000000000,
            // 5
            0x61900011c20b000,
            0x10200e0802984000,
            0x81847064c1000000,
            0x2840018000470000,
            // 6
            0xc503210091a0000,
            0xc081000010041000,
            0x302400e120816000,
            0x2000882000000,
            // 7
            0xda24004d80800000,
            0x400080008000000,
            0x2100108000001000,
            0x2200000008000,
            // 8
            0x82c4120900590000,
            0x1c00400600026000,
            0x2128086005009000,
            0x4002a590aa800000,
        ];

        // Create trail array
        // let mut c: [u64; 4] = [0, 0, 0, 0];
        // for t in trail.iter() {
        //     let a = t & 0xF00;
        //     if a == 0 {
        //         let p = *t & 3;
        //         c[p as usize] |= *t & 0xFFFF_FFFF_FFFF_F000;
        //     }
        //     if t & 0x70 == 0x70 {
        //         let m = 0xFFFF_FFFF_FFFF_F000 ^ c[0] ^ c[1] ^ c[2] ^ c[3];
        //         println!("0x{:x}, 0x{:x}, 0x{:x}, 0x{:x},", c[0], c[1], c[2], c[3],);
        //         c = [0, 0, 0, 0];
        //     }
        // }

        let mut gs = SrvGameState::new(8);
        let mut cp: usize = 0;

        gs.deal(Some(&cards[0..4]));

        assert_eq!(gs.turn, 3);
        assert_eq!(gs.round, 1);
        assert_eq!(gs.rounds, 8);

        for play in trail.iter() {
            let action = *play as i32 & 0xF00;
            let player = ((*play as i32 & 0x7) << 29) >> 29;
            let toact = ((*play as i32 & 0x70) << 25) >> 29;

            let mut error: Result<(), SrvGameError> = Ok(());
            let hand: u64 = play & 0xFFFF_FFFF_FFFF_F000;

            match action {
                0x800 => {
                    println!("UPDATE {:16x}", play);
                    if *play == 0x111_1800 {
                        cp += 4;
                        gs.deal(Some(&cards[cp..cp + 4]));
                        println!("++ Start new game, round {}/{}", gs.round, gs.rounds);
                    }
                }
                0x000 => {
                    print!(
                        "++ PLAY: player {} hand {:16x} card {:16x} - ",
                        player, hand, gs.cards[player as usize]
                    );
                    error = gs.play(player, hand);
                    if error.is_ok() {
                        let c = gs.cards[player as usize];
                        println!("card {:16x} c&h {:16x} p {:16x}", c, c & hand, play);
                        assert!(c & hand == 0);
                        assert_eq!(gs.turn, toact);
                    } else {
                        println!("error");
                    }
                }
                0x100 => {
                    println!("++ PASS: player {}", player);
                    error = gs.pass(player);
                }
                0x400 => {
                    println!("++ DEAL: {:16x}", play);
                    // Match hand
                    assert_eq!(hand, gs.cards[player as usize]);
                    // turn and next user have to match
                    assert_eq!(toact, gs.turn);
                }
                _ => println!("Unknown action {}", action),
            }

            println!(
                " AFTER: BS{:4x} HP{:4x} T{}",
                gs.board_score, gs.has_passed, gs.turn
            );

            if let Err(e) = error {
                println!("Error with hand {:?}", e);
                match e {
                    SrvGameError::PlayerPlayedIllegalCard(hand) => println!(
                        "PLAY: hand {:16x} card {:16x}",
                        hand, gs.cards[player as usize]
                    ),
                    SrvGameError::NotPlayersTurn => println!("Turn {} player {}", gs.turn, player),
                    _ => print!(""),
                }
            }
        }
    }
}
