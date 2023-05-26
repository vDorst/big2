use crate::network::legacy as network;

use self::cards::ScoreKind;

pub const RANKS: [u8; 13] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

pub type Cards = u64;

pub mod deck {
    use super::{deck, Cards};
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    pub const NUMBER_OF_CARDS: u8 = 52;
    pub const START_BIT: u8 = 12;

    #[must_use]
    pub fn deal() -> [Cards; 4] {
        // Create and shulle deck of cards
        let deck = {
            let mut deck: Vec<u8> = (0..deck::NUMBER_OF_CARDS)
                .map(|card_number| deck::START_BIT + card_number)
                .collect();

            // Randomize/shuffle the cards
            for _ in 0..256 {
                deck.shuffle(&mut thread_rng());
            }
            deck
        };
        deal_cards(&deck)
    }
    pub(crate) fn deal_cards(cards: &[u8]) -> [Cards; 4] {
        let players_hand: [Cards; 4] = cards
            .chunks_exact(13)
            .map(|v| {
                let mut card = 0;
                for &d in v {
                    card |= 1 << Cards::from(d);
                }
                card
            })
            .collect::<Vec<Cards>>()
            .try_into()
            .unwrap();
        assert!(
            (players_hand[0] | players_hand[1] | players_hand[2] | players_hand[3])
                == 0xFFFF_FFFF_FFFF_F000u64
        );
        players_hand
    }
}

pub mod cards {
    #[non_exhaustive]
    pub struct Kind;
    #[non_exhaustive]
    pub struct Rank;

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum ScoreKind {
        Single(u8),
        Pair(u8),
        Set(u8),
        Straight(u8),
        Flush(u8),
        FullHouse(u8),
        Quads(u8),
        StraightFlush(u8),
    }

    impl PartialOrd for ScoreKind {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            match (self, other) {
                (ScoreKind::Single(a), ScoreKind::Single(b)) => Some(a.cmp(b)),
                (ScoreKind::Single(_), _) => None,
                (ScoreKind::Pair(a), ScoreKind::Pair(b)) => Some(a.cmp(b)),
                (ScoreKind::Pair(_), _) => None,
                (ScoreKind::Set(a), ScoreKind::Set(b)) => Some(a.cmp(b)),
                (ScoreKind::Set(_), _) => None,
                (ScoreKind::Straight(a), ScoreKind::Straight(b)) => Some(a.cmp(b)),
                (
                    ScoreKind::Straight(_),
                    ScoreKind::Flush(_)
                    | ScoreKind::FullHouse(_)
                    | ScoreKind::Quads(_)
                    | ScoreKind::StraightFlush(_),
                ) => Some(std::cmp::Ordering::Greater),
                (ScoreKind::Straight(_), _) => None,
                (ScoreKind::Flush(a), ScoreKind::Flush(b)) => Some(a.cmp(b)),
                (
                    ScoreKind::Flush(_),
                    ScoreKind::FullHouse(_) | ScoreKind::Quads(_) | ScoreKind::StraightFlush(_),
                ) => Some(std::cmp::Ordering::Greater),
                (ScoreKind::Flush(_), _) => None,
                (ScoreKind::FullHouse(a), ScoreKind::FullHouse(b)) => Some(a.cmp(b)),
                (ScoreKind::FullHouse(_), ScoreKind::Quads(_) | ScoreKind::StraightFlush(_)) => {
                    Some(std::cmp::Ordering::Greater)
                }
                (ScoreKind::FullHouse(_), _) => None,
                (ScoreKind::Quads(a), ScoreKind::Quads(b)) => Some(a.cmp(b)),
                (ScoreKind::Quads(_), ScoreKind::StraightFlush(_)) => {
                    Some(std::cmp::Ordering::Greater)
                }
                (ScoreKind::Quads(_), _) => None,
                (ScoreKind::StraightFlush(a), ScoreKind::StraightFlush(b)) => Some(a.cmp(b)),
                (ScoreKind::StraightFlush(_), _) => None,
            }
        }
    }

    impl Kind {
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
        pub const THREE: u8 = 3;
        pub const FOUR: u8 = 4;
        pub const FIVE: u8 = 5;
        pub const SIX: u8 = 6;
        pub const SEVEN: u8 = 7;
        pub const EIGTH: u8 = 8;
        pub const NINE: u8 = 9;
        pub const TEN: u8 = 10;
        pub const JACK: u8 = 11;
        pub const QUEEN: u8 = 12;
        pub const KING: u8 = 13;
        pub const ACE: u8 = 14;
        pub const TWO: u8 = 15;
    }

    #[must_use]
    pub fn has_rank(hand: u64, rank: u8) -> u64 {
        let mask = Kind::SUITMASK << (rank << 2);
        hand & mask
    }

    #[must_use]
    pub fn cnt_rank(hand: u64, rank: u8) -> u64 {
        u64::from(has_rank(hand, rank).count_ones())
    }

    #[must_use]
    pub fn card_selected(card: u64) -> u8 {
        u8::try_from(card.trailing_zeros()).expect("Should fit")
    }

    #[must_use]
    pub fn has_rank_idx(card: u64) -> u8 {
        card_selected(card) as u8 >> 2 
    }

    #[must_use]
    pub fn has_suit(card: u64) -> u64 {
        1 << (card_selected(card) & 0x3)
    }
}

