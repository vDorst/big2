pub mod display {
	use crate::big2rules;
	use std::str;

	fn cards_to_utf8(card: u64, card_str: &mut String) {
		//		       0123456789ABCDEF
		let rank_str: Vec<u8> = ".+-3456789TJQKA2".into();
		let rank: usize;
		let suit: u64;

		rank = big2rules::cards::has_rank_idx(card) as usize;
		suit = big2rules::cards::has_suit(card);

		card_str.push_str("\u{1b}[30;107m");

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
		//		       0123456789ABCDEF
		let rank_str: Vec<u8> = ".+-3456789TJQKA2".into();
		let rank: usize;
		let suit: u64;

		rank = big2rules::cards::has_rank_idx(card) as usize;
		suit = big2rules::cards::has_suit(card);

		card_str.push_str("\u{1b}[30;107m");

		card_str.push(rank_str[rank] as char);

		if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("\u{1b}[34m"); }
		if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("\u{1b}[32m"); }
		if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("\u{1b}[31m"); }
		if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("\u{1b}[30m"); }

		if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("d"); }
		if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("c"); }
		if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("h"); }
		if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("s"); }

		card_str.push_str("\u{1b}[49;39m");
	}

	// https://en.wikipedia.org/wiki/Playing_cards_in_Unicode
	fn cards_to_emoji(card: u64, card_str: &mut String) {
		//		       0123456789ABCDEF
		let rank: u64;
		let suit: u64;
		let mut unicode = [0xf0, 0x9f, 0x82, 0x00];
					  //"\u{1F0A0}" =   [f0, 9f, 82, a0]
		rank = big2rules::cards::has_rank_idx(card);
		suit = big2rules::cards::has_suit(card);

		card_str.push_str("\u{1b}[1;30;107m");

		unicode[3] = (rank as u8) & 0xF;
		if rank == big2rules::cards::Rank::ACE { unicode[3] = 1; }
		if rank == big2rules::cards::Rank::TWO { unicode[3] = 2; }


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
			for c in 0..big2rules::deck::NUMBER_OF_CARDS {
				let bit: u64 = (big2rules::deck::START_BIT + c) as u64;
				let dsp_card = card & (1 << bit);
				if dsp_card == 0 { continue; }
				if way == 2  { cards_to_utf8(dsp_card as u64, &mut out_str) };
				if way == 1 { cards_to_plain(dsp_card as u64, &mut out_str) };
				if way == 3 { cards_to_emoji(dsp_card as u64, &mut out_str) };

				out_str.push(' ');
			}
			println!("p{:x}: {}", p, out_str);
	    	}
	}

	pub fn my_cards(cards: u64) {
		let mut out_str = String::from("");
		for c in 0..big2rules::deck::NUMBER_OF_CARDS {
			let bit: u64 = (big2rules::deck::START_BIT + c) as u64;
			let dsp_card = cards & (1 << bit);
			if dsp_card == 0 { continue; }
			cards_to_utf8(dsp_card as u64, &mut out_str);
			out_str.push(' ');
		}
		println!("mycards: {}", out_str);
	}

	pub fn board(gs: &big2rules::GameState) {
		let mut out_str = String::from("");
		let board_hand = gs.board;
		let board_kind = gs.board_score & big2rules::cards::Kind::TYPE;
		let odd_straight: bool = (board_kind == big2rules::cards::Kind::STRAIGHT || board_kind == big2rules::cards::Kind::STRAIGHTFLUSH) && gs.board_score & (0x40 | 0x80) != 0;
		let mut bit: u64 = 1 << 11;
		if odd_straight { bit = 1 << 38; };

		// Clear screen
		print!("\u{1b}[2J");

		for _ in 12..64 {
			if bit == 1 << 63 { bit = 1 << 11; };
			bit <<= 1;
			let card = board_hand & bit;
			if card == 0 { continue; }
			cards_to_utf8(card, &mut out_str);
			out_str.push(' ');
		}
		print!("\r\n  {:>16}: {}/{} - {}             ", "Board", gs.round, gs.rounds, out_str);

		let mut p = gs.i_am_player;
		{
			let mut player = &gs.players[p];
			if p == gs.player_to_act && gs.is_valid_hand { print!("\u{1b}[49;102m"); } else { print!("\u{1b}[40;100m"); }
			print!("[ PLAY ]\u{1b}[49;39m    ");
			if !player.has_passed { print!("\u{1b}[49;101m"); } else { print!("\u{1b}[40;100m"); }
			print!("[ PASS ]\u{1b}[49;39m\r\n\n");
		}

		for _ in 0..gs.players.len() {
			let player = &gs.players[p];
			let mut out_str = String::from("");
			let mut out_sel_str = String::from("");
			let n_cards: usize = {
				let n = player.hand.count_ones() as usize;
				std::cmp::min(n, 13)
			};
			if p == gs.i_am_player {
				for bit in 12..64 {
					let card = player.hand & (1 << bit);
					if card == 0 { continue; }
					if gs.cards_selected & (1 << bit) != 0 {
						cards_to_utf8(card, &mut out_sel_str);
						out_str.push_str("^^");
					} else {
						out_sel_str.push_str("  ");
						cards_to_utf8(card, &mut out_str);
					}
					out_str.push(' ');
					out_sel_str.push(' ');
				}
				print!("                        {}\n", out_sel_str);
			} else {
				out_str = "\u{1b}[30;107m##\u{1b}[49;39m ".to_string().repeat(n_cards);
			}
			let no_cards = ".. ".to_string().repeat(13 - n_cards);
			let score = format!("\u{1b}[33;100m€\u{1b}[44m{:4}\u{1b}[49;39m", player.score);
			let mut passed = String::from("");
			if player.has_passed {
				passed = "\u{1b}[49;101mPASS\u{1b}[49;39m".to_string();
				print!("\u{1b}[49;101m");
			}
			if p == gs.player_to_act { print!("\u{1b}[49;102m"); }
			print!("\r{}.{:>16}\u{1b}[49;39m:", p + 1, player.name);
			print!(" #{:2}", n_cards);
			print!(" {}{}", out_str, no_cards);
			print!(" {}", score);
			print!(" {}\r\n", passed);
			p += 1; if p == gs.players.len() { p = 0; };
		}
	}
}
