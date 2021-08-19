use self::{cards::ScoreCards, rules::CardScore};

//use crate::network;

pub const RANKS: [u8; 13] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
pub const NUMBER_OF_PLAYERS: usize = 4;

pub mod deck {
    use super::*;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    pub const NUMBER_OF_CARDS: u8 = 52;
    pub const START_BIT: u8 = 12;

    pub fn deal() -> [u64; NUMBER_OF_PLAYERS] {
        // Create and shulle deck of cards
        let deck = {
            let mut deck: Vec<u8> = (0..deck::NUMBER_OF_CARDS)
                .into_iter()
                .map(|c| deck::START_BIT + c)
                .collect();

            // Randomize/shuffle the cards
            for _ in 0..256 {
                deck.shuffle(&mut thread_rng());
            }

            deck
        };

        // deck must be 52 cards in size.
        assert!(deck.len() == 52);

        let mut players_hand: [u64; NUMBER_OF_PLAYERS] = [0, 0, 0, 0];

        // deal the cards to the players.
        for (card_nr, value) in deck.into_iter().enumerate() {
            let player = card_nr & 0x3;
            players_hand[player] |= 1 << value;
        }

        assert!(
            (players_hand[0] | players_hand[1] | players_hand[2] | players_hand[3])
                == 0xFFFF_FFFF_FFFF_F000u64
        );

        players_hand
    }
}

pub mod cards {
    use std::cmp::Ordering;

    use super::rules::{is_valid_hand, score_hand, CardScore};

    #[derive(PartialEq, Eq)]
    pub enum CardRank {
        THREE,
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

    impl From<u8> for CardRank {
        fn from(item: u8) -> CardRank {
            match item >> 2 {
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
                _ => CardRank::TWO,
            }
        }
    }
    impl Into<u8> for CardRank {
        fn into(self) -> u8 {
            match self {
                CardRank::THREE => 3,
                CardRank::FOUR => 4,
                CardRank::FIVE => 5,
                CardRank::SIX => 6,
                CardRank::SEVEN => 7,
                CardRank::EIGTH => 8,
                CardRank::NINE => 9,
                CardRank::TEN => 10,
                CardRank::JACK => 11,
                CardRank::QUEEN => 12,
                CardRank::KING => 13,
                CardRank::ACE => 14,
                CardRank::TWO => 15,
            }
        }
    }

    #[derive(PartialEq, Eq)]
    pub enum CardSuit {
        DIAMONDS,
        CLUBS,
        HEARTS,
        SPADES,
    }

    impl From<u8> for CardSuit {
        fn from(item: u8) -> Self {
            match item & 0x3 {
                1 => CardSuit::CLUBS,
                2 => CardSuit::HEARTS,
                3 => CardSuit::SPADES,
                _ => CardSuit::DIAMONDS,
            }
        }
    }

    impl Into<u8> for CardSuit {
        fn into(self) -> u8 {
            match self {
                CardSuit::DIAMONDS => 0,
                CardSuit::CLUBS => 1,
                CardSuit::HEARTS => 2,
                CardSuit::SPADES => 3,
            }
        }
    }

    pub struct Card {
        bit: u8,
        pub rank: CardRank,
        pub suit: CardSuit,
    }

    impl Card {
        //! card is bit number in the deck. 12 to 63.
        pub fn new(card_bit: usize) -> Result<Self, ()> {
            if !(12..64).contains(&card_bit) {
                return Err(());
            }
            let bit = card_bit as u8;
            let suit = CardSuit::from(bit);
            let rank = CardRank::from(bit);

            Ok(Self { bit, rank, suit })
        }
        pub fn bit(&self) -> u8 {
            self.bit
        }
        pub fn card(rank: CardRank, suit: CardSuit) -> Self {
            let rank_bit: u8 = rank.into();
            let suit_bit: u8 = suit.into();
            let bit = (rank_bit << 2) + suit_bit;
            Card::new(bit as usize).unwrap()
        }
    }

    impl PartialEq for Card {
        fn eq(&self, other: &Self) -> bool {
            self.bit == other.bit
        }
    }