pub mod rules {
    use super::{
        cards::{self, ScoreKind},
        RANKS,
    };

    #[allow(dead_code)]
    pub fn get_numbers(hand: u64) {
        let mut ranks: [u32; 16] = [0; 16];
        let mut straigth: u64 = 0;
        let mut tripps: u32 = 0;
        let mut quads: u32 = 0;
        let mut straigths: u32 = 0;
        let mut doubles: u32 = 0;

        for r in &RANKS {
            let idx: usize = (*r).into();
            ranks[idx] = cards::cnt_rank(hand, idx as u8) as u32;
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
            "R{ranks:x?} S{straigth:16b} {straigths:x} D{doubles:x} T{tripps:x} Q{quads:x} FH{fullhouse:x} FL{flushs:x}"
        );
    }
    #[must_use]
    pub fn is_valid_hand(hand: u64) -> bool {
        // Check cards range. Only the upper 52 bits are used.
        let ret: bool = hand.trailing_zeros() >= 12;

        // Check number of cards played. count = 1, 2, 3 or 5 is valid.
        let cardcount = hand.count_ones();
        ret && cardcount != 4 && cardcount < 6 && cardcount != 0
    }
    #[allow(dead_code)]
    #[must_use]
    pub fn beter_hand(board: u64, hand: u64) -> bool {
        if !is_valid_hand(hand) {
            return false;
        }

        let card_cnt_hand = hand.count_ones();
        let card_cnt_board = board.count_ones();

        // Board and hand count must match.
        // Board count 0 means new turn.
        !(card_cnt_board != 0 && card_cnt_board != card_cnt_hand)
    }

    #[must_use]
    pub fn higher_single_card(board: u64, hand: u64) -> u64 {
        let mask: u64 = u64::MAX.wrapping_shl(board.trailing_zeros());
        let higher_cards = hand & mask;
        let mask: u64 = 1u64.wrapping_shl(higher_cards.trailing_zeros());

        hand & mask
    }

