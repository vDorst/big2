pub mod display {
	use crate::big2rules;
	use std::str;

	fn cards_to_utf8(card: u64, card_str: &mut String) {
		//                       0123456789ABCDEF
		let rank_str: Vec<u8> = ".+-3456789TJQKA2".into();
		let rank: usize;
		let suit: u64;

		rank = big2rules::cards::has_rank_idx(card) as usize;
		suit = big2rules::cards::has_suit(card);

		card_str.push_str("\u{1b}[1;30;107m");

		card_str.push(rank_str[rank] as char);

		if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("\u{1b}[34m"); }
		if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("\u{1b}[32m"); }
		if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("\u{1b}[31m"); }
		if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("\u{1b}[30m"); }

		if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("\u{2666}"); }
		if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("\u{2663}"); }
		if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("\u{2665}"); }
		if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("\u{2660}"); }
		
		card_str.push_str("\u{1b}[0;49;39m");
	}

	fn cards_to_plain(card: u64, card_str: &mut String) {
		//                       0123456789ABCDEF
		let rank_str: Vec<u8> = ".+-3456789TJQKA2".into();
		let rank: usize;
		let suit: u64;

		rank = big2rules::cards::has_rank_idx(card) as usize;
		suit = big2rules::cards::has_suit(card);

		card_str.push_str("\u{1b}[1;30;107m");

		card_str.push(rank_str[rank] as char);

		if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("\u{1b}[34m"); }
		if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("\u{1b}[32m"); }
		if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("\u{1b}[31m"); }
		if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("\u{1b}[30m"); }

		if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("d"); }
		if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("c"); }
		if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("h"); }
		if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("s"); }

		card_str.push_str("\u{1b}[0;49;39m");
	}

	// https://en.wikipedia.org/wiki/Playing_cards_in_Unicode
	fn cards_to_emoji(card: u64, card_str: &mut String) {
		//                       0123456789ABCDEF
		let rank: u64;
		let suit: u64;
		let mut unicode = [0xf0, 0x9f, 0x82, 0x00];
					  //"\u{1F0A0}" =   [f0, 9f, 82, a0]
		rank = big2rules::cards::has_rank_idx(card);
		suit = big2rules::cards::has_suit(card);

		card_str.push_str("\u{1b}[1;30;107m");

		unicode[3] = (rank as u8) & 0xF;
		if (rank == big2rules::cards::Rank::ACE) { unicode[3] = 1; }
		if (rank == big2rules::cards::Rank::TWO) { unicode[3] = 2; }


		if suit == big2rules::cards::Kind::DIAMONDS { unicode[3] |= 0xC0; }
		if suit == big2rules::cards::Kind::CLUBS    { unicode[3] |= 0xD0; }
		if suit == big2rules::cards::Kind::HEARTS   { unicode[3] |= 0xB0; }
		if suit == big2rules::cards::Kind::SPADES   { unicode[3] |= 0xA0; }
		
		
		if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("\u{1b}[34m"); }
		if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("\u{1b}[32m"); }
		if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("\u{1b}[31m"); }
		if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("\u{1b}[30m"); }

		let s = str::from_utf8(&unicode).unwrap();

		println!("{}", s);
		//card_str.push(s);
		
		card_str.push_str("\u{1b}[0;49;39m");
	}

	pub fn cards(cards: [u64; 4], way: usize) {
		for (p, card) in cards.iter().enumerate() {
			let mut out_str = String::from("");
			for b in 0..big2rules::deck::NUMBER_OF_CARDS {
				let dsp_card = card & (1 << (b + big2rules::deck::START_BIT));
				if dsp_card == 0 { continue; }
				if way == 2  { cards_to_utf8(dsp_card, &mut out_str) };
				if way == 1 { cards_to_plain(dsp_card, &mut out_str) };
				if way == 3 { cards_to_emoji(dsp_card, &mut out_str) };
				
				out_str.push(' ');
			}
			println!("p{:x}: {}", p, out_str);
	    	}
	}
}