    impl Eq for Card {}

    impl Ord for Card {
        fn cmp(&self, other: &Self) -> Ordering {
            if self.bit > other.bit {
                return Ordering::Greater;
            };
            if self.bit < other.bit {
                return Ordering::Less;
            };
            Ordering::Equal
        }
    }

    impl PartialOrd for Card {
        fn partial_cmp(&self, other: &Card) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl PartialEq<CardSuit> for Card {
        fn eq(&self, other: &CardSuit) -> bool {
            self.suit == *other
        }
    }

    impl PartialEq<CardRank> for Card {
        fn eq(&self, other: &CardRank) -> bool {
            self.rank == *other
        }
    }

    impl ToString for Card {
        fn to_string(&self) -> String {
            let mut card_str = String::with_capacity(2);
            match self.rank {
                CardRank::THREE => card_str.push('3'),
                CardRank::FOUR => card_str.push('4'),
                CardRank::FIVE => card_str.push('5'),
                CardRank::SIX => card_str.push('6'),
                CardRank::SEVEN => card_str.push('7'),
                CardRank::EIGTH => card_str.push('8'),
                CardRank::NINE => card_str.push('9'),
                CardRank::TEN => card_str.push('T'),
                CardRank::JACK => card_str.push('J'),
                CardRank::QUEEN => card_str.push('Q'),
                CardRank::KING => card_str.push('K'),
                CardRank::ACE => card_str.push('A'),
                CardRank::TWO => card_str.push('2'),
            }

            match self.suit {
                CardSuit::DIAMONDS => card_str.push('\u{2666}'),
                CardSuit::CLUBS => card_str.push('\u{2663}'),
                CardSuit::HEARTS => card_str.push('\u{2665}'),
                CardSuit::SPADES => card_str.push('\u{2660}'),
            }

            card_str
        }
    }

    pub struct Cards {
        cards: u64,
    }

    pub struct ScoreCards {
        cards: Cards,
        score: CardScore,
    }

    impl ScoreCards {
        pub fn hand_from(cards: u64) -> Result<Self, ()> {
            if cards == 0 {
                return Ok(Self {
                    cards: Cards::hand_from(cards).unwrap(),
                    score: CardScore::None,
                });
            }
            if !is_valid_hand(cards) {
                return Err(());
            }
            let c = Cards::hand_from(cards);
            if let Ok(cards) = c {
                let score = score_hand(cards.cards());
                return Ok(Self { cards, score });
            }
            Err(())
        }
        pub fn board_from(cards: u64) -> Result<Self, ()> {
            if cards == 0 {
                return Ok(Self {
                    cards: Cards::hand_from(cards).unwrap(),
                    score: CardScore::None,
                });
            }
            if !is_valid_hand(cards) {
                return Err(());
            }
            let c = Cards::hand_from(cards);
            if let Ok(cards) = c {
                let score = score_hand(cards.cards());
                return Ok(Self { cards, score });
            }
            Err(())
        }

        pub fn is_empty(&self) -> bool {
            self.cards.cards() == 0
        }

        pub fn score(&self) -> &CardScore {
            &self.score
        }

        pub fn is_better(&self, b: &CardScore) -> bool {
            match (&self.score, b) {
                (_, CardScore::None) => true,
                (CardScore::Single(a), CardScore::Single(b)) => *a > *b,
                (CardScore::Pair(a), CardScore::Pair(b)) => *a > *b,
                (CardScore::Set(a), CardScore::Set(b)) => *a > *b,
                (CardScore::Five(a, ac), CardScore::Five(b, bc)) => {
                    if *a == *b && *ac > *bc {
                        true
                    } else if *a > *b {
                        true
                    } else {
                        false
                    }
                }

                (_, _) => false,
            }
        }
    }

    impl Cards {
        pub fn hand_from(cards: u64) -> Result<Self, ()> {
            if cards & 0xFFF != 0 {
                return Err(());
            }
            if cards.count_ones() > 13 {
                return Err(());
            }

            Ok(Self { cards })
        }