    #[must_use]
    pub fn is_flush(hand: u64) -> bool {
        let mut mask: u64 = 0x1111_1111_1111_1000;
        for _ in 0..4 {
            if (hand & !mask) == 0 {
                return true;
            }
            mask <<= 1;
        }
        false
    }
    #[must_use]
    pub fn has_flush(hand: u64) -> u8 {
        let mut mask: u64 = 0x1111_1111_1111_1000;
        let mut flushs: u8 = 0;
        for _ in 0..4 {
            if (hand & mask).count_ones() >= 5 {
                flushs += 1;
            }
            mask <<= 1;
        }
        flushs
    }
    #[must_use]
    pub fn score_hand(hand: u64) -> Option<ScoreKind> {
        // Score:
        //  0xKNN = One, Pair and Straigth, Flush
        //    |++- highest card: bit nummer of the highest card
        //    +--- Kind: Kind::ONE or Kind::TWO

        //  0xK0R = Set, Quad, FullHouse
        //    ||+- Rank: Only the RANK. Because only one RANK of each can exists.
        //    |+-- Zero
        //    +--- Kind: Kind::QUADS or Kind::SET or Kind::FULLHOUSE

        if !is_valid_hand(hand) {
            return None;
        }

        let card_cnt_hand = hand.count_ones();

        // find the highest card and calc the rank.
        let highest_card = 63 - u8::try_from(hand.leading_zeros()).ok()?;
        let rank = highest_card >> 2;

        // Get the played suit of that rank.
        let suitmask = hand >> (rank << 2);
        // Count number of cards based on the suit
        let cnt = suitmask.count_ones();

        if card_cnt_hand <= 3 {
            // If cnt doesn't match the card_cnt then it is invalid hand.
            if cnt != card_cnt_hand {
                return None;
            }

            if card_cnt_hand == 1 {
                return Some(ScoreKind::Single(highest_card));
            }
            if card_cnt_hand == 2 {
                return Some(ScoreKind::Pair(highest_card));
            }
            if card_cnt_hand == 3 {
                return Some(ScoreKind::Set(rank));
            }

            return None;
        }

        let lowest_card = u8::try_from(hand.trailing_zeros()).ok()?;
        let low_rank = lowest_card >> 2;
        // Get the played suit of that rank.
        let low_suitmask = hand >> (low_rank << 2) & cards::Kind::SUITMASK;
        // Count number of cards based on the suit
        let low_cnt = low_suitmask.count_ones();

        // Quad
        if cnt == 4 {
            return Some(ScoreKind::Quads(rank as u8));
        }
        if low_cnt == 4 {
            return Some(ScoreKind::Quads(low_rank as u8));
        }

        // Full House
        if cnt == 3 && low_cnt == 2 {
            return Some(ScoreKind::FullHouse(rank as u8));
        }
        if cnt == 2 && low_cnt == 3 {
            return Some(ScoreKind::FullHouse(low_rank as u8));
        }

        // Flush
        let is_flush: bool = is_flush(hand);

        // Straigth detection
        let mut is_straight: bool = rank - low_rank == 4 || rank - low_rank == 12;

        if is_straight {
            let mut straigth_score: u8 = 0;
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
                    return Some(ScoreKind::StraightFlush(straigth_score as u8));
                }
                return Some(ScoreKind::Straight(straigth_score as u8));
            }
        }

        if !is_straight && is_flush {
            return Some(ScoreKind::Flush(highest_card as u8));
        }

        None
    }
}

pub struct GameState {
    pub sm: network::StateMessage,
    pub srn: std::io::Stdout,
    pub board: Cards,
    pub board_score: Option<ScoreKind>,
    pub cards_selected: u64,
    pub auto_pass: bool,
    pub i_am_ready: bool,
    pub is_valid_hand: bool,
    pub hand_score: Option<ScoreKind>,
}

pub struct SrvGameState {
    pub prev_action: u64,
    pub last_action: u64,
    pub board_score: Option<ScoreKind>,
    pub has_passed: u8,
    pub turn: i32,
    pub round: u8,
    pub rounds: u8,
    pub cards: [Cards; 4],
    pub played_cards: Cards,
    pub score: [i16; 4],
    pub card_cnt: [u8; 4],
}

#[derive(Debug)]
pub enum SrvGameError {
    NotPlayersTurn,
    PlayerPlayedIllegalCard(u64),
    InvalidHand,
    AllreadyPassed,
}

