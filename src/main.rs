mod big2rules;
mod cli;

fn main() {
	let cards: [u64; 4] = big2rules::deck::deal();
	// let mut players: [Player; 4];

//	for value in big2rules::RANKS.iter() {
//		for suit in SUITS.iter() {
//			let bit = card_encode(*value, *suit);
//			println!("r{:02x} s{:02x} b{:64b}", value, suit, bit);
//		}
//	}

	cli::display::cards(cards);
	for p in 0..cards.len() {
		big2rules::rules::get_numbers(cards[p]);
		//println!("P{}: Quads: {}", p, Rules::hasQuads(cards[p]));
	}
}

