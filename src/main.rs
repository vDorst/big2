mod big2rules;
use rand::Rng;

fn cards_to_str(card: u8, card_str: &mut String) {
	//                       0123456789ABCDEF
	let rank_str: Vec<u8> = ".+-3456789TJQKA2".into();
	let rank: usize;
	let suit: u8;

	rank = ((card >> 4) & 0xF).into();
	suit = (card & 0x3).into();

	// Suit 0: Diamon: Blue
	//      1: Clubs:  Green
	//      2: hearts: Red
	//      3: Spades: Black

	card_str.push_str("\u{1b}[1;30;107m");
	
	card_str.push(rank_str[rank] as char);

	//if suit == big2rules::SUITS[0] { card_str.push_str("\u{1b}[106m\u{1b}[30m"); }
	//if suit == big2rules::SUITS[1] { card_str.push_str("\u{1b}[102m\u{1b}[30m"); }
	//if suit == big2rules::SUITS[2] { card_str.push_str("\u{1b}[101m\u{1b}[30m"); }
	//if suit == big2rules::SUITS[3] { card_str.push_str("\u{1b}[107m\u{1b}[30m"); }

	if suit == big2rules::SUITS[0] { card_str.push_str("\u{1b}[34m"); }
	if suit == big2rules::SUITS[1] { card_str.push_str("\u{1b}[32m"); }
	if suit == big2rules::SUITS[2] { card_str.push_str("\u{1b}[31m"); }
	if suit == big2rules::SUITS[3] { card_str.push_str("\u{1b}[30m"); }

	if suit == big2rules::SUITS[0] { card_str.push_str("\u{2666}"); }
	if suit == big2rules::SUITS[1] { card_str.push_str("\u{2663}"); }    		
	if suit == big2rules::SUITS[2] { card_str.push_str("\u{2665}"); } 
	if suit == big2rules::SUITS[3] { card_str.push_str("\u{2660}"); }
	
	card_str.push_str("\u{1b}[0;49;39m");
}

fn diplay_cards(cards: [u64; 4]) {
	for p in 0..cards.len() {
		let card = cards[p];
		let mut out_str = String::from("");
		for r in big2rules::RANKS.iter() {
			for s in big2rules::SUITS.iter() {
				if (card & card_encode(*r, *s)) == 0 { continue; }
				cards_to_str(u8::from((r << 4) + s), &mut out_str);
				out_str.push(' ');
			}
		}
		println!("p{:x}: {:64b}: {}", p, card, out_str);
    	}
}

pub fn gen_deck() -> [u64; 4] {
	let mut rng = rand::thread_rng();
	let mut deck = Vec::<u8>::with_capacity(52);
	let mut o: usize;
	let cards: [u64; 4];
	
	// Create Cards
	for s in big2rules::SUITS.iter() {
		for r in big2rules::RANKS.iter() {
			deck.push(deck_encode(*r, *s));
		}
	}

	assert_eq!(deck.len(), 52, "Strange card count must be 52!");
	
	// Randomize/shuffle the cards
	for _ in 0..256 {
		for c in 0..deck.len() {
			o = rng.gen_range(0, deck.len());
			deck.swap(c, o);
		}
	}

	// Deal cards
	cards = deal_cards(deck);
	assert!((cards[0] | cards[1] | cards[2] | cards[3]) != 0xFFFF_FFFF_FFFF_0000u64);
	return cards;
}

fn deck_encode(value: u8, suit: u8) -> u8 {
	return (value << 4) + suit;
}

fn deck_decode(deckvalue: u8) -> (u8, u8) {
	let value = (deckvalue >> 4) & 0xF;
	let suit  = deckvalue & 0x3;
	return (value, suit);
}

fn card_encode(value: u8, suit: u8) -> u64 {
	return (1 << (u64::from(value) << 2)) << u64::from(suit);
}

fn deal_cards(deck: Vec<u8>) -> [u64; 4] {
	let mut player_cards: [u64; 4] = [0,0,0,0];
	let mut p: usize = 0;
	let mut c: usize = 0;
	
	for r in deck {
		let (value, suit) = deck_decode(r);
		let bit = card_encode(value, suit);
		player_cards[p] |= bit;
		// println!("r{:02x} v{:02x} s{:02x} b{:16x} - {:16x?} + {:64b}", r, value, suit, bit, player_cards[p], player_cards[p]);
		c += 1;
		if c == 13 {
			// println!("p{:x} {:#08x?}", p, player_cards[p]);
			assert!(player_cards[p].count_ones() == 13);
			c = 0;
			p += 1;
		}
	}
	return player_cards;
}

fn main() {
	let cards: [u64; 4] = gen_deck();
	// let mut players: [Player; 4];

//	for value in big2rules::RANKS.iter() {
//		for suit in SUITS.iter() {
//			let bit = card_encode(*value, *suit);
//			println!("r{:02x} s{:02x} b{:64b}", value, suit, bit);
//		}
//	}

	diplay_cards(cards);
	for p in 0..cards.len() {
		big2rules::rules::get_numbers(cards[p]);
		//println!("P{}: Quads: {}", p, Rules::hasQuads(cards[p]));
	}
}
