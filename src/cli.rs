pub mod display {
    use crate::big2rules;
    use crate::client;
    use std::str;

    // https://en.wikipedia.org/wiki/ANSI_escape_code
    const COL_PASSED:          &str = "\u{1b}[97;100m"; // White on Grey
    const COL_NORMAL:          &str = "\u{1b}[0m";  // White on black
    const COL_PLAYER_ACT:      &str = "\u{1b}[97;42m"; // White on Green
    const COL_PLAYER_PASSED:   &str = "\u{1b}[97;100m";
    const COL_CARD_BACK:       &str = "\u{1b}[30;47m";
    const COL_BTN_DIS:         &str = "\u{1b}[97;100m";
    const COL_BTN_PASS_AUTO:   &str = "\u{1b}[97;104m"; // white on blue
    const COL_BTN_PASS_ACTIVE: &str = "\u{1b}[97;101m"; // white on red

    const COL_SCORE_POS:       &str = "\u{1b}[97;42m"; // White on Green
    const COL_SCORE_NEG:       &str = "\u{1b}[97;41m"; // White on Red
    const COL_SCORE_ZERO:      &str = "\u{1b}[97;100m"; // White on Grey

    const COL_DIAMONDS:        &str = "\u{1b}[34m"; // White on Grey
    const COL_CLUBS:           &str = "\u{1b}[32m";
    const COL_HEARTS:          &str = "\u{1b}[31m";
    const COL_SPADES:          &str = "\u{1b}[30m";

    fn cards_to_utf8(card: u64, card_str: &mut String) {
        //             0123456789ABCDEF
        let rank_str: Vec<u8> = ".+-3456789TJQKA2".into();
        let rank: usize;
        let suit: u64;

        rank = big2rules::cards::has_rank_idx(card) as usize;
        suit = big2rules::cards::has_suit(card);

        card_str.push_str(COL_CARD_BACK);

        card_str.push(rank_str[rank] as char);

        if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str(COL_DIAMONDS); }
        if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str(COL_CLUBS); }
        if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str(COL_HEARTS); }
        if suit == big2rules::cards::Kind::SPADES   { card_str.push_str(COL_SPADES); }

        if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("\u{2666}"); }
        if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("\u{2663}"); }
        if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("\u{2665}"); }
        if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("\u{2660}"); }

        card_str.push_str(COL_NORMAL);
    }

    fn cards_to_plain(card: u64, card_str: &mut String) {
        //             0123456789ABCDEF
        let rank_str: Vec<u8> = ".+-3456789TJQKA2".into();
        let rank: usize;
        let suit: u64;

        rank = big2rules::cards::has_rank_idx(card) as usize;
        suit = big2rules::cards::has_suit(card);

        card_str.push_str("\u{1b}[30;107m");

        card_str.push(rank_str[rank] as char);

        if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str(COL_DIAMONDS); }
        if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str(COL_CLUBS); }
        if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str(COL_HEARTS); }
        if suit == big2rules::cards::Kind::SPADES   { card_str.push_str(COL_SPADES); }

        if suit == big2rules::cards::Kind::DIAMONDS { card_str.push_str("d"); }
        if suit == big2rules::cards::Kind::CLUBS    { card_str.push_str("c"); }
        if suit == big2rules::cards::Kind::HEARTS   { card_str.push_str("h"); }
        if suit == big2rules::cards::Kind::SPADES   { card_str.push_str("s"); }

        card_str.push_str("\u{1b}[49;39m");
    }

    // https://en.wikipedia.org/wiki/Playing_cards_in_Unicode
    fn cards_to_emoji(card: u64, card_str: &mut String) {
        //             0123456789ABCDEF
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    pub fn name_from_muon_string16(sm_name: &client::MuonString16) -> String {
        let mut s = String::with_capacity(16);
        if sm_name.count < 0 || sm_name.count > 16 {
            s.push_str("Invalid string");
            return s;
        }

        let cnt: usize = sm_name.count as usize;
        let s_ret = String::from_utf8(sm_name.data[..cnt].to_vec());
        match s_ret {
            Err(_) => s.push_str("Can't convert"),
            Ok(st) => s = st,
        }
        return s;
    }

    fn score_str(score: i32) -> String {
        let mut buf = String::with_capacity(32);
        if score < 0 { buf.push_str(COL_SCORE_NEG); }
        if score == 0 { buf.push_str(COL_SCORE_ZERO); }
        if score > 0 { buf.push_str(COL_SCORE_POS); }
        buf.push_str(&format!("€{:4}", score));
        buf.push_str(COL_NORMAL);
        return buf;
    }

    pub fn board(gs: &big2rules::GameState) {
        let sm = &gs.sm;
        let mut out_str = String::with_capacity(64);
        let board_kind = gs.board_score & big2rules::cards::Kind::TYPE;
        let odd_straight: bool = (board_kind == big2rules::cards::Kind::STRAIGHT || board_kind == big2rules::cards::Kind::STRAIGHTFLUSH) && gs.board_score & (0x40 | 0x80) != 0;
        let mut bit: u64 = 1 << 11;
        if odd_straight { bit = 1 << 38; };

        // Clear screen
        print!("\u{1b}[2J");

        match gs.sm.action.action_type {
        client::StateMessageActionType::PASS => {
            let name = name_from_muon_string16(&gs.sm.players[gs.sm.action.player as usize].name);
            print!("\r\n        {1:>16}: {0}PASSED{2}", COL_PASSED, name, COL_NORMAL);
        },
        client::StateMessageActionType::PLAY => {
            let name = name_from_muon_string16(&gs.sm.players[gs.sm.action.player as usize].name);
            let cards = client::client::muon_inline8_to_card(&gs.sm.action.cards);
            let mut card_str = String::from("");
            for _ in 12..64 {
                if bit == 1 << 63 { bit = 1 << 11; };
                bit <<= 1;
                let card = cards & bit;
                if card == 0 { continue; }
                cards_to_utf8(card, &mut card_str);
                card_str.push(' ');
            }
            print!("\r\n        {:>16}: {}", name, card_str);
        },
        _ => print!("\r\n"),
        }

        let cards = client::client::muon_inline8_to_card(&gs.sm.board);
        for _ in 12..64 {
            if bit == 1 << 63 { bit = 1 << 11; };
            bit <<= 1;
            let card = cards & bit;
            if card == 0 { continue; }
            cards_to_utf8(card, &mut out_str);
            out_str.push(' ');
        }
        print!("\r\nRounds: {}/{} {:>12}: {}             ", gs.sm.round, gs.sm.num_rounds, "Board", out_str);

        let mut p = sm.your_index;
        if p < 0 || p > 3 {
            p = 0;
        }

        if gs.sm.turn == -1 {
            for _ in 0..4 {
                let player = &sm.players[p as usize];
                let mut name = name_from_muon_string16(&player.name);
                if name == "" { name = String::from("-- Empty Seat --"); }
                let n_cards: usize = player.num_cards as usize;
                print!("\r{}.{:>16}{}:", p + 1, name, COL_NORMAL);
                print!(" #{:2}", n_cards);
                print!("{:>34}: ", "Delta Score");
                print!(" {}  {}", score_str(player.delta_score), score_str(player.score));
                if player.is_ready {
                    print!(" {}READY{}", COL_BTN_PASS_AUTO, COL_NORMAL);
                }
                print!("\r\n");
                p += 1; if p == 4 { p = 0; };
            }
            // println!("4.           Nick3: #11 ## ## ## ## ## ## ## ## ## ## ## .. ..  €   0 PASS^");
            return;
        }

        if p >= 0 {
            let player = &sm.players[p as usize];
            if p == sm.turn && gs.is_valid_hand { print!("\u{1b}[49;102m"); } else { print!("{}", COL_BTN_DIS); }
            print!("[ PLAY ]\u{1b}[49;39m    ");

            let col_pass_btn: &str;
            if !player.has_passed_this_cycle {
                if gs.auto_pass { col_pass_btn = COL_BTN_PASS_AUTO; } else { col_pass_btn = COL_BTN_PASS_ACTIVE; }
            } else { col_pass_btn = COL_BTN_DIS; };
            print!("{}[ PASS ]{}\r\n\n", col_pass_btn, COL_NORMAL);
        }

        for _ in 0..4 {
            let player = &sm.players[p as usize];
            let mut out_str = String::from("");
            let mut out_sel_str = String::from("");
            let n_cards: usize = player.num_cards as usize;

            if p == sm.your_index {
                let cards = client::client::muon_inline16_to_card(&gs.sm.your_hand);
                for bit in 12..64 {
                    let card = cards & (1 << bit);
                    if card == 0 { continue; }
                    if gs.cards_selected & card  != 0 {
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
                out_str = format!("{}##{} ", COL_CARD_BACK, COL_NORMAL).repeat(n_cards);
            }
            let no_cards = ".. ".to_string().repeat(13 - n_cards);
            let mut passed = String::from("");
            if player.has_passed_this_cycle {
                passed = format!("{}PASS{}", COL_PASSED, COL_NORMAL);
                print!("{}", COL_PLAYER_PASSED);
            }
            if p == gs.sm.turn { print!("{}", COL_PLAYER_ACT); }
            let mut name = name_from_muon_string16(&player.name);
            if name == "" { name = String::from("-- Empty Seat --"); }
            print!("\r{}.{:>16}{}:", p + 1, name, COL_NORMAL);
            print!(" #{:2}", n_cards);
            print!(" {}{}", out_str, no_cards);
            print!(" {}", score_str(player.score));
            print!(" {}\r\n", passed);
            p += 1; if p == 4 { p = 0; };
        }
    }
}