        pub fn board_from(cards: u64) -> Result<Self, ()> {
            if cards == 0 {
                return Ok(Self { cards });
            }

            if !crate::big2rules::rules::is_valid_hand(cards) {
                return Err(());
            }
            // let (_, score) = crate::big2rules::rules::score_hand(cards);

            Ok(Self { cards })
        }
        pub fn cards(&self) -> u64 {
            self.cards
        }

        pub fn to_bit(&self) -> u32 {
            self.cards.trailing_zeros()
        }
    }

    impl Iterator for Cards {
        type Item = (u32, u64);

        fn next(&mut self) -> Option<Self::Item> {
            if self.cards == 0 {
                return None;
            }
            let bit = self.cards.trailing_zeros();
            let mask = 1 << bit;
            self.cards = self.cards ^ mask;
            Some((bit, mask))
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, Some(self.cards.count_ones() as usize))
        }
    }

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

        pub const HIGHEST: u8 = 0x3f;
        pub const LOWEST: u8 = 12;
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

    pub fn has_rank(hand: u64, rank: u64) -> u64 {
        let mask = Kind::SUITMASK << (rank << 2);
        hand & mask
    }

    pub fn cnt_rank(hand: u64, rank: u64) -> u64 {
        has_rank(hand, rank).count_ones() as u64
    }

    pub fn card_selected(card: u64) -> u64 {
        card.trailing_zeros() as u64
    }

    pub fn has_rank_idx(card: u64) -> u64 {
        card_selected(card) >> 2
    }

    pub fn has_suit(card: u64) -> u64 {
        1 << (card_selected(card) & 0x3)
    }
}

pub mod rules {
    use super::*;

    pub fn have_to_pass(board: u64, hand: u64) -> bool {
        let board_cnt = board.count_ones();
        let hand_cnt = hand.count_ones();

        board_cnt > hand_cnt
    }

    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn beter_hand(board: u64, hand: u64) -> bool {
        if !is_valid_hand(hand) {
            return false;
        }

        let card_cnt_hand = hand.count_ones();
        let card_cnt_board = board.count_ones();

        // Board and hand count must match.
        // Board count 0 means new turn.
        if card_cnt_board != 0 && card_cnt_board != card_cnt_hand {
            return false;
        }

        true
    }

    pub fn higher_single_card(board: u64, hand: u64) -> u64 {
        let mask: u64 = u64::MAX.wrapping_shl(board.trailing_zeros());
        let higher_cards = hand & mask;
        let mask: u64 = 1u64.wrapping_shl(higher_cards.trailing_zeros());

        hand & mask
    }

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

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    pub enum FiveCardStraight {
        Normal,
        S23456,
        SA2345,
    }

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    pub enum FiveCard {
        STRAIGHT(FiveCardStraight),
        FLUSH,
        FULLHOUSE,
        QUADS,
        STRAIGHTFLUSH(FiveCardStraight),
    }

    #[derive(PartialEq, Eq)]
    pub enum CardScore {
        None,
        Invalid,
        Single(u8),
        Pair(u8),
        Set(u8),
        Five(FiveCard, u8),
    }

    pub fn score_hand(hand: u64) -> CardScore {
        // Score:
        //  0xKNN = One, Pair and Straigth, Flush
        //    |++- highest card: bit nummer of the highest card
        //    +--- Kind: Kind::ONE or Kind::TWO

        //  0xK0R = Set, Quad, FullHouse
        //    ||+- Rank: Only the RANK. Because only one RANK of each can exists.
        //    |+-- Zero
        //    +--- Kind: Kind::QUADS or Kind::SET or Kind::FULLHOUSE

        if !is_valid_hand(hand) {
            return CardScore::Invalid;
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
                return CardScore::Invalid;
            }

            if card_cnt_hand == 1 {
                return CardScore::Single(highest_card as u8);
            }
            if card_cnt_hand == 2 {
                return CardScore::Pair(highest_card as u8);
            }
            if card_cnt_hand == 3 {
                return CardScore::Set(rank as u8);
            }

            return CardScore::Invalid;
        }

