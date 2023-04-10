pub mod display {
    use crate::{big2rules, network};
    use log::trace;

    use std::{io::stdout, time::Duration};

    use crossterm::{
        cursor::{MoveTo, RestorePosition, SavePosition},
        event::{
            poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
            MouseButton, MouseEvent, MouseEventKind,
        },
        execute,
        //queue,
        style::{Print, ResetColor, Stylize},
        //QueueableCommand,
        terminal::{
            disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
            LeaveAlternateScreen, SetSize, SetTitle,
        },
        Result,
    };

    #[derive(PartialEq)]
    pub enum UserEvent {
        Nothing,
        Play,
        Pass,
        Ready,
        Quit,
        Clear,
        Resize,
        ToggleCard1,
        ToggleCard2,
        ToggleCard3,
        ToggleCard4,
        ToggleCard5,
        ToggleCard6,
        ToggleCard7,
        ToggleCard8,
        ToggleCard9,
        ToggleCard10,
        ToggleCard11,
        ToggleCard12,
        ToggleCard13,
    }

    // https://en.wikipedia.org/wiki/ANSI_escape_code
    const COL_NORMAL: &str = "\u{1b}[0m"; // White on black

    const COL_CARD_BACK: &str = "\u{1b}[30;47m";

    const COL_BTN_PASS_AUTO: &str = "\u{1b}[97;104m"; // white on blue

    const COL_SCORE_POS: &str = "\u{1b}[97;42m"; // White on Green
    const COL_SCORE_NEG: &str = "\u{1b}[97;41m"; // White on Red
    const COL_SCORE_ZERO: &str = "\u{1b}[97;100m"; // White on Grey

    const COL_DIAMONDS: &str = "\u{1b}[34m"; // White on Grey
    const COL_CLUBS: &str = "\u{1b}[32m";
    const COL_HEARTS: &str = "\u{1b}[31m";
    const COL_SPADES: &str = "\u{1b}[30m";

    pub fn clear(srn: &mut std::io::Stdout) -> Result<()> {
        execute!(srn, Clear(ClearType::All))
    }

    pub fn titlebar(srn: &mut std::io::Stdout, title: &str) -> Result<()> {
        execute!(srn, SetTitle(&title))
    }

    pub fn init(title: &str) -> Result<std::io::Stdout> {
        let mut srn = stdout();

        execute!(
            srn,
            EnterAlternateScreen,
            EnableMouseCapture,
            SetSize(80, 10),
            Clear(ClearType::All),
            //SetTitle(&title),
        )?;

        titlebar(&mut srn, title)?;

        enable_raw_mode()?;

        Ok(srn)
    }

    pub fn close(mut srn: std::io::Stdout) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            srn,
            ResetColor,
            DisableMouseCapture,
            LeaveAlternateScreen,
            Clear(ClearType::All),
            Print("Bye".white().on_dark_grey()),
        )
    }

    #[must_use]
    pub fn poll_user_events() -> UserEvent {
        // Poll user events
        let polled_event = poll(Duration::from_millis(100));

        let cli_user_event = match polled_event {
            Err(_) | Ok(false) => return UserEvent::Nothing,
            // It's guaranteed that read() wont block if `poll` returns `Ok(true)`
            Ok(true) => read().expect("Read should not"),
        };

        match cli_user_event {
            Event::Key(key_event) => handle_key_events(key_event),
            Event::Mouse(mouse_event) => handle_mouse_events(mouse_event),
            Event::Resize(_, _) => UserEvent::Resize,
            Event::FocusGained | Event::FocusLost | Event::Paste(_) => UserEvent::Nothing,
        }
    }

    fn handle_mouse_events(event: MouseEvent) -> UserEvent {
        if let MouseEventKind::Down(MouseButton::Right) = event.kind {
            return UserEvent::Clear;
        }
        if let MouseEventKind::Down(MouseButton::Left) = event.kind {
            let x = event.column;
            let y = event.row;
            if y == 3 || y == 2 {
                if x == 24 || x == 25 {
                    return UserEvent::ToggleCard1;
                }
                if x == 27 || x == 28 {
                    return UserEvent::ToggleCard2;
                }
                if x == 30 || x == 31 {
                    return UserEvent::ToggleCard3;
                }
                if x == 33 || x == 34 {
                    return UserEvent::ToggleCard4;
                }
                if x == 36 || x == 37 {
                    return UserEvent::ToggleCard5;
                }
                if x == 39 || x == 40 {
                    return UserEvent::ToggleCard6;
                }
                if x == 42 || x == 43 {
                    return UserEvent::ToggleCard7;
                }
                if x == 45 || x == 46 {
                    return UserEvent::ToggleCard8;
                }
                if x == 48 || x == 49 {
                    return UserEvent::ToggleCard9;
                }
                if x == 51 || x == 52 {
                    return UserEvent::ToggleCard10;
                }
                if x == 54 || x == 55 {
                    return UserEvent::ToggleCard11;
                }
                if x == 57 || x == 58 {
                    return UserEvent::ToggleCard12;
                }
                if x == 60 || x == 61 {
                    return UserEvent::ToggleCard13;
                }
            }
            if y == 1 {
                if (43..=49).contains(&x) {
                    return UserEvent::Play;
                }
                if (55..=62).contains(&x) {
                    return UserEvent::Pass;
                }
                if (66..=74).contains(&x) {
                    return UserEvent::Ready;
                }
            }
            trace!("{:?}", event);
        }
        UserEvent::Nothing
    }

    fn handle_key_events(event: crossterm::event::KeyEvent) -> UserEvent {
        if event.modifiers != KeyModifiers::NONE {
            return UserEvent::Nothing;
        }

        match event.code {
            KeyCode::Char('r') => UserEvent::Ready,
            KeyCode::Char('`') => UserEvent::Clear,
            KeyCode::Esc => UserEvent::Quit,
            KeyCode::Enter => UserEvent::Play,
            KeyCode::Char('/') => UserEvent::Pass,
            KeyCode::Char('1') => UserEvent::ToggleCard1,
            KeyCode::Char('2') => UserEvent::ToggleCard2,
            KeyCode::Char('3') => UserEvent::ToggleCard3,
            KeyCode::Char('4') => UserEvent::ToggleCard4,
            KeyCode::Char('5') => UserEvent::ToggleCard5,
            KeyCode::Char('6') => UserEvent::ToggleCard6,
            KeyCode::Char('7') => UserEvent::ToggleCard7,
            KeyCode::Char('8') => UserEvent::ToggleCard8,
            KeyCode::Char('9') => UserEvent::ToggleCard9,
            KeyCode::Char('0') => UserEvent::ToggleCard10,
            KeyCode::Char('-') => UserEvent::ToggleCard11,
            KeyCode::Char('=') => UserEvent::ToggleCard12,
            KeyCode::Backspace => UserEvent::ToggleCard13,
            KeyCode::Char('d') => UserEvent::Resize,
            _ => UserEvent::Nothing,
        }
    }

    fn cards_to_utf8(card: u64, card_str: &mut String) {
        //             0123456789ABCDEF
        let rank_str: Vec<u8> = ".+-3456789TJQKA2".into();

        let rank: usize = big2rules::cards::has_rank_idx(card) as usize;
        let suit: u64 = big2rules::cards::has_suit(card);

        card_str.push_str(COL_CARD_BACK);

        card_str.push(rank_str[rank] as char);

        if suit == big2rules::cards::Kind::DIAMONDS {
            card_str.push_str(COL_DIAMONDS);
        }
        if suit == big2rules::cards::Kind::CLUBS {
            card_str.push_str(COL_CLUBS);
        }
        if suit == big2rules::cards::Kind::HEARTS {
            card_str.push_str(COL_HEARTS);
        }
        if suit == big2rules::cards::Kind::SPADES {
            card_str.push_str(COL_SPADES);
        }

        if suit == big2rules::cards::Kind::DIAMONDS {
            card_str.push('\u{2666}');
        }
        if suit == big2rules::cards::Kind::CLUBS {
            card_str.push('\u{2663}');
        }
        if suit == big2rules::cards::Kind::HEARTS {
            card_str.push('\u{2665}');
        }
        if suit == big2rules::cards::Kind::SPADES {
            card_str.push('\u{2660}');
        }

        card_str.push_str(COL_NORMAL);
    }

    #[allow(dead_code)]
    pub fn cards(cards: [u64; 4], way: usize) {
        for (p, card) in cards.iter().enumerate() {
            let mut out_str = String::new();
            for c in 0..big2rules::deck::NUMBER_OF_CARDS {
                let bit: u64 = u64::from(big2rules::deck::START_BIT + c);
                let dsp_card = card & (1 << bit);
                if dsp_card == 0 {
                    continue;
                }
                if way == 2 {
                    cards_to_utf8(dsp_card, &mut out_str);
                };

                out_str.push(' ');
            }
            println!("p{p:x}: {out_str}");
        }
    }

    #[allow(dead_code)]
    pub fn my_cards(cards: u64) {
        let mut out_str = String::new();
        for c in 0..big2rules::deck::NUMBER_OF_CARDS {
            let bit: u64 = u64::from(big2rules::deck::START_BIT + c);
            let dsp_card = cards & (1 << bit);
            if dsp_card == 0 {
                continue;
            }
            cards_to_utf8(dsp_card, &mut out_str);
            out_str.push(' ');
        }
        println!("mycards: {out_str}");
    }

    fn score_str(score: i32) -> String {
        let mut buf = String::with_capacity(32);
        if score < 0 {
            buf.push_str(COL_SCORE_NEG);
        }
        if score == 0 {
            buf.push_str(COL_SCORE_ZERO);
        }
        if score > 0 {
            buf.push_str(COL_SCORE_POS);
        }
        buf.push_str(&format!("€{score:4}"));
        buf.push_str(COL_NORMAL);
        buf
    }

    // 0         1         2         3         4         5         6         7
    // 123456789_123456789_123456789_123456789_123456789_123456789_123456789_123456789_
    // 1.         pietje2: # 0                       Delta Score:  €   0  €   0 READY
    // 2.-- Empty Seat --: # 0                       Delta Score:  €   0  €   0 READY
    // 3.-- Empty Seat --: # 0                       Delta Score:  €   0  €   0
    // 4.-- Empty Seat --: # 0                       Delta Score:  €   0  €   0

    // 0         1         2         3         4         5         6         7
    // _123456789_123456789_123456789_123456789_123456789_123456789_123456789_123456789_
    //         _________pietje2: __ __ __ __ __
    // Rounds: 1/8        Board: 3♦              [ PLAY ]    [ PASS ]    [ ] READY
    //                                                             2♥
    // 3.         pietje2: #13 3♣ 5♥ 6♦ 7♥ 7♠ J♠ Q♠ K♣ K♠ A♣ A♥ A♠ ^^  €   0 PASS
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

    pub fn draw_btn_ready(gs: &mut big2rules::GameState) -> Result<()> {
        execute!(gs.srn, SavePosition, MoveTo(66, 1))?;

        if gs.i_am_ready {
            execute!(gs.srn, Print("[x] READY".white().on_dark_grey()))?;
        } else {
            execute!(gs.srn, Print("[ ] READY".white().on_dark_blue()))?;
        }
        execute!(gs.srn, RestorePosition)?;
        Ok(())
    }

    pub fn draw_btn_pass(gs: &mut big2rules::GameState) -> Result<()> {
        execute!(gs.srn, SavePosition, MoveTo(55, 1))?;

        let has_passed_this_cycle = gs.sm.players[gs.sm.your_index as usize].has_passed_this_cycle;

        if has_passed_this_cycle {
            execute!(gs.srn, Print("[X] PASS".white().on_dark_grey()))?;
        } else if gs.auto_pass {
            execute!(gs.srn, Print("[v] PASS".white().on_blue()))?;
        } else {
            execute!(gs.srn, Print("[ ] PASS".white().on_red()))?;
        }
        execute!(gs.srn, RestorePosition)?;
        Ok(())
    }

    #[must_use]
    pub fn cards_str(cards: u64) -> String {
        let mut bit: u64 = 1 << 11;
        let score = big2rules::rules::score_hand(cards);
        let board_kind = score & big2rules::cards::Kind::TYPE;
        let odd_straight: bool = (board_kind == big2rules::cards::Kind::STRAIGHT
            || board_kind == big2rules::cards::Kind::STRAIGHTFLUSH)
            && score & (0x40 | 0x80) != 0;
        if odd_straight {
            bit = 1 << 38;
        };
        let mut card_str = String::with_capacity(64);
        for _ in 12..64 {
            if bit == 1 << 63 {
                bit = 1 << 11;
            };
            bit <<= 1;
            let card = cards & bit;
            if card == 0 {
                continue;
            }
            cards_to_utf8(card, &mut card_str);
            card_str.push(' ');
        }
        card_str
    }

    pub fn board(gs: &mut big2rules::GameState) -> Result<()> {
        let name = gs.sm.players[gs.sm.action.player as usize].name.as_string();
        let s = format!("{name:>16}: ");
        if gs.sm.action.action_type == network::StateMessageActionType::Pass {
            execute!(
                gs.srn,
                MoveTo(9, 0),
                Print(&s),
                Print("PASSED".white().on_dark_grey())
            )?;
        } else if gs.sm.action.action_type == network::StateMessageActionType::Play {
            let cards = gs.sm.action.cards.as_card().expect("Should not crash!");
            let card_str = cards_str(cards);
            execute!(gs.srn, MoveTo(9, 0), Print(&s), Print(card_str))?;
        } else {
            execute!(gs.srn, MoveTo(9, 0), Clear(ClearType::CurrentLine))?;
        }

        let s = format!("Rounds: {}/{}", gs.sm.round, gs.sm.num_rounds);
        execute!(gs.srn, MoveTo(0, 1), Print(s))?;

        let cards = gs.sm.board.as_card().expect("Should not crash!");
        let out_str = cards_str(cards);
        execute!(gs.srn, MoveTo(20, 1), Print("Board: "), Print(out_str))?;

        let mut p = gs.sm.your_index;
        if !(0..=3).contains(&p) {
            p = 0;
        }

        if gs.sm.turn == -1 {
            execute!(gs.srn, MoveTo(0, 3))?;
            for _ in 0..4 {
                let player = &gs.sm.players[p as usize];
                let name = player.name.as_string();
                let name = if name.is_empty() {
                    String::from("-- Empty Seat --")
                } else {
                    name
                };
                let n_cards: usize = player.num_cards as usize;
                print!("\r{}.{:>16}{}:", p + 1, name, COL_NORMAL);
                print!(" #{n_cards:2}");
                print!("{:>34}: ", "Delta Score");
                print!(
                    " {}  {}",
                    score_str(player.delta_score),
                    score_str(player.score)
                );
                if player.is_ready {
                    print!(" {COL_BTN_PASS_AUTO}READY{COL_NORMAL}");
                }
                print!("\r\n");
                p += 1;
                if p == 4 {
                    p = 0;
                };
            }
            draw_btn_ready(gs)?;

            return Ok(());
        }

        if p >= 0 {
            draw_btn_play(gs)?;
            draw_btn_pass(gs)?;
        }

        for row in 0..4 {
            let player = &gs.sm.players[p as usize];
            let name = player.name.as_string();
            let name = if name.is_empty() {
                String::from("-- Empty Seat --")
            } else {
                name
            };
            let s = format!("{name:>16}: ");

            let mut out_str = String::new();
            let mut out_sel_str = String::new();
            let n_cards = player.num_cards as usize;

            let has_passed = player.has_passed_this_cycle;
            let player_score = player.score;

            if p == gs.sm.your_index {
                let cards = gs.sm.your_hand.to_card();
                for bit in 12..64 {
                    let card = cards & (1 << bit);
                    if card == 0 {
                        continue;
                    }
                    if gs.cards_selected & card == 0 {
                        out_sel_str.push_str("  ");
                        cards_to_utf8(card, &mut out_str);
                    } else {
                        cards_to_utf8(card, &mut out_sel_str);
                        out_str.push_str("^^");
                    }
                    out_str.push(' ');
                    out_sel_str.push(' ');
                }
                execute!(
                    gs.srn,
                    MoveTo(24, 2),
                    Clear(ClearType::CurrentLine),
                    Print(out_sel_str),
                )?;
            } else {
                out_str = format!("{COL_CARD_BACK}##{COL_NORMAL} ").repeat(n_cards);
            }
            let number_of_cards = ".. ".to_string().repeat(13 - n_cards);

            // Number and Names.
            execute!(gs.srn, MoveTo(0, 3 + row), Print(format!("{}.", p + 1)),)?;
            if p == gs.sm.turn {
                execute!(gs.srn, Print(s.on_dark_green()))?;
            } else if has_passed {
                execute!(gs.srn, Print(s.on_dark_grey()))?;
            } else {
                execute!(gs.srn, Print(s))?;
            }

            // Cards
            execute!(
                gs.srn,
                Print(format!("#{n_cards:2}")),
                Print(format!(" {out_str}{number_of_cards}")),
                Print(score_str(player_score)),
            )?;

            // Passed Text
            if has_passed {
                execute!(
                    gs.srn,
                    MoveTo(70, 3 + row),
                    Print("PASS".white().on_dark_grey()),
                )?;
            }
            p += 1;
            if p == 4 {
                p = 0;
            };
        }

        // // Debug Text
        // execute!(
        //     gs.srn,
        //     MoveTo(0, 7),
        //     Clear(ClearType::CurrentLine),
        //     Print(format!(
        //         "Debug: B {:x} BS {} s {:x} HS {}",
        //         gs.board, gs.board_score, gs.cards_selected, gs.hand_score
        //     ))
        // )?;

        Ok(())
    }
}