impl SrvGameState {
    #[must_use]
    pub fn new(rounds: u8) -> Self {
        SrvGameState {
            prev_action: 0,
            last_action: 0,
            board_score: None,
            has_passed: 0,
            turn: -1,
            round: 0,
            rounds,
            cards: [0; 4],
            played_cards: 0,
            score: [0; 4],
            card_cnt: [13; 4],
        }
    }
    pub fn deal(&mut self, cards: Option<&[u64; 4]>) {
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
        self.board_score = None;
        self.has_passed = 0;
        self.card_cnt = [13; 4];

        let mut m: u64 = 0;
        for c in &self.cards {
            m |= c;
            println!("C 0x{:16x} count {}", c, c.count_ones());
        }
        let im = !(m | 0xFFF);
        println!("! 0x{:16x} M 0x{:16x} count {}", im, m, im.count_ones());
        // assert!(m == 0xFFFF_FFFF_FFFF_F000);

        // Which player to start
        self.turn = if self.round == 1 {
            self.cards
                .iter()
                .position(|&x| x & 0x1000 != 0)
                .expect("Weard a use should start with 0x1000 card!") as i32
        } else {
            let p = self.last_action & 0x3;
            println!("Last action {:16x} P{}", self.last_action, p);
            p as i32
        };
    }
    pub fn play(&mut self, player: i32, hand: Cards) -> Result<(), SrvGameError> {
        if player != self.turn {
            return Err(SrvGameError::NotPlayersTurn);
        }

        let p: usize = player as usize & 0x3;
        let pc = self.cards[p];

        let illegal_cards = (pc ^ hand) & hand;
        if illegal_cards != 0 {
            return Err(SrvGameError::PlayerPlayedIllegalCard(illegal_cards));
        }

        let Some(score) = rules::score_hand(hand) else {
            return Err(SrvGameError::InvalidHand);
        };

        if let Some(board_score) = &self.board_score {
            if &score <= board_score {
                return Err(SrvGameError::InvalidHand);
            }
        }

        self.prev_action = self.last_action;
        self.last_action = hand | (p as Cards) | ((self.last_action & 0x3) << 2);

        self.board_score = Some(score);
        self.cards[p] ^= hand;

        let cnt = hand.count_ones();
        self.card_cnt[p] -= cnt as u8;

        if self.card_cnt[p] == 0 {
            self.calc_score();
            println!("No more cards! Score: {:?}", self.score);
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

        if let Some(board_score) = self.board_score {
            if board_score == ScoreKind::Single(0x3F)
                || board_score == ScoreKind::Pair(0x3F)
                || board_score == ScoreKind::Set(0x3F)
            {
                println!("Play 2s which is the highest card bs {:?}", board_score);
                self.board_score = None;
                self.has_passed = 0;
                return;
            }
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
            println!("\tEveryone has passed bs {:?}", self.board_score);
            self.board_score = None;
            self.has_passed = 0;
        }

        self.turn = next;
    }

    fn calc_score(&mut self) {
        let prev_player = self.prev_action as usize & 0x3;
        let curr_player = self.last_action as usize & 0x3;
        let hand = self.last_action & 0xFFFF_FFFF_FFFF_F000;

        // Assist!
        let assisted = prev_player != curr_player
            && matches!(self.board_score, Some(ScoreKind::Single(_)))
            && hand < self.cards[prev_player];

        if assisted {
            println!(
                "Assist! PP{} {:16x} CP{} {:16x}",
                prev_player, self.cards[prev_player], self.turn, hand,
            );
        }

        let mut total_score: i16 = 0;
        let delta_score: [i16; 4] = self
            .card_cnt
            .iter()
            .map(|&card_cnt| {
                let mut s = i16::from(card_cnt);
                if card_cnt == 13 {
                    s *= 3;
                } else if card_cnt > 9 {
                    s *= 2;
                };

                total_score += s;
                s
            })
            .collect::<Vec<i16>>()
            .try_into()
            .expect("Should fit");

        if assisted {
            self.score[prev_player] -= total_score;
        } else {
            self.score
                .iter_mut()
                .zip(delta_score)
                .for_each(|(score, delta_score)| *score -= delta_score);
        }
        self.score[self.turn as usize] += total_score;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rules_sizes() {
        assert!(!rules::is_valid_hand(0));
        assert!(!rules::is_valid_hand(0x1001));
        assert!(!rules::is_valid_hand(0b1), "1 invalid card");
        assert!(rules::is_valid_hand(0b1 << 12));
        assert!(rules::is_valid_hand(0b11 << 12));
        assert!(rules::is_valid_hand(0b111 << 12));
        assert!(!rules::is_valid_hand(0b1111 << 12), "4 cards");
        assert!(rules::is_valid_hand(0b11111 << 12));
        assert!(!rules::is_valid_hand(0b11_1111 << 12), "6 cards");
    }
    #[test]
    fn rules_board_hand_new_turn() {
        assert!(rules::beter_hand(0, 0b1 << 12));
        assert!(rules::beter_hand(0, 0b11 << 12));
        assert!(rules::beter_hand(0, 0b111 << 12));
        assert!(!rules::beter_hand(0, 0b1111 << 12));
        assert!(rules::beter_hand(0, 0b11111 << 12));
    }
    #[test]
    fn rules_board_hand_one_pair() {
        assert!(rules::beter_hand(0b1 << 12, 0b1 << 12));
        assert!(!rules::beter_hand(0b1 << 12, 0b11 << 12));
        assert!(!rules::beter_hand(0b1 << 12, 0b111 << 12));
        assert!(!rules::beter_hand(0b1 << 12, 0b11111 << 12));
    }
    #[test]
    fn rules_board_hand_two_pair() {
        assert!(!rules::beter_hand(0b11 << 12, 0b1 << 12));
        assert!(rules::beter_hand(0b11 << 12, 0b11 << 12));
        assert!(!rules::beter_hand(0b11 << 12, 0b111 << 12));
        assert!(!rules::beter_hand(0b11 << 12, 0b11111 << 12));
    }
    #[test]
    fn rules_board_hand_three_of_kind() {
        assert!(!rules::beter_hand(0b111 << 12, 0b1 << 12));
        assert!(!rules::beter_hand(0b111 << 12, 0b11 << 12));
        assert!(rules::beter_hand(0b111 << 12, 0b111 << 12));
        assert!(!rules::beter_hand(0b111 << 12, 0b11111 << 12));
    }
    #[test]
    fn rules_board_hand_fivecards() {
        assert!(!rules::beter_hand(0b11111 << 12, 0b1 << 12));
        assert!(!rules::beter_hand(0b11111 << 12, 0b11 << 12));
        assert!(!rules::beter_hand(0b11111 << 12, 0b111 << 12));
        assert!(rules::beter_hand(0b11111 << 12, 0b11111 << 12));
    }
    #[test]
    fn b_rules_score_hand() {
        // ONE
        assert_eq!(
            rules::score_hand(0x0000_0000_0000_1000),
            Some(ScoreKind::Single(cards::Kind::LOWEST as u8))
        );
        assert_eq!(
            rules::score_hand(0x8000_0000_0000_0000),
            Some(ScoreKind::Single(cards::Kind::HIGHEST as u8))
        );

        // PAIR
        assert_eq!(rules::score_hand(0b11 << 12), Some(ScoreKind::Pair(13)));
        // Select one 3 and one 4
        assert!(rules::score_hand(0b11000 << 12).is_none());
        assert!(rules::score_hand(0b11 << 12) < rules::score_hand(0b11 << 62));

        // SET
        assert_eq!(
            rules::score_hand(0b0111 << 12),
            Some(ScoreKind::Set(cards::Rank::THREE as u8))
        );
        assert_eq!(
            rules::score_hand(0b1110 << 12),
            Some(ScoreKind::Set(cards::Rank::THREE as u8))
        );
        assert_eq!(
            rules::score_hand(0b1101 << 12),
            Some(ScoreKind::Set(cards::Rank::THREE as u8))
        );
        assert_eq!(
            rules::score_hand(0b1011 << 12),
            Some(ScoreKind::Set(cards::Rank::THREE as u8))
        );
        assert!(rules::score_hand(0b11100 << 12).is_none());
        assert!(rules::score_hand(0b11 << 12) < rules::score_hand(0b11 << 13));

        // QUAD
        assert_eq!(
            rules::score_hand(0b0001_1111_0000 << 12),
            Some(ScoreKind::Quads(cards::Rank::FOUR as u8))
        );
        assert_eq!(
            rules::score_hand(0b0000_1111_1000 << 12),
            Some(ScoreKind::Quads(cards::Rank::FOUR as u8))
        );
        assert_eq!(
            rules::score_hand(0b0001_1111_0000 << 52),
            Some(ScoreKind::Quads(cards::Rank::ACE as u8))
        );
        assert_eq!(
            rules::score_hand(0b0000_1111_1000 << 52),
            Some(ScoreKind::Quads(cards::Rank::ACE as u8))
        );
        assert_eq!(
            rules::score_hand(0b1111_0000_1000 << 52),
            Some(ScoreKind::Quads(cards::Rank::TWO as u8))
        );
        assert!(rules::score_hand(0b1111_0001_1000 << 52).is_none());
        assert!(rules::score_hand(0b1111_0000_1001 << 52).is_none());

        // FULL HOUSE
        assert_eq!(
            rules::score_hand(0b0011_1011_0000 << 12),
            Some(ScoreKind::FullHouse(cards::Rank::FOUR as u8))
        );

        assert_eq!(
            rules::score_hand(0b0000_1101_1001 << 12),
            Some(ScoreKind::FullHouse(cards::Rank::FOUR as u8))
        );
        assert_eq!(
            rules::score_hand(0b0000_1011_0110 << 12),
            Some(ScoreKind::FullHouse(cards::Rank::FOUR as u8))
        );
        assert_eq!(
            rules::score_hand(0b1110_1001_0000 << 52),
            Some(ScoreKind::FullHouse(cards::Rank::TWO as u8))
        );
        assert_eq!(
            rules::score_hand(0b0000_0111_1001 << 52),
            Some(ScoreKind::FullHouse(cards::Rank::ACE as u8))
        );
        assert_eq!(
            rules::score_hand(0b0000_1101_0110 << 52),
            Some(ScoreKind::FullHouse(cards::Rank::ACE as u8))
        );

        // STRAIGHT
        assert_eq!(
            rules::score_hand(0x0002_1111 << 12),
            Some(ScoreKind::Straight(0x1d))
        );
        assert_eq!(
            rules::score_hand(0x0002_2221 << 12),
            Some(ScoreKind::Straight(0x1d))
        );
        assert_eq!(
            rules::score_hand(0x0000_0002_2221 << 12),
            Some(ScoreKind::Straight(0x1d))
        );
        // 23456
        assert_eq!(
            rules::score_hand(0x8000_0000_0111_1000),
            Some(ScoreKind::Straight(cards::Kind::HIGHEST as u8 | 0x40))
        );
        // A2345
        assert_eq!(
            rules::score_hand(0x8200_0000_0011_1000),
            Some(ScoreKind::Straight(cards::Kind::HIGHEST as u8 | 0x80))
        );

        // FLUSH
        assert_eq!(
            rules::score_hand(0x0011_1101 << 12),
            Some(ScoreKind::Flush(32))
        );
        assert_eq!(
            rules::score_hand(0x8800_0000_0808_8000),
            Some(ScoreKind::Flush(cards::Kind::HIGHEST as u8))
        );

        // STRAIGHT FLUSH
        assert_eq!(
            rules::score_hand(0x0001_1111 << 12),
            Some(ScoreKind::StraightFlush(0x1c))
        );
        assert_eq!(
            rules::score_hand(0x1111_1000_0000_0000),
            Some(ScoreKind::StraightFlush(0x3c))
        );
        assert_eq!(
            rules::score_hand(0x8888_8000_0000_0000),
            Some(ScoreKind::StraightFlush(cards::Kind::HIGHEST as u8))
        );
        // 23456
        assert_eq!(
            rules::score_hand(0x8000_0000_0888_8000),
            Some(ScoreKind::StraightFlush(cards::Kind::HIGHEST as u8 | 0x40))
        );
        assert_eq!(
            rules::score_hand(0x1000_0000_0111_1000),
            Some(ScoreKind::StraightFlush(0x3c | 0x40))
        );
        // A2345
        assert_eq!(
            rules::score_hand(0x8800_0000_0088_8000),
            Some(ScoreKind::StraightFlush(cards::Kind::HIGHEST as u8 | 0x80))
        );
        assert_eq!(
            rules::score_hand(0x1100_0000_0011_1000),
            Some(ScoreKind::StraightFlush(0x3c | 0x80))
        );

        // GARBAGE
        assert!(rules::score_hand(0x0001_0311 << 12).is_none());
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
    fn assist_test() {
        let mut gs = SrvGameState::new(1);
        gs.deal(Some(&[
            0x1111_1111_1111_1000,
            0x2222_2222_2222_2000,
            0x4444_4444_4444_4000,
            0x8888_8888_8888_8000,
        ]));
        assert_eq!(gs.turn, 0);

        // reduct cards
        gs.cards = [0x24000, 0x8000, 0x2000, 0x1000];
        gs.card_cnt = [2, 1, 1, 1];

        assert!(gs.play(0, 0x4000).is_ok());
        assert!(gs.play(1, 0x8000).is_ok());
        assert_eq!(gs.score, [-3, 3, 0, 0]);
    }
    #[test]
    fn non_assist_test() {
        let mut gs = SrvGameState::new(1);
        gs.deal(Some(&[
            0x1111_1111_1111_1000,
            0x2222_2222_2222_2000,
            0x4444_4444_4444_4000,
            0x8888_8888_8888_8000,
        ]));
        assert_eq!(gs.turn, 0);

        // reduct cards
        gs.cards = [0x5000, 0x8000, 0x2000, 0x40000];
        gs.card_cnt = [2, 1, 1, 1];

        assert!(gs.play(0, 0x4000).is_ok());
        assert!(gs.play(1, 0x8000).is_ok());
        assert_eq!(gs.score, [-1, 3, -1, -1]);
    }
    #[test]
    fn score_multiply_test() {
        let mut gs = SrvGameState::new(1);
        gs.deal(Some(&[
            0x1111_1111_1111_1000,
            0x2222_2222_2222_2000,
            0x8444_4444_4444_4000,
            0x4888_8888_8888_8000,
        ]));
        assert_eq!(gs.turn, 0);

        // reduct cards
        gs.cards[2] = 0x8000_0000_0000_0000;
        gs.card_cnt[2] = 1;

        match gs.play(0, 0x1000) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
        match gs.play(1, 0x2000) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
        match gs.play(2, 0x8000_0000_0000_0000) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
        assert_eq!(gs.score, [-24, -24, 87, -39]);
    }

    #[test]
    fn better_single_card() {
        let board: u64 = 0x0_1000;
        let my_hand: u64 = 0x1_2000;
        let play = rules::higher_single_card(board, my_hand);
        assert_eq!(play, 0x2000);

        let board: u64 = 0x2000;
        let my_hand: u64 = 0x1_1000;
        let play = rules::higher_single_card(board, my_hand);
        assert_eq!(play, 0x1_0000);

        let board: u64 = 0x8000_0000_0000_0000;
        let play = rules::higher_single_card(board, my_hand);
        assert_eq!(play, 0);

        let board: u64 = 0x4000_0000_0000_0000;
        let my_hand: u64 = 0x8000_0000_0000_0000;
        let play = rules::higher_single_card(board, my_hand);
        assert_eq!(play, 0x8000_0000_0000_0000);

        let board: u64 = 0x0;
        let my_hand: u64 = 0xFFF8_0000_0000_0000;
        let play = rules::higher_single_card(board, my_hand);
        assert_eq!(play, 0x8_0000_0000_0000);
    }

    #[test]
    fn deal_test() {
        use super::deck;

        let deck: Vec<u8> = (0..deck::NUMBER_OF_CARDS)
            .map(|card_number| deck::START_BIT + card_number)
            .collect();

        let cards = deck::deal_cards(&deck);

        assert_eq!(
            &cards,
            &[
                0x0000_0000_01FF_F000,
                0x0000_003F_FE00_0000,
                0x0007_FFC0_0000_0000,
                0xFFF8_0000_0000_0000,
            ]
        );
    }
}