        let lowest_card: u64 = hand.trailing_zeros() as u64;
        let low_rank: u64 = lowest_card >> 2;
        // Get the played suit of that rank.
        let low_suitmask = hand >> (low_rank << 2) & cards::Kind::SUITMASK;
        // Count number of cards based on the suit
        let low_cnt: u64 = low_suitmask.count_ones() as u64;

        // Quad
        if cnt == 4 {
            return CardScore::Five(FiveCard::QUADS, rank as u8);
        }
        if low_cnt == 4 {
            return CardScore::Five(FiveCard::QUADS, low_rank as u8);
        }

        // Full House
        if cnt == 3 && low_cnt == 2 {
            return CardScore::Five(FiveCard::FULLHOUSE, rank as u8);
        }
        if cnt == 2 && low_cnt == 3 {
            return CardScore::Five(FiveCard::FULLHOUSE, low_rank as u8);
        }

        // Flush
        let is_flush: bool = is_flush(hand);

        // Straigth detection
        let mut is_straight: bool = rank - low_rank == 4 || rank - low_rank == 12;

        if is_straight {
            let mut straigth_score: u64 = 0;
            let mut straigth_type: FiveCardStraight = FiveCardStraight::Normal;
            if rank - low_rank == 12 {
                is_straight = cards::has_rank(hand, cards::Rank::THREE as u64) != 0
                    && cards::has_rank(hand, cards::Rank::FOUR as u64) != 0
                    && cards::has_rank(hand, cards::Rank::FIVE as u64) != 0
                    && cards::has_rank(hand, cards::Rank::TWO as u64) != 0;
                // Straight 23456
                if is_straight && cards::has_rank(hand, cards::Rank::SIX as u64) != 0 {
                    straigth_score = highest_card;
                    straigth_type = FiveCardStraight::S23456;
                }
                // Straight A2345
                if is_straight && cards::has_rank(hand, cards::Rank::ACE as u64) != 0 {
                    straigth_score = highest_card;
                    straigth_type = FiveCardStraight::SA2345;
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
                    return CardScore::Five(
                        FiveCard::STRAIGHTFLUSH(straigth_type),
                        highest_card as u8,
                    );
                }
                return CardScore::Five(FiveCard::STRAIGHT(straigth_type), highest_card as u8);
            }
        }

        if !is_straight && is_flush {
            return CardScore::Five(FiveCard::FLUSH, highest_card as u8);
        }

        CardScore::Invalid
    }
}

// pub struct GameState {
//     pub sm: network::StateMessage,
//     pub srn: std::io::Stdout,
//     pub board: u64,
//     pub board_score: u64,
//     pub cards_selected: u64,
//     pub auto_pass: bool,
//     pub i_am_ready: bool,
//     pub is_valid_hand: bool,
//     pub hand_score: u64,
// }

type Cards = u64;
type PlayerID = u8;

pub enum GameActions {
    Deal {
        cards: [Cards; 4],
        shuffel: u8,
        to_act: PlayerID,
    },
    Undo,
    Play {
        player: PlayerID,
        cards: Cards,
    },
    Pass {
        player: PlayerID,
    },
    Score {
        won: PlayerID,
        score: [i16; 4],
        assist: Option<PlayerID>,
        end: Option<PlayerID>,
    },
}

pub struct SrvGameState {
    // pub board: Cards,
    // pub round: u8,
    // pub rounds: u8,
    // pub cards: [Cards; 4],
    // pub played_cards: Cards,
    // pub score: [i16; 4],
    // pub trail: Vec<GameActions>,
    // pub board: Cards,
    // pub round: u8,
    // pub rounds: u8,
    // pub cards: [Cards; 4],
    // pub played_cards: Cards,
    // pub score: [i16; 4],
    // pub trail: Vec<GameActions>,
    pub prev_action: u64,
    pub last_action: u64,
    pub board_score: ScoreCards,
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
    PlayerPlayedIllegalCard(u64),
    InvalidHand,
    AllreadyPassed,
}

