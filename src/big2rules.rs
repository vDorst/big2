use std::cmp::Ordering;

use crate::network::legacy as network;

use self::cards::{CardNum, CardRank, Cards, ScoreKind};

pub const RANKS: [u8; 13] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

pub mod deck {
    use super::{cards::Cards, deck};
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
                let mut card = Cards(0);
                for &d in v {
                    card.set_bit(d);
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
    use core::fmt;
    use std::{
        cmp::Ordering,
        ops::{Add, BitAnd, BitOr, BitOrAssign, BitXor, BitXorAssign, Sub},
    };

    #[derive(Debug, Eq, PartialEq)]
    pub enum ParseCardsError {
        InvalidLowerBits,
        InvalidBoard,
        InvalidHand,
        IlligalHand,
        InvalidInput,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Default)]
    pub struct Cards(pub u64);

    impl fmt::Debug for Cards {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Cards(0x{:08X})", &self.0)
        }
    }

    impl Cards {
        #[must_use]
        pub fn from(cards: u64) -> Self {
            Self(cards)
        }
        pub fn set_bit(&mut self, bit: u8) {
            self.0 |= 1 << bit;
        }

        #[must_use]
        pub fn has_lowest_card(&self) -> bool {
            self.0 & 0x1000 != 0
        }

        // pub fn contains(&self, rhs: Cards) -> bool {
        //     self.0 & rhs.0 == rhs.0;
        // }

        #[must_use]
        pub fn count_ones(&self) -> u32 {
            self.0.count_ones()
        }
    }

    // impl Into<u64> for Cards {
    //     type Error = ParseIntError;

    //     fn try_into(self) -> Result<u64, Self::Error> {
    //         let val = self;
    //         if val & 0xFFFF_FFFF_FFFF_F000 != val {
    //             return Err(Self::Error);
    //         }
    //         Ok(Self(val))
    //     }
    // }

    impl TryFrom<u64> for Cards {
        type Error = ParseCardsError;

        fn try_from(value: u64) -> Result<Self, Self::Error> {
            if value & 0xFFFF_FFFF_FFFF_F000 != value {
                return Err(ParseCardsError::InvalidLowerBits);
            }
            Ok(Self(value))
        }
    }

    impl BitOr for Cards {
        type Output = u64;

        fn bitor(self, rhs: Self) -> Self::Output {
            self.0 | rhs.0
        }
    }

    impl BitOr<u64> for Cards {
        type Output = Cards;

        fn bitor(self, rhs: u64) -> Self::Output {
            Cards(self.0 | rhs)
        }
    }

    impl BitOrAssign<u64> for Cards {
        fn bitor_assign(&mut self, rhs: u64) {
            self.0 |= rhs;
        }
    }

    impl BitOr<Cards> for u64 {
        type Output = u64;

        fn bitor(self, rhs: Cards) -> Self::Output {
            self | rhs.0
        }
    }

    impl BitXorAssign for Cards {
        fn bitxor_assign(&mut self, rhs: Self) {
            self.0 = self.0.bitxor(rhs.0);
        }
    }

    impl BitAnd<Cards> for u64 {
        type Output = u64;

        fn bitand(self, rhs: Cards) -> Self::Output {
            self & rhs.0
        }
    }

    impl BitAnd for Cards {
        type Output = u64;

        fn bitand(self, rhs: Cards) -> Self::Output {
            self.0 & rhs.0
        }
    }

    impl fmt::LowerHex for Cards {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let val = self.0;

            fmt::LowerHex::fmt(&val, f)
        }
    }

    impl fmt::UpperHex for Cards {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let val = self.0;

            fmt::UpperHex::fmt(&val, f)
        }
    }

    #[non_exhaustive]
    pub struct Kind;
    #[non_exhaustive]
    pub struct Rank;

    #[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
    #[non_exhaustive]
    pub struct CardNum(u8);

    impl CardNum {
        #[must_use]
        #[inline]
        pub fn highcard(cards: Cards) -> Option<Self> {
            let cardnum = 63 - u8::try_from(cards.0.leading_zeros()).ok()?;
            Self::try_from(cardnum)
        }

        #[must_use]
        #[inline]
        pub fn lowcard(cards: Cards) -> Option<Self> {
            let cardnum = u8::try_from(cards.0.trailing_zeros()).ok()?;
            Self::try_from(cardnum)
        }

        #[must_use]
        #[inline]
        pub fn try_from(val: u8) -> Option<Self> {
            if (12..12 + super::deck::NUMBER_OF_CARDS).contains(&val) {
                Some(Self(val))
            } else {
                None
            }
        }

        pub const HIGHCARD: CardNum = Self(63);
        pub const LOWCARD: CardNum = Self(12);

        #[must_use]
        #[inline]
        pub fn rank(&self) -> CardRank {
            CardRank::from(self.0 >> 2)
        }
        #[must_use]
        #[inline]
        pub fn suit(&self) -> CardSuit {
            CardSuit::from(self.0 & 0x3)
        }

        #[must_use]
        #[inline]
        pub fn set_straight_23456(self) -> Self {
            Self(self.0 | 0x40)
        }

        #[allow(non_snake_case)]
        #[must_use]
        #[inline]
        pub fn set_straight_A2345(self) -> Self {
            Self(self.0 | 0x80)
        }

        #[must_use]
        #[inline]
        pub fn is_odd_straight(&self) -> bool {
            self.0 & (0x40 | 0x80) != 0
        }

        #[must_use]
        #[inline]
        pub fn as_card(&self) -> Cards {
            Cards(1 << self.0)
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
    #[repr(u8)]
    pub enum CardSuit {
        /// Diamonds - Blue
        Diamonds = 0,
        /// Clubs - Green
        Clubs = 1,
        /// Hearts - Red
        Hearts = 2,
        /// Spades - Black
        Spades = 3,
    }

    impl CardSuit {
        #[must_use]
        pub fn from(suit: u8) -> Self {
            assert!(suit < 4);
            match suit {
                0 => Self::Diamonds,
                1 => Self::Clubs,
                2 => Self::Hearts,
                3 => Self::Spades,
                _ => panic!("Invalid suit"),
            }
        }

        #[must_use]
        pub fn as_char(&self) -> char {
            match self {
                CardSuit::Diamonds => '\u{2666}',
                CardSuit::Clubs => '\u{2663}',
                CardSuit::Hearts => '\u{2665}',
                CardSuit::Spades => '\u{2660}',
            }
        }

        #[must_use]
        pub fn as_color(&self) -> &str {
            match self {
                CardSuit::Diamonds => "\u{1b}[34m",
                CardSuit::Clubs => "\u{1b}[32m",
                CardSuit::Hearts => "\u{1b}[31m",
                CardSuit::Spades => "\u{1b}[30m",
            }
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
    #[repr(u8)]
    pub enum CardRank {
        THREE = 3,
        FOUR,
        FIVE,
        SIX,
        SEVEN,
        EIGTH,
        NINE,
        TEN,
        JACK,
        QUEEN,
        KING,
        ACE,
        TWO,
    }

    impl CardRank {
        #[must_use]
        pub fn from(rank: u8) -> Self {
            match rank {
                3 => CardRank::THREE,
                4 => CardRank::FOUR,
                5 => CardRank::FIVE,
                6 => CardRank::SIX,
                7 => CardRank::SEVEN,
                8 => CardRank::EIGTH,
                9 => CardRank::NINE,
                10 => CardRank::TEN,
                11 => CardRank::JACK,
                12 => CardRank::QUEEN,
                13 => CardRank::KING,
                14 => CardRank::ACE,
                15 => CardRank::TWO,
                e => panic!("Cardnum {e} is not valid"),
            }
        }

        #[must_use]
        pub fn as_char(&self) -> char {
            let rank_str = b".+-3456789TJQKA2";
            char::from(rank_str[*self as usize])
        }
    }

    impl Sub for CardRank {
        type Output = u8;

        fn sub(self, rhs: Self) -> Self::Output {
            self as u8 - rhs as u8
        }
    }

    impl Add for CardRank {
        type Output = CardRank;

        fn add(self, rhs: Self) -> Self::Output {
            CardRank::from(self as u8 + rhs as u8)
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum ScoreKind {
        Single(CardNum),
        Pair(CardNum),
        Set(CardRank),
        Straight(CardNum),
        Flush(CardNum),
        FullHouse(CardRank),
        Quads(CardRank),
        StraightFlush(CardNum),
    }

    impl PartialOrd for ScoreKind {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            #[allow(clippy::match_same_arms)]
            match (self, other) {
                // Singles
                (ScoreKind::Single(a), ScoreKind::Single(b)) => a.partial_cmp(b),
                (ScoreKind::Single(_), _) => None,
                (_, ScoreKind::Single(_)) => None,
                // Pairs
                (ScoreKind::Pair(a), ScoreKind::Pair(b)) => a.partial_cmp(b),
                (ScoreKind::Pair(_), _) => None,
                (_, ScoreKind::Pair(_)) => None,
                // Sets
                (ScoreKind::Set(a), ScoreKind::Set(b)) => a.partial_cmp(b),
                (ScoreKind::Set(_), _) => None,
                (_, ScoreKind::Set(_)) => None,
                // Five cards
                // Straight
                (ScoreKind::Straight(a), ScoreKind::Straight(b)) => a.partial_cmp(b),
                (
                    ScoreKind::Straight(_),
                    ScoreKind::Flush(_)
                    | ScoreKind::FullHouse(_)
                    | ScoreKind::Quads(_)
                    | ScoreKind::StraightFlush(_),
                ) => Some(Ordering::Less),
                #[allow(unreachable_patterns)]
                (ScoreKind::Straight(_), _) => None,
                // Flush
                (ScoreKind::Flush(a), ScoreKind::Flush(b)) => a.partial_cmp(b),
                (
                    ScoreKind::Flush(_),
                    ScoreKind::FullHouse(_) | ScoreKind::Quads(_) | ScoreKind::StraightFlush(_),
                ) => Some(Ordering::Less),
                (ScoreKind::Flush(_), ScoreKind::Straight(_)) => Some(Ordering::Greater),
                #[allow(unreachable_patterns)]
                (ScoreKind::Flush(_), _) => None,
                // FullHouse
                (ScoreKind::FullHouse(a), ScoreKind::FullHouse(b)) => a.partial_cmp(b),
                (ScoreKind::FullHouse(_), ScoreKind::Straight(_) | ScoreKind::Flush(_)) => {
                    Some(Ordering::Greater)
                }
                (ScoreKind::FullHouse(_), ScoreKind::Quads(_) | ScoreKind::StraightFlush(_)) => {
                    Some(Ordering::Less)
                }
                #[allow(unreachable_patterns)]
                (ScoreKind::FullHouse(_), _) => None,
                // Quads
                (ScoreKind::Quads(a), ScoreKind::Quads(b)) => a.partial_cmp(b),
                (ScoreKind::Quads(_), ScoreKind::StraightFlush(_)) => Some(Ordering::Less),
                (
                    ScoreKind::Quads(_),
                    ScoreKind::Straight(_) | ScoreKind::Flush(_) | ScoreKind::FullHouse(_),
                ) => Some(Ordering::Greater),
                #[allow(unreachable_patterns)]
                (ScoreKind::Quads(_), _) => None,
                // StraightFlush
                (ScoreKind::StraightFlush(a), ScoreKind::StraightFlush(b)) => a.partial_cmp(b),
                (
                    ScoreKind::StraightFlush(_),
                    ScoreKind::Straight(_)
                    | ScoreKind::Flush(_)
                    | ScoreKind::FullHouse(_)
                    | ScoreKind::Quads(_),
                ) => Some(Ordering::Greater),
                #[allow(unreachable_patterns)]
                (ScoreKind::StraightFlush(_), _) => None,
            }
        }
    }

    #[must_use]
    pub fn has_rank(hand: u64, rank: CardRank) -> u64 {
        let mask = 0b1111 << ((rank as u8) << 2);
        hand & mask
    }

    #[must_use]
    pub fn cnt_rank(hand: u64, rank: CardRank) -> u64 {
        u64::from(has_rank(hand, rank).count_ones())
    }

    #[must_use]
    pub fn card_selected(card: Cards) -> CardNum {
        CardNum::highcard(card).expect("Should fit")
    }

    // #[must_use]
    // pub fn has_rank_idx(card: u64) -> u8 {
    //     card_selected(card) >> 2
    // }

    // #[must_use]
    // pub fn has_suit(card: u64) -> u64 {
    //     1 << (card_selected(card) & 0x3)
    // }
}

pub mod rules {
    use std::cmp::Ordering;

    use super::cards::{self, CardNum, CardRank, Cards, ScoreKind};

    // #[allow(dead_code)]
    // pub fn get_numbers(hand: u64) {
    //     let mut ranks: [u32; 16] = [0; 16];
    //     let mut straigth: u64 = 0;
    //     let mut tripps: u32 = 0;
    //     let mut quads: u32 = 0;
    //     let mut straigths: u32 = 0;
    //     let mut doubles: u32 = 0;

    //     for r in &RANKS {
    //         let idx: usize = (*r).into();
    //         ranks[idx] = cards::cnt_rank(hand, idx as u8) as u32;
    //         if ranks[idx] != 0 {
    //             straigth |= 1 << r;
    //         }
    //         if ranks[idx] == 2 {
    //             doubles += 1;
    //         }
    //         if ranks[idx] == 3 {
    //             tripps += 1;
    //         }
    //         if ranks[idx] == 4 {
    //             quads += 1;
    //         }
    //     }
    //     let mut mask = 0b11111;
    //     for _ in 4..16 {
    //         if straigth & mask == mask {
    //             straigths += 1;
    //         }
    //         mask <<= 1;
    //     }
    //     // A2345
    //     mask = 0b1100_0000_0011_1000;
    //     if straigth & mask == mask {
    //         straigths += 1;
    //     };
    //     // 23456
    //     mask = 0b1000_0000_0111_1000;
    //     if straigth & mask == mask {
    //         straigths += 1;
    //     };

    //     let flushs = has_flush(hand);

    //     let fullhouse = std::cmp::min(doubles, tripps)
    //         + std::cmp::min(doubles, quads)
    //         + std::cmp::min(tripps, quads);
    //     println!(
    //         "R{ranks:x?} S{straigth:16b} {straigths:x} D{doubles:x} T{tripps:x} Q{quads:x} FH{fullhouse:x} FL{flushs:x}"
    //     );
    // }
    #[must_use]
    pub fn is_valid_hand(hand: u64) -> bool {
        // Check cards range. Only the upper 52 bits are used.
        let ret: bool = hand.trailing_zeros() >= 12;

        // Check number of cards played. count = 1, 2, 3 or 5 is valid.
        let cardcount = hand.count_ones();
        ret && cardcount != 4 && cardcount < 6 && cardcount != 0
    }

    #[must_use]
    pub fn beter_hand(hand: Option<ScoreKind>, board: Option<ScoreKind>) -> Option<Ordering> {
        match (hand, board) {
            (None, _) => None,
            (Some(_), None) => Some(Ordering::Greater),
            (Some(h), Some(b)) => h.partial_cmp(&b),
        }
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
    pub fn is_flush3(mut hand: u64) -> bool {
        let mut start = hand.trailing_zeros();
        let mut ret = true;
        for _ in 0..4 {
            let bit = 1 << start;
            hand ^= bit;
            let sec = hand.trailing_zeros();
            if (sec - start) % 4 != 0 {
                ret = false;
            }
            start = sec;
        }
        ret
    }

    #[must_use]
    pub fn has_flush(hand: u64) -> u8 {
        let mut mask: u64 = 0x1111_1111_1111_1000;
        let mut flushs: u8 = 0;
        for _ in 0..4 {
            let num = (hand & mask).count_ones() / 5;
            if num != 0 {
                flushs += u8::try_from(num).unwrap();
            }
            mask <<= 1;
        }
        flushs
    }
    #[must_use]
    pub fn score_hand(hand: Cards) -> Option<ScoreKind> {
        // Score:
        //  0xKNN = One, Pair and Straigth, Flush
        //    |++- highest card: bit nummer of the highest card
        //    +--- Kind: Kind::ONE or Kind::TWO

        //  0xK0R = Set, Quad, FullHouse
        //    ||+- Rank: Only the RANK. Because only one RANK of each can exists.
        //    |+-- Zero
        //    +--- Kind: Kind::QUADS or Kind::SET or Kind::FULLHOUSE

        let lowest_card = CardNum::lowcard(hand)?;
        let card_cnt_hand = hand.count_ones();

        if card_cnt_hand == 1 {
            return Some(ScoreKind::Single(lowest_card));
        }

        // Check number of cards played. count = 1, 2, 3 or 5 is valid.
        if card_cnt_hand == 4 || card_cnt_hand > 5 {
            return None;
        }

        // find the highest card and calc the rank.
        let high_card = CardNum::highcard(hand)?;
        let high_rank = high_card.rank();

        // find the lowest card and calc the rank.
        let low_rank = lowest_card.rank();

        if card_cnt_hand <= 3 {
            // If cnt doesn't match the card_cnt then it is invalid hand.
            if high_rank != low_rank {
                return None;
            }
            if card_cnt_hand == 2 {
                return Some(ScoreKind::Pair(high_card));
            }

            return Some(ScoreKind::Set(high_rank));
        }

        // Get the played suit of that rank.
        let high_suitmask = hand.0 >> ((high_rank as u8) << 2);
        // Count number of cards based on the suit
        let high_cnt = high_suitmask.count_ones();

        // Get the played suit of that rank.
        let low_suitmask = hand.0 >> ((low_rank as u8) << 2) & 0xF;
        // Count number of cards based on the suit
        let low_cnt = low_suitmask.count_ones();

        // Quad
        if high_cnt == 4 {
            return Some(ScoreKind::Quads(high_rank));
        }
        if low_cnt == 4 {
            return Some(ScoreKind::Quads(low_rank));
        }

        // Full House
        if high_cnt == 3 && low_cnt == 2 {
            return Some(ScoreKind::FullHouse(high_rank));
        }
        if high_cnt == 2 && low_cnt == 3 {
            return Some(ScoreKind::FullHouse(low_rank));
        }

        // Flush
        let is_flush = {
            let mut mask: u64 = 0x1111_1111_1111_1000;
            let mut is_flush = false;
            for _ in 0..4 {
                if (hand.0 & !mask) == 0 {
                    is_flush = true;
                }
                mask <<= 1;
            }
            is_flush
        };

        // Straigth detection
        let maybe_straight = [4, 12].contains(&(high_rank - low_rank));

        if maybe_straight {
            let mut straigth_score: Option<CardNum> = None;

            if high_rank - low_rank == 4 {
                if cards::has_rank(hand.0, low_rank) != 0
                    && cards::has_rank(hand.0, CardRank::from(low_rank as u8 + 1)) != 0
                    && cards::has_rank(hand.0, CardRank::from(low_rank as u8 + 2)) != 0
                    && cards::has_rank(hand.0, CardRank::from(low_rank as u8 + 3)) != 0
                    && cards::has_rank(hand.0, CardRank::from(low_rank as u8 + 4)) != 0
                {
                    straigth_score = Some(high_card);
                }
            } else if cards::has_rank(hand.0, CardRank::TWO) != 0
                && cards::has_rank(hand.0, CardRank::THREE) != 0
                && cards::has_rank(hand.0, CardRank::FOUR) != 0
                && cards::has_rank(hand.0, CardRank::FIVE) != 0
            {
                // Straight 23456
                if cards::has_rank(hand.0, CardRank::SIX) != 0 {
                    straigth_score = Some(high_card.set_straight_23456());
                }
                // Straight A2345
                if cards::has_rank(hand.0, CardRank::ACE) != 0 {
                    straigth_score = Some(high_card.set_straight_A2345());
                }
            }

            if let Some(straigth_score) = straigth_score {
                if !is_flush {
                    return Some(ScoreKind::Straight(straigth_score));
                }
                return Some(ScoreKind::StraightFlush(straigth_score));
            }
        }

        if is_flush {
            return Some(ScoreKind::Flush(high_card));
        }

        None
    }
}

pub struct GameState {
    pub sm: network::StateMessage,
    pub srn: std::io::Stdout,
    pub board: Cards,
    pub board_score: Option<ScoreKind>,
    pub cards_selected: Cards,
    pub auto_pass: bool,
    pub i_am_ready: bool,
    pub card_selected_score: Option<Ordering>,
    pub hand: Cards,
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
            cards: [Cards::default(); 4],
            played_cards: Cards::default(),
            score: [0; 4],
            card_cnt: [13; 4],
        }
    }
    pub fn deal(&mut self, cards: Option<&[Cards; 4]>) {
        // create cards
        if let Some(cards) = cards {
            assert_eq!(cards.len(), 4);
            self.cards.copy_from_slice(cards);
        } else {
            self.cards = deck::deal();
        }

        // Setup
        self.round = self.round.saturating_add(1);
        self.has_passed = 0;
        self.board_score = None;
        self.has_passed = 0;
        self.card_cnt = [13; 4];

        let mut m: u64 = 0;
        for &c in &self.cards {
            m = m | c;
            println!("C 0x{:16x} count {}", c.0, c.0.count_ones());
        }
        let im = !(m | 0xFFF);
        println!("! 0x{:16x} M 0x{:16x} count {}", im, m, im.count_ones());

        // Disable by the limited info in the trace
        // assert_eq!(m, 0xFFFF_FFFF_FFFF_F000);

        // Which player to start
        self.turn = if self.round == 1 {
            i32::try_from(
                self.cards
                    .iter()
                    .position(|&x| x.has_lowest_card())
                    .expect("Weard a use should start with 0x1000 card!"),
            )
            .unwrap()
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

        let p = usize::try_from(player).unwrap() & 0x3;
        let pc = self.cards[p];

        let illegal_cards = (pc.0 ^ hand.0) & hand.0;
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
        self.last_action = hand.0 | (p as u64) | ((self.last_action & 0x3) << 2);

        self.board_score = Some(score);
        self.cards[p] ^= hand;

        let cnt = hand.count_ones();
        self.card_cnt[p] -= u8::try_from(cnt).expect("Should allways valid!");

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
            if board_score == ScoreKind::Single(CardNum::HIGHCARD)
                || board_score == ScoreKind::Pair(CardNum::HIGHCARD)
                || board_score == ScoreKind::Set(CardRank::TWO)
            {
                println!("Play 2s which is the highest card bs {board_score:?}");
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
        let prev_player = usize::try_from(self.prev_action & 0x3).unwrap();
        let curr_player = usize::try_from(self.last_action & 0x3).unwrap();
        let hand = self.last_action & 0xFFFF_FFFF_FFFF_F000;

        // Assist!
        let assisted = prev_player != curr_player
            && matches!(self.board_score, Some(ScoreKind::Single(_)))
            && hand < self.cards[prev_player].0;

        if assisted {
            println!(
                "Assist! PP{} {:16x} CP{} {:16x}",
                prev_player, self.cards[prev_player].0, self.turn, hand,
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
        self.score[curr_player] += total_score;
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use crate::big2rules::{
        cards::CardRank,
        rules::{is_flush, is_flush3, score_hand},
    };

    use super::*;

    #[test]
    fn a_rules_sizes() {
        assert!(!rules::is_valid_hand(0));
        assert!(!rules::is_valid_hand(0x1001));
        assert!(!rules::is_valid_hand(0b1), "1 invalid card");
        assert!(!rules::is_valid_hand(0b1 << 11));
        assert!(rules::is_valid_hand(0b1 << 12));
        assert!(rules::is_valid_hand(0b11 << 12));
        assert!(rules::is_valid_hand(0b111 << 12));
        assert!(!rules::is_valid_hand(0b1111 << 12), "4 cards");
        assert!(rules::is_valid_hand(0b11111 << 12));
        assert!(!rules::is_valid_hand(0b11_1111 << 12), "6 cards");
    }
    // #[test]
    // fn rules_board_hand_new_turn() {
    //     assert!(rules::beter_hand(None, 0b1 << 12));
    //     assert!(rules::beter_hand(None, 0b11 << 12));
    //     assert!(rules::beter_hand(None, 0b111 << 12));
    //     assert!(!rules::beter_hand(None, 0b1111 << 12));
    //     assert!(rules::beter_hand(None, 0b11111 << 12));
    // }
    // #[test]
    // fn rules_board_hand_one_pair() {
    //     assert!(rules::beter_hand(0b1 << 12, 0b1 << 12));
    //     assert!(!rules::beter_hand(0b1 << 12, 0b11 << 12));
    //     assert!(!rules::beter_hand(0b1 << 12, 0b111 << 12));
    //     assert!(!rules::beter_hand(0b1 << 12, 0b11111 << 12));
    // }
    // #[test]
    // fn rules_board_hand_two_pair() {
    //     assert!(!rules::beter_hand(0b11 << 12, 0b1 << 12));
    //     assert!(rules::beter_hand(0b11 << 12, 0b11 << 12));
    //     assert!(!rules::beter_hand(0b11 << 12, 0b111 << 12));
    //     assert!(!rules::beter_hand(0b11 << 12, 0b11111 << 12));
    // }
    // #[test]
    // fn rules_board_hand_three_of_kind() {
    //     assert!(!rules::beter_hand(0b111 << 12, 0b1 << 12));
    //     assert!(!rules::beter_hand(0b111 << 12, 0b11 << 12));
    //     assert!(rules::beter_hand(0b111 << 12, 0b111 << 12));
    //     assert!(!rules::beter_hand(0b111 << 12, 0b11111 << 12));
    // }
    // #[test]
    // fn rules_board_hand_fivecards() {
    //     assert!(!rules::beter_hand(0b11111 << 12, 0b1 << 12));
    //     assert!(!rules::beter_hand(0b11111 << 12, 0b11 << 12));
    //     assert!(!rules::beter_hand(0b11111 << 12, 0b111 << 12));
    //     assert!(rules::beter_hand(0b11111 << 12, 0b11111 << 12));
    // }
    #[test]
    fn b_rules_score_hand() {
        // INVALID
        assert_eq!(rules::score_hand(Cards(0x0000_0000_0000_0000)), None);
        let mut card = 0x0000_0000_0000_0001;
        for _ in 0..12 {
            assert_eq!(rules::score_hand(Cards(card)), None);
            card <<= 1;
        }
        assert_eq!(rules::score_hand(Cards(0x0000_0000_0000_F000)), None);
        assert_eq!(rules::score_hand(Cards(0x0000_0000_0003_F000)), None);
        assert_eq!(rules::score_hand(Cards(0x0000_0000_0000_F800)), None);

        // ONE
        assert_eq!(
            rules::score_hand(Cards(0x0000_0000_0000_1000)),
            Some(ScoreKind::Single(CardNum::LOWCARD))
        );
        assert_eq!(
            rules::score_hand(Cards(0x8000_0000_0000_0000)),
            Some(ScoreKind::Single(CardNum::HIGHCARD))
        );

        let mut card = 0x0000_0000_0000_1000;
        for c in 12..64 {
            assert_eq!(
                rules::score_hand(Cards(card)),
                Some(ScoreKind::Single(CardNum::try_from(c).unwrap()))
            );
            card <<= 1;
        }

        // PAIR
        assert_eq!(
            rules::score_hand(Cards(0b11 << 12)),
            Some(ScoreKind::Pair(CardNum::try_from(13).unwrap()))
        );
        // Select one 3 and one 4
        assert!(rules::score_hand(Cards(0b11000 << 12)).is_none());
        assert!(rules::score_hand(Cards(0b11 << 12)) < rules::score_hand(Cards(0b11 << 62)));

        // SET
        assert_eq!(
            rules::score_hand(Cards(0b0111 << 12)),
            Some(ScoreKind::Set(CardRank::THREE))
        );
        assert_eq!(
            rules::score_hand(Cards(0b1110 << 12)),
            Some(ScoreKind::Set(CardRank::THREE))
        );
        assert_eq!(
            rules::score_hand(Cards(0b1101 << 12)),
            Some(ScoreKind::Set(CardRank::THREE))
        );
        assert_eq!(
            rules::score_hand(Cards(0b1011 << 12)),
            Some(ScoreKind::Set(CardRank::THREE))
        );
        assert!(rules::score_hand(Cards(0b11100 << 12)).is_none());
        assert!(rules::score_hand(Cards(0b11 << 12)) < rules::score_hand(Cards(0b11 << 13)));

        // QUAD
        assert_eq!(
            rules::score_hand(Cards(0b0001_1111_0000 << 12)),
            Some(ScoreKind::Quads(CardRank::FOUR))
        );
        assert_eq!(
            rules::score_hand(Cards(0b0000_1111_1000 << 12)),
            Some(ScoreKind::Quads(CardRank::FOUR))
        );
        assert_eq!(
            rules::score_hand(Cards(0b0001_1111_0000 << 52)),
            Some(ScoreKind::Quads(CardRank::ACE))
        );
        assert_eq!(
            rules::score_hand(Cards(0b0000_1111_1000 << 52)),
            Some(ScoreKind::Quads(CardRank::ACE))
        );
        assert_eq!(
            rules::score_hand(Cards(0b1111_0000_1000 << 52)),
            Some(ScoreKind::Quads(CardRank::TWO))
        );
        assert!(rules::score_hand(Cards(0b1111_0001_1000 << 52)).is_none());
        assert!(rules::score_hand(Cards(0b1111_0000_1001 << 52)).is_none());

        // FULL HOUSE
        assert_eq!(
            rules::score_hand(Cards(0b0011_1011_0000 << 12)),
            Some(ScoreKind::FullHouse(CardRank::FOUR))
        );

        assert_eq!(
            rules::score_hand(Cards(0b0000_1101_1001 << 12)),
            Some(ScoreKind::FullHouse(CardRank::FOUR))
        );
        assert_eq!(
            rules::score_hand(Cards(0b0000_1011_0110 << 12)),
            Some(ScoreKind::FullHouse(CardRank::FOUR))
        );
        assert_eq!(
            rules::score_hand(Cards(0b1110_1001_0000 << 52)),
            Some(ScoreKind::FullHouse(CardRank::TWO))
        );
        assert_eq!(
            rules::score_hand(Cards(0b0000_0111_1001 << 52)),
            Some(ScoreKind::FullHouse(CardRank::ACE))
        );
        assert_eq!(
            rules::score_hand(Cards(0b0000_1101_0110 << 52)),
            Some(ScoreKind::FullHouse(CardRank::ACE))
        );

        // QUAD & FULLHOUSE
        let mut card = 0x0000_0000_0001_F000;
        for c in 12..60 {
            let score = rules::score_hand(Cards(card));
            match score {
                Some(ScoreKind::Quads(_) | ScoreKind::FullHouse(_)) => (),
                _ => panic!("Invalid score c {c} - {score:?}"),
            }
            card <<= 1;
        }

        // STRAIGHT
        assert_eq!(
            rules::score_hand(Cards(0x0002_1111 << 12)),
            Some(ScoreKind::Straight(CardNum::try_from(0x1d).unwrap()))
        );
        assert_eq!(
            rules::score_hand(Cards(0x0002_2221 << 12)),
            Some(ScoreKind::Straight(CardNum::try_from(0x1d).unwrap()))
        );
        assert_eq!(
            rules::score_hand(Cards(0x0000_0002_2221 << 12)),
            Some(ScoreKind::Straight(CardNum::try_from(0x1d).unwrap()))
        );
        // 23456
        assert_eq!(
            rules::score_hand(Cards(0x8000_0000_0111_1000)),
            Some(ScoreKind::Straight(CardNum::HIGHCARD.set_straight_23456()))
        );
        // A2345
        assert_eq!(
            rules::score_hand(Cards(0x8200_0000_0011_1000)),
            Some(ScoreKind::Straight(CardNum::HIGHCARD.set_straight_A2345()))
        );

        // FLUSH
        assert_eq!(
            rules::score_hand(Cards(0x0011_1101 << 12)),
            Some(ScoreKind::Flush(CardNum::try_from(32).unwrap()))
        );
        assert_eq!(
            rules::score_hand(Cards(0x8800_0000_0808_8000)),
            Some(ScoreKind::Flush(CardNum::HIGHCARD))
        );

        // STRAIGHT FLUSH
        assert_eq!(
            rules::score_hand(Cards(0x0001_1111 << 12)),
            Some(ScoreKind::StraightFlush(CardNum::try_from(0x1c).unwrap()))
        );
        assert_eq!(
            rules::score_hand(Cards(0x1111_1000_0000_0000)),
            Some(ScoreKind::StraightFlush(CardNum::try_from(0x3c).unwrap()))
        );
        assert_eq!(
            rules::score_hand(Cards(0x8888_8000_0000_0000)),
            Some(ScoreKind::StraightFlush(CardNum::HIGHCARD))
        );
        // 23456
        assert_eq!(
            rules::score_hand(Cards(0x8000_0000_0888_8000)),
            Some(ScoreKind::StraightFlush(
                CardNum::HIGHCARD.set_straight_23456()
            ))
        );
        assert_eq!(
            rules::score_hand(Cards(0x1000_0000_0111_1000)),
            Some(ScoreKind::StraightFlush(
                CardNum::try_from(0x3c).unwrap().set_straight_23456()
            ))
        );
        // A2345
        assert_eq!(
            rules::score_hand(Cards(0x8800_0000_0088_8000)),
            Some(ScoreKind::StraightFlush(
                CardNum::HIGHCARD.set_straight_A2345()
            ))
        );
        assert_eq!(
            rules::score_hand(Cards(0x1100_0000_0011_1000)),
            Some(ScoreKind::StraightFlush(
                CardNum::try_from(0x3c).unwrap().set_straight_A2345()
            ))
        );

        // GARBAGE
        assert!(rules::score_hand(Cards(0x0001_0311 << 12)).is_none());
    }
    #[test]
    fn c_deal_hand() {
        // No cards generated
        assert!(deck::deal() != [Cards(0); 4]);
        // Detect shuffle is did not work at all.
        assert!(
            deck::deal()
                != [
                    Cards(0x1111_1111_1111_1000),
                    Cards(0x2222_2222_2222_2000),
                    Cards(0x4444_4444_4444_4000),
                    Cards(0x8888_8888_8888_8000),
                ]
        );
    }
    // #[test]
    // fn d_cards_test() {
    //     // No cards generated
    //     let card: u64 = 0x1000;
    //     assert_eq!(cards::has_rank_idx(card), CardRank::THREE as u8);
    //     assert!(cards::has_suit(card) == cards::Kind::DIAMONDS);
    //     let card: u64 = 0x20000;
    //     assert!(cards::has_rank_idx(card) == CardRank::FOUR);
    //     assert!(cards::has_suit(card) == cards::Kind::CLUBS);
    //     let card: u64 = 0x0400_0000_0000_0000;
    //     assert!(cards::has_rank_idx(card) == CardRank::ACE);
    //     assert!(cards::has_suit(card) == cards::Kind::HEARTS);
    //     let card: u64 = 0x8000_0000_0000_0000;
    //     assert!(cards::has_rank_idx(card) == CardRank::TWO);
    //     assert!(cards::has_suit(card) == cards::Kind::SPADES);
    // }

    #[test]
    fn assist_test() {
        let mut gs = SrvGameState::new(1);
        gs.deal(Some(&[
            Cards(0x1111_1111_1111_1000),
            Cards(0x2222_2222_2222_2000),
            Cards(0x4444_4444_4444_4000),
            Cards(0x8888_8888_8888_8000),
        ]));
        assert_eq!(gs.turn, 0);

        // reduct cards
        gs.cards = [Cards(0x24000), Cards(0x8000), Cards(0x2000), Cards(0x1000)];
        gs.card_cnt = [2, 1, 1, 1];

        assert!(gs.play(0, Cards(0x4000)).is_ok());
        assert!(gs.play(1, Cards(0x8000)).is_ok());
        assert_eq!(gs.score, [-3, 3, 0, 0]);
    }
    #[test]
    fn non_assist_test() {
        let mut gs = SrvGameState::new(1);
        gs.deal(Some(&[
            Cards(0x1111_1111_1111_1000),
            Cards(0x2222_2222_2222_2000),
            Cards(0x4444_4444_4444_4000),
            Cards(0x8888_8888_8888_8000),
        ]));
        assert_eq!(gs.turn, 0);

        // reduct cards
        gs.cards = [Cards(0x5000), Cards(0x8000), Cards(0x2000), Cards(0x40000)];
        gs.card_cnt = [2, 1, 1, 1];

        assert!(gs.play(0, Cards(0x4000)).is_ok());
        assert!(gs.play(1, Cards(0x8000)).is_ok());
        assert_eq!(gs.score, [-1, 3, -1, -1]);
    }
    #[test]
    fn score_multiply_test() {
        let mut gs = SrvGameState::new(1);
        gs.deal(Some(&[
            Cards(0x1111_1111_1111_1000),
            Cards(0x2222_2222_2222_2000),
            Cards(0x4444_4444_4444_4000),
            Cards(0x8888_8888_8888_8000),
        ]));
        assert_eq!(gs.turn, 0);

        // reduct cards
        gs.cards[2] = Cards(0x8000_0000_0000_0000);
        gs.card_cnt[2] = 1;

        assert!(gs.play(0, Cards(0x1000)).is_ok());
        assert!(gs.play(1, Cards(0x2000)).is_ok());
        assert!(gs.play(2, Cards(0x8000_0000_0000_0000)).is_ok());
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
    fn better_five_cards() {
        let board = Cards(0x8200_0000_0088_2000);
        let board_score = score_hand(board);

        let my_hand = Cards(0x0000_0000_F000_1000);
        let hand_score = score_hand(my_hand);
        let play = rules::beter_hand(hand_score, board_score);
        assert_eq!(play, Some(Ordering::Greater));

        let board_score = score_hand(board).unwrap();
        assert_eq!(
            board_score,
            ScoreKind::Straight(CardNum::HIGHCARD.set_straight_A2345())
        );
        let hand_score = score_hand(my_hand).unwrap();
        assert_eq!(hand_score, ScoreKind::Quads(CardRank::SEVEN));

        let cmp = board_score.partial_cmp(&hand_score);
        println!("{board_score:?} vs {hand_score:?} = {cmp:?}");

        assert_eq!(cmp, Some(Ordering::Less));

        let cmp = hand_score.partial_cmp(&board_score);
        println!("{hand_score:?} vs {board_score:?} = {cmp:?}");

        assert_eq!(cmp, Some(Ordering::Greater));

        let is_valid_hand = match (Some(board_score), Some(hand_score)) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(b), Some(h)) => matches!(b.partial_cmp(&h), Some(Ordering::Less)),
        };
        assert!(is_valid_hand);
    }

    #[test]
    fn random_score_test() {
        let card_low = CardNum::try_from(24).unwrap();
        let card_hi = CardNum::try_from(26).unwrap();
        assert_eq!(card_hi.cmp(&card_low), Ordering::Greater);

        assert_eq!(
            ScoreKind::Single(CardNum::try_from(26).unwrap())
                .partial_cmp(&ScoreKind::Single(CardNum::try_from(24).unwrap())),
            Some(Ordering::Greater)
        );
        assert_eq!(
            ScoreKind::Single(CardNum::try_from(24).unwrap())
                .partial_cmp(&ScoreKind::Single(CardNum::try_from(26).unwrap())),
            Some(Ordering::Less)
        );

        assert_eq!(
            ScoreKind::FullHouse(CardRank::TEN)
                .partial_cmp(&ScoreKind::Flush(CardNum::try_from(36).unwrap())),
            Some(Ordering::Greater)
        );

        assert_eq!(
            ScoreKind::Flush(CardNum::try_from(36).unwrap())
                .partial_cmp(&ScoreKind::FullHouse(CardRank::TEN)),
            Some(Ordering::Less)
        );
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
                Cards(0x0000_0000_01FF_F000),
                Cards(0x0000_003F_FE00_0000),
                Cards(0x0007_FFC0_0000_0000),
                Cards(0xFFF8_0000_0000_0000),
            ]
        );
    }

    #[test]
    fn test_is_flush() {
        let mut cards = 0x1_1111 << 12;
        for _ in 0..4 {
            assert!(is_flush(cards));
            assert!(is_flush3(cards));
            cards <<= 1;
        }

        let mut cards = 0x2_1111 << 12;
        for _ in 0..4 {
            assert!(!is_flush(cards));
            assert!(!is_flush3(cards));
            cards <<= 1;
        }
        // let mut cards = 0x1111 << 12;
        // for _ in 0..4 {
        //     // assert!(!is_flush(cards));
        //     assert!(!is_flush3(cards));
        //     cards <<= 1;
        // }
    }
}

// mod verification {
//     use self::{cards::has_rank, rules::score_hand};

//     use super::*;

//     #[kani::proof]
//     pub fn check_something() {
//         let hand = kani::any();
//         //kani::assume(hand > 0xFFF);
//         let hand = Cards(hand);
//         score_hand(hand);
//     }

//     #[kani::proof]
//     pub fn kani_has_rank() {
//         let rank: u8 = kani::any();
//         kani::assume(rank > 2 && rank < 16);
//         let rank = CardRank::from(rank);
//         let hand = kani::any();
//         kani::assume(hand > 0xFFF);
//         has_rank(hand, rank);
//     }
// }
