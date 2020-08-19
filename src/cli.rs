pub mod display {
    use crate::big2rules;
    use crate::client;

    use std::{
        io::{stdout, Write},
        time::Duration,
    };

    use crossterm::{
        //queue,
        style::{Colorize, Print},
        Result,
        //QueueableCommand,
        terminal::{disable_raw_mode, enable_raw_mode, SetTitle, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetSize},
        event::{poll, read, Event, KeyCode, KeyModifiers},
        cursor::{MoveTo, RestorePosition, SavePosition},
        execute,
    };

    #[derive(PartialEq)]
    pub enum UserEvent {
        NOTHING,
        PLAY,
        PASS,
        READY,
        QUIT,
        CLEAR,
        RESIZE,
        TOGGLECARD1,
        TOGGLECARD2,
        TOGGLECARD3,
        TOGGLECARD4,
        TOGGLECARD5,
        TOGGLECARD6,
        TOGGLECARD7,
        TOGGLECARD8,
        TOGGLECARD9,
        TOGGLECARD10,
        TOGGLECARD11,
        TOGGLECARD12,
        TOGGLECARD13,
    }
    
    // use crossterm::{cursor, execute, queue, style, style::Colorize, Command};

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

    pub fn init(title: &str) -> Result<std::io::Stdout> {
        let mut srn = stdout();

        if enable_raw_mode().is_err() {
            println!("Can't set raw mode");
        }

        execute!(srn, EnterAlternateScreen, SetSize(80, 10))?;
        
        // execute!(stdout, EnableMouseCapture)?;
      
        execute!(srn,
            Clear(ClearType::All),
            SetTitle(&title),
        )?;

        return Ok(srn);
    }

    pub fn close(mut srn: std::io::Stdout) -> Result<()> {
        // execute!(stdout, DisableMouseCapture)?;
        execute!(srn, LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn poll_user_events(gs: &mut big2rules::GameState) -> UserEvent {
        // Poll user events
        let polled_event = poll(Duration::from_millis(100));

        if polled_event.is_err() || !polled_event.unwrap() { return UserEvent::NOTHING };

        // It's guaranteed that read() wont block if `poll` returns `Ok(true)`
        let cli_user_event = read().unwrap();

         match cli_user_event {
            Event::Key(key_event) => return handle_key_events(key_event),
            //Event::Mouse(mouse_event) => return UserEvent::NOTHING,
            Event::Resize(width, height) => return UserEvent::RESIZE,
            _ => return UserEvent::NOTHING,
        }
    }

    fn handle_key_events(event: crossterm::event::KeyEvent) -> UserEvent {
        if event.modifiers != KeyModifiers::NONE { return UserEvent::NOTHING; }

        match event.code {
            KeyCode::Char('r') => return UserEvent::READY,
            KeyCode::Char('`') => return UserEvent::CLEAR,
            KeyCode::Esc       => return UserEvent::QUIT,
            KeyCode::Enter     => return UserEvent::PLAY,
            KeyCode::Char('/') => return UserEvent::PASS,
            KeyCode::Char('1') => return UserEvent::TOGGLECARD1,
            KeyCode::Char('2') => return UserEvent::TOGGLECARD2,
            KeyCode::Char('3') => return UserEvent::TOGGLECARD3,
            KeyCode::Char('4') => return UserEvent::TOGGLECARD4,
            KeyCode::Char('5') => return UserEvent::TOGGLECARD5,
            KeyCode::Char('6') => return UserEvent::TOGGLECARD6,
            KeyCode::Char('7') => return UserEvent::TOGGLECARD7,
            KeyCode::Char('8') => return UserEvent::TOGGLECARD8,
            KeyCode::Char('9') => return UserEvent::TOGGLECARD9,
            KeyCode::Char('0') => return UserEvent::TOGGLECARD10,
            KeyCode::Char('-') => return UserEvent::TOGGLECARD11,
            KeyCode::Char('=') => return UserEvent::TOGGLECARD12,
            KeyCode::Backspace => return UserEvent::TOGGLECARD13,
            _ => return UserEvent::NOTHING,
        }
    }

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

    #[allow(dead_code)]
    pub fn cards(cards: [u64; 4], way: usize) {
        for (p, card) in cards.iter().enumerate() {
            let mut out_str = String::from("");
            for c in 0..big2rules::deck::NUMBER_OF_CARDS {
                let bit: u64 = (big2rules::deck::START_BIT + c) as u64;
                let dsp_card = card & (1 << bit);
                if dsp_card == 0 { continue; }
                if way == 2  { cards_to_utf8(dsp_card as u64, &mut out_str) };

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

    // 0         1         2         3         4         5         6         7
    // 123456789_123456789_123456789_123456789_123456789_123456789_123456789_123456789_
    // 1.         pietje2: # 0                       Delta Score:  €   0  €   0 READY
    // 2.-- Empty Seat --: # 0                       Delta Score:  €   0  €   0 READY
    // 3.-- Empty Seat --: # 0                       Delta Score:  €   0  €   0
    // 4.-- Empty Seat --: # 0                       Delta Score:  €   0  €   0

    // 0         1         2         3         4         5         6         7
    // 123456789_123456789_123456789_123456789_123456789_123456789_123456789_123456789_     
    //         _________pietje2: __ __ __ __ __
    // Rounds: 1/8        Board: 3♦              [ PLAY ]    [ PASS ]
                                                    
    // 3.         pietje2: #13 3♣ 5♥ 6♦ 7♥ 7♠ J♠ Q♠ K♣ K♠ A♣ A♥ A♠ 2♥  €   0 PASS
    // 4.         pietje2: #12 ## ## ## ## ## ## ## ## ## ## ## ## ..  €   0 
    // 1.         pietje2: #13 ## ## ## ## ## ## ## ## ## ## ## ## ##  €   0 
    // 2.         pietje3: #13 ## ## ## ## ## ## ## ## ## ## ## ## ##  €   0 


    pub fn draw_btn_play(gs: &mut big2rules::GameState) -> Result<()> {
        execute!(gs.srn, SavePosition, MoveTo(43, 1))?;

        if gs.sm.your_index == gs.sm.turn && gs.is_valid_hand { 
            execute!(gs.srn, Print("[ PLAY ]".white().on_green()))?;
        } else {
            execute!(gs.srn, Print("[ PLAY ]".white().on_dark_grey()))?;
        }
        execute!(gs.srn, RestorePosition)?;
        Ok(())
    }

    pub fn draw_btn_pass(gs: &mut big2rules::GameState) -> Result<()> {
        execute!(gs.srn, SavePosition, MoveTo(55, 1))?;

        let has_passed_this_cycle = gs.sm.players[gs.sm.your_index as usize].has_passed_this_cycle;

        if has_passed_this_cycle { 
            execute!(gs.srn, Print("[X] PASS".white().on_dark_grey()))?;
        } else {
            if gs.auto_pass {
                execute!(gs.srn, Print("[v] PASS".white().on_blue()))?;
            } else {
                execute!(gs.srn, Print("[ ] PASS".white().on_red()))?;
            }
        }
        execute!(gs.srn, RestorePosition)?;
        Ok(())
    }

    pub fn board(gs: &mut big2rules::GameState) -> Result<()> {
        let mut out_str = String::with_capacity(64);
        let board_kind = gs.board_score & big2rules::cards::Kind::TYPE;
        let odd_straight: bool = (board_kind == big2rules::cards::Kind::STRAIGHT || board_kind == big2rules::cards::Kind::STRAIGHTFLUSH) && gs.board_score & (0x40 | 0x80) != 0;
        let mut bit: u64 = 1 << 11;
        if odd_straight { bit = 1 << 38; };

        execute!(gs.srn, MoveTo(0, 0), Print("+".white().on_red()))?;

        if gs.sm.action.action_type == client::StateMessageActionType::PASS {
            let name = name_from_muon_string16(&gs.sm.players[gs.sm.action.player as usize].name);
            let s = format!("{:>16}: ", name);
            execute!(gs.srn, MoveTo(9, 1), Print(&s), Print("PASSED".white().on_dark_grey()))?;          
        }

        if gs.sm.action.action_type == client::StateMessageActionType::PLAY {
            let name = name_from_muon_string16(&gs.sm.players[gs.sm.action.player as usize].name);
            let s = format!("{:>16}: ", name);
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
            execute!(gs.srn, MoveTo(9, 2), Print(&s), Print(card_str))?;  
        }

        execute!(gs.srn, MoveTo(0, 1), Print(format!("Rounds: {}/{}", gs.sm.round, gs.sm.num_rounds)))?;
        let cards = client::client::muon_inline8_to_card(&gs.sm.board);
        for _ in 12..64 {
            if bit == 1 << 63 { bit = 1 << 11; };
            bit <<= 1;
            let card = cards & bit;
            if card == 0 { continue; }
            cards_to_utf8(card, &mut out_str);
            out_str.push(' ');
        }
        execute!(gs.srn, MoveTo(20, 1), Print("Board: "), Print(out_str))?;

        let mut p = gs.sm.your_index;
        if p < 0 || p > 3 {
            p = 0;
        }

        if gs.sm.turn == -1 {
            for _ in 0..4 {
                let player = &gs.sm.players[p as usize];
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
            return Ok(()); 
        }

        if p >= 0 {
            draw_btn_play(gs)?;
            draw_btn_pass(gs)?;
        }

        for row in 0..4 {
            let player = &gs.sm.players[p as usize];
            let mut name = name_from_muon_string16(&player.name);
            if name == "" { name = String::from("-- Empty Seat --"); }
            let s = format!("{:>16}: ", name);

            let mut out_str = String::from("");
            let mut out_sel_str = String::from("");
            let n_cards: usize = player.num_cards as usize;

            let has_passed =  player.has_passed_this_cycle;
            let player_score = player.score;

            if p == gs.sm.your_index {
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
                execute!(gs.srn,
                    MoveTo(25, 2),
                    Print(out_sel_str),
                )?;
            } else {
                out_str = format!("{}##{} ", COL_CARD_BACK, COL_NORMAL).repeat(n_cards);
            }
            let no_cards = ".. ".to_string().repeat(13 - n_cards);
            if p == gs.sm.turn { print!("{}", COL_PLAYER_ACT); }

            execute!(gs.srn,
                MoveTo(0, 3+row),
                Print(format!("{}.", p+1)),
                Print(&s),
                Print(format!(" #{:2}", n_cards)),
                Print(format!(" {}{}", out_str, no_cards)),
                Print(format!(" {}", score_str(player_score))),
            )?;

            if has_passed {
                execute!(gs.srn,
                    MoveTo(71, 3+row),
                    Print("PASS".white().on_dark_grey() ),
                )?;
            }
            p += 1; if p == 4 { p = 0; };
        }

        execute!(gs.srn,
            MoveTo(0, 7),
            Print(format!("Debug: B {:x} BS {} H {:x} HS {} s {:x} CNT {}\r\n", gs.board, gs.board_score, gs.hand,
            gs.hand_score, gs.cards_selected, gs.counter))
        )?;
        Ok(())
    }
}