impl SrvGameState {
    pub fn new(rounds: u8) -> Self {
        SrvGameState {
            prev_action: 0,
            last_action: 0,
            board_score: ScoreCards::board_from(0).unwrap(),
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
        self.board_score = ScoreCards::board_from(0).unwrap();
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
        let pc = self.cards[p];
        let illegal_cards = (pc ^ hand) & hand;

        if illegal_cards != 0 {
            return Err(SrvGameError::PlayerPlayedIllegalCard(illegal_cards));
        }

        let hand_score = ScoreCards::hand_from(hand);
        if hand_score.is_err() {
            return Err(SrvGameError::InvalidHand);
        }

        let hand_score = hand_score.unwrap();
        if !hand_score.is_better(self.board_score.score()) {
            return Err(SrvGameError::InvalidHand);
        }

        self.prev_action = self.last_action;
        self.last_action = hand | (p as u64) | ((self.last_action & 0x3) << 2);

        self.board_score = hand_score;
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

        let bs = self.board_score.score();
        if *bs == CardScore::Single(0x3f)
            || *bs == CardScore::Pair(0x3f)
            || *bs == CardScore::Set(0x3f)
        {
            println!("Play 2s which is the highest card");
            self.board_score = ScoreCards::board_from(0).unwrap();
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
            self.board_score = ScoreCards::board_from(0).unwrap();
            self.has_passed = 0;
            println!("\tEveryone has passed");
        }

        self.turn = next;
    }
    fn calc_score(&mut self) {
        let mut t: i16 = 0;

        let prev_player = self.prev_action as usize & 0x3;
        let curr_player = self.last_action as usize & 0x3;
        let hand = self.last_action & 0xFFFF_FFFF_FFFF_F000;

        let assisted = false;
        // // Assist!
        // let assisted = self.board_score & 0xF00 == 0x100
        //     && prev_player != curr_player
        //     && hand < self.cards[prev_player];

        // if assisted {
        //     println!(
        //         "Assist! PP{} {:16x} CP{} {:16x}",
        //         prev_player, self.cards[prev_player], self.turn, hand,
        //     )
        // }

        let mut delta_score: [i16; 4] = [0; 4];
        for (item, card_cnt) in delta_score.iter_mut().zip(self.card_cnt.iter()) {
            let mut s = *(card_cnt) as i16;
            if s == 13 {
                s *= 3
            } else if s > 9 {
                s *= 2
            };
            t += s;
            *item = s;
        }
        if assisted {
            self.score[prev_player] -= t
        } else {
            for (score, d_score) in self.score.iter_mut().zip(delta_score.iter()) {
                *score -= *d_score;
            }
        }
        self.score[self.turn as usize] += t;
    }
}

#[cfg(test)]
mod tests {
    use crate::big2rules::{
        cards::{CardRank, CardSuit},
        rules::FiveCard,
    };

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
        assert!(rules::score_hand(0x0000_0000_0000_1000) == CardScore::Single(cards::Kind::LOWEST));
        assert!(
            rules::score_hand(0x8000_0000_0000_0000) == CardScore::Single(cards::Kind::HIGHEST)
        );

        // PAIR
        assert!(rules::score_hand(0b11 << 12) == CardScore::Pair(13));
        // Select one 3 and one 4
        assert!(rules::score_hand(0b11000 << 12) == CardScore::Invalid);
        // assert!(!rules::score_hand(0b11 << 12).is_better(rules::score_hand(0b11 << 62)));

        // // SET
        // assert!(rules::score_hand(0b0111 << 12) == cards::Kind::SET | cards::Rank::THREE);
        // assert!(rules::score_hand(0b1110 << 12) == cards::Kind::SET | cards::Rank::THREE);
        // assert!(rules::score_hand(0b1101 << 12) == cards::Kind::SET | cards::Rank::THREE);
        // assert!(rules::score_hand(0b1011 << 12) == cards::Kind::SET | cards::Rank::THREE);
        // assert!(rules::score_hand(0b11100 << 12) == 0);
        // assert!(rules::score_hand(0b11 << 12) < rules::score_hand(0b11 << 13));

