pub mod display {
	use crate::big2rules;

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

	pub fn cards(cards: [u64; 4]) {
		for p in 0..cards.len() {
			let card = cards[p];
			let mut out_str = String::from("");
			for r in big2rules::RANKS.iter() {
				for s in big2rules::SUITS.iter() {
					if (card & big2rules::deck::card_encode(*r, *s)) == 0 { continue; }
					cards_to_str(u8::from((r << 4) + s), &mut out_str);
					out_str.push(' ');
				}
			}
			println!("p{:x}: {:64b}: {}", p, card, out_str);
	    	}
	}
}