        // // QUAD
        assert!(
            rules::score_hand(0b0001_1111_0000 << 12)
                == CardScore::Five(FiveCard::QUADS, cards::Rank::FOUR)
        );
        assert!(
            rules::score_hand(0b0000_1111_1000 << 12)
                == CardScore::Five(FiveCard::QUADS, cards::Rank::FOUR)
        );
        assert!(
            rules::score_hand(0b0001_1111_0000 << 52)
                == CardScore::Five(FiveCard::QUADS, cards::Rank::ACE)
        );
        assert!(
            rules::score_hand(0b0000_1111_1000 << 52)
                == CardScore::Five(FiveCard::QUADS, cards::Rank::ACE)
        );
        assert!(
            rules::score_hand(0b1111_0000_1000 << 52)
                == CardScore::Five(FiveCard::QUADS, cards::Rank::TWO)
        );
        assert!(rules::score_hand(0b1111_0001_1000 << 52) == CardScore::Invalid);
        assert!(rules::score_hand(0b1111_0000_1001 << 52) == CardScore::Invalid);

        // // FULL HOUSE
        assert!(
            rules::score_hand(0b0011_1011_0000 << 12)
                == CardScore::Five(FiveCard::FULLHOUSE, cards::Rank::FOUR)
        );
        assert!(
            rules::score_hand(0b0000_1101_1001 << 12)
                == CardScore::Five(FiveCard::FULLHOUSE, cards::Rank::FOUR)
        );
        assert!(
            rules::score_hand(0b0000_1011_0110 << 12)
                == CardScore::Five(FiveCard::FULLHOUSE, cards::Rank::FOUR)
        );
        assert!(
            rules::score_hand(0b1110_1001_0000 << 52)
                == CardScore::Five(FiveCard::FULLHOUSE, cards::Rank::TWO)
        );
        assert!(
            rules::score_hand(0b0000_0111_1001 << 52)
                == CardScore::Five(FiveCard::FULLHOUSE, cards::Rank::ACE)
        );
        assert!(
            rules::score_hand(0b0000_1101_0110 << 52)
                == CardScore::Five(FiveCard::FULLHOUSE, cards::Rank::ACE)
        );

        // // STRAIGHT
        // assert!(rules::score_hand(0x0002_1111 << 12) == cards::Kind::STRAIGHT | 0x1d);
        // assert!(rules::score_hand(0x0002_2221 << 12) == cards::Kind::STRAIGHT | 0x1d);
        // assert!(rules::score_hand(0x0000_0002_2221 << 12) == cards::Kind::STRAIGHT | 0x1d);
        // // 23456
        // assert!(
        //     rules::score_hand(0x8000_0000_0111_1000)
        //         == cards::Kind::STRAIGHT | cards::Kind::HIGHEST | 0x40
        // );
        // // A2345
        // assert!(
        //     rules::score_hand(0x8200_0000_0011_1000)
        //         == cards::Kind::STRAIGHT | cards::Kind::HIGHEST | 0x80
        // );

        // // FLUSH
        // assert!(rules::score_hand(0x0011_1101 << 12) == cards::Kind::FLUSH | 32);
        // assert!(
        //     rules::score_hand(0x8800_0000_0808_8000) == cards::Kind::FLUSH | cards::Kind::HIGHEST
        // );

        // // STRAIGHT FLUSH
        // assert!(rules::score_hand(0x0001_1111 << 12) == cards::Kind::STRAIGHTFLUSH | 0x1c);
        // assert!(rules::score_hand(0x1111_1000_0000_0000) == cards::Kind::STRAIGHTFLUSH | 0x3c);
        // assert!(
        //     rules::score_hand(0x8888_8000_0000_0000)
        //         == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST
        // );
        // // 23456
        // assert!(
        //     rules::score_hand(0x8000_0000_0888_8000)
        //         == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST | 0x40
        // );
        // assert!(
        //     rules::score_hand(0x1000_0000_0111_1000) == cards::Kind::STRAIGHTFLUSH | 0x3c | 0x40
        // );
        // // A2345
        // assert!(
        //     rules::score_hand(0x8800_0000_0088_8000)
        //         == cards::Kind::STRAIGHTFLUSH | cards::Kind::HIGHEST | 0x80
        // );
        // assert!(
        //     rules::score_hand(0x1100_0000_0011_1000) == cards::Kind::STRAIGHTFLUSH | 0x3c | 0x80
        // );

        // // GARBAGE
        // assert!(rules::score_hand(0x0001_0311 << 12) == 0);
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
        use super::cards::{Card, CardRank, CardSuit};

        // No cards generated
        let card = Card::new(12).unwrap();
        assert!(card.rank == CardRank::THREE);
        assert!(card.suit == CardSuit::DIAMONDS);
        assert!(card.bit() == 12);

        let card = Card::new(15).unwrap();
        assert!(card.rank == CardRank::THREE);
        assert!(card.suit == CardSuit::SPADES);
        assert!(card.bit() == 15);

        let card = Card::new(63).unwrap();
        assert!(card.rank == CardRank::TWO);
        assert!(card.suit == CardSuit::SPADES);
        assert!(card.bit() == 63);

        assert!(card == Card::new(63).unwrap());
        assert!(card == CardRank::TWO);
        assert!(card == CardSuit::SPADES);

        assert!(Card::new(0).is_err());
        assert!(Card::new(11).is_err());
        assert!(Card::new(64).is_err());
        assert!(Card::new(255).is_err());

        let low_card = Card::new(12).unwrap();
        let high_card = Card::new(63).unwrap();

        assert!(Card::new(13).unwrap() != low_card);
        assert!(Card::new(13).unwrap() > low_card);
        assert!(low_card < Card::new(13).unwrap());

        assert!(high_card > low_card);
        assert!(low_card < high_card);
        assert!(high_card != low_card);

        assert!(low_card == Card::card(CardRank::THREE, CardSuit::DIAMONDS));
        assert!(high_card == Card::card(CardRank::TWO, CardSuit::SPADES));
    }

    #[test]
    fn card_to_string_test() {
        use super::cards::{Card, CardRank, CardSuit};

        let card_3d = Card::new(12).unwrap();
        let card_3c = Card::new(13).unwrap();
        let card_3h = Card::new(14).unwrap();
        let card_3s = Card::new(15).unwrap();

        assert!(card_3d.to_string() == "3♦".to_string());
        assert!(card_3c.to_string() == "3♣".to_string());
        assert!(card_3h.to_string() == "3♥".to_string());
        assert!(card_3s.to_string() == "3♠".to_string());

        let card_7h = Card::card(CardRank::SEVEN, CardSuit::HEARTS);
        let card_7s = Card::new(card_7h.bit() as usize + 1).unwrap();
        let card_8d = Card::new(card_7h.bit() as usize + 2).unwrap();
        let card_8c = Card::new(card_7h.bit() as usize + 3).unwrap();

        assert!(card_7h.to_string() == "7♥".to_string());
        assert!(card_7s.to_string() == "7♠".to_string());
        assert!(card_8d.to_string() == "8♦".to_string());
        assert!(card_8c.to_string() == "8♣".to_string());

        let card_2d = Card::new(60).unwrap();
        let card_2c = Card::new(61).unwrap();
        let card_2h = Card::new(62).unwrap();
        let card_2s = Card::new(63).unwrap();

        assert!(card_2d.to_string() == "2♦".to_string());
        assert!(card_2c.to_string() == "2♣".to_string());
        assert!(card_2h.to_string() == "2♥".to_string());
        assert!(card_2s.to_string() == "2♠".to_string());
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

        match gs.play(0, 0x4000) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
        match gs.play(1, 0x8000) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
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

        match gs.play(0, 0x4000) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
        match gs.play(1, 0x8000) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
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
    fn have_to_pass() {
        // Have to pass because the player has less card then the board.
        assert!(rules::have_to_pass(0x1F, 0x01));
        assert!(rules::have_to_pass(0x1F, 0x0F));

        // Don't have to pass because the player has equel or more cards then the board.
        assert!(!rules::have_to_pass(0x0, 0x01));
        assert!(!rules::have_to_pass(0x0F, 0x0F));
        assert!(!rules::have_to_pass(0x01, 0x0F));
        assert!(!rules::have_to_pass(0x01, 0x03));
    }
}
