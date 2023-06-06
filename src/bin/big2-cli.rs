use big2::{
    big2rules::{self, cards::Cards},
    network::legacy as net_legacy,
};
use crossterm::event::{Event, EventStream};
use futures::{select, FutureExt, StreamExt};
use std::{cmp::Ordering, fs::File, io::IsTerminal, thread, time};

use log::error;
#[macro_use]
extern crate log;
extern crate simplelog;

use pico_args::{Arguments, Error as paError};
use simplelog::{Config, LevelFilter, WriteLogger};

use crate::display::UserEvent;

#[derive(Debug, PartialEq)]
enum AppMode {
    HostOnly,
    Host,
    Client,
}

#[derive(Debug, PartialEq)]
struct CliArgs {
    name: String,
    app_mode: AppMode,
    socket_addr: String,
    rounds: u8,
    host_port: u16,
    auto_play: bool,
}

fn parse_args(mut args: Arguments) -> Result<CliArgs, paError> {
    let mut cli_args = CliArgs {
        name: String::new(),
        app_mode: AppMode::Client,
        socket_addr: String::new(),
        rounds: 8,
        host_port: net_legacy::common::PORT,
        auto_play: args.contains("--auto-play"),
    };

    let join: Option<String> = args.opt_value_from_str("--join")?;

    let name: Option<String> = args.opt_value_from_str("--name")?;

    let be_host = args.contains("--host");

    let be_hostonly = args.contains("--host-only");

    if join.is_some() && (be_host || be_hostonly) {
        return Err(paError::ArgumentParsingFailed {
            cause: "--join combined with --host or --host-only is now allowed.".to_string(),
        });
    }

    if (join.is_some() || be_host) && name.is_none() {
        return Err(paError::ArgumentParsingFailed {
            cause: "--join or -host is missing --name".to_string(),
        });
    }

    if be_host {
        cli_args.app_mode = AppMode::Host;
    }

    if be_hostonly {
        cli_args.app_mode = AppMode::HostOnly;
    }

    if let Some(name) = name {
        if name.is_empty() || name.len() > 16 {
            return Err(paError::ArgumentParsingFailed {
                cause: "Name length min 1 max 16 bytes!".to_string(),
            });
        }
        if name.contains(' ') {
            return Err(paError::ArgumentParsingFailed {
                cause: "No spaces allowed in name".to_string(),
            });
        }
        cli_args.name = name;
    }

    if let Some(join_addr) = join {
        if !join_addr.is_empty() {
            let mut join_addr = join_addr;
            // append default port is not provided.
            if !join_addr.contains(':') {
                join_addr.push(':');
                join_addr.push_str(&net_legacy::common::PORT.to_string());
            }
            cli_args.socket_addr = join_addr;
            cli_args.app_mode = AppMode::Client;
        }
    }

    if be_host {
        let value: Option<u8> = args.opt_value_from_str("--rounds")?;
        cli_args.rounds = value.unwrap_or(8);

        let value: Option<u16> = args.opt_value_from_str("--port")?;
        cli_args.host_port = value.unwrap_or(net_legacy::common::PORT);
    }

    let remaining = args.finish();

    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {remaining:?}.");
        return Err(paError::MissingArgument);
    }
    Ok(cli_args)
}

pub mod display {
    use super::{big2rules, net_legacy};
    use big2::big2rules::cards::{CardNum, Cards, ScoreKind};
    use log::trace;
    use std::io::Write;

    use crossterm::{
        cursor::{MoveTo, RestorePosition, SavePosition},
        event::{
            DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers, MouseButton,
            MouseEvent, MouseEventKind,
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

    #[derive(PartialEq, Debug)]
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

    // const COL_DIAMONDS: &str = "\u{1b}[32m"; // White on Grey
    // const COL_CLUBS: &str = "\u{1b}[34m";
    // const COL_HEARTS: &str = "\u{1b}[31m";
    // const COL_SPADES: &str = "\u{1b}[30m";

    pub fn clear<W>(srn: &mut W) -> Result<()>
    where
        W: Write,
    {
        execute!(srn, Clear(ClearType::All))
    }

    pub fn titlebar<W>(srn: &mut W, title: &str) -> Result<()>
    where
        W: Write,
    {
        execute!(srn, SetTitle(&title))
    }

    pub fn init<W>(srn: &mut W, title: &str) -> Result<()>
    where
        W: Write,
    {
        execute!(
            srn,
            EnterAlternateScreen,
            EnableMouseCapture,
            SetSize(80, 10),
            Clear(ClearType::All),
            //SetTitle(&title),
        )?;

        titlebar(srn, title)?;

        enable_raw_mode()
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

    pub fn handle_mouse_events(event: MouseEvent) -> UserEvent {
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

    pub fn handle_key_events(event: crossterm::event::KeyEvent) -> UserEvent {
        if event.modifiers != KeyModifiers::NONE {
            return UserEvent::Nothing;
        }

        match event.code {
            KeyCode::Enter => UserEvent::Play,
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
            KeyCode::Char('r') => UserEvent::Ready,
            KeyCode::Char('`') => UserEvent::Clear,
            KeyCode::Char('q') => UserEvent::Quit,
            KeyCode::Char('/') => UserEvent::Pass,
            KeyCode::Char('d') => UserEvent::Resize,
            _ => UserEvent::Nothing,
        }
    }

    pub(crate) fn cards_to_utf8(card: CardNum, card_str: &mut String) {
        let rank = card.rank();
        let suit = card.suit();

        card_str.push_str(COL_CARD_BACK);
        card_str.push(rank.as_char());

        card_str.push_str(suit.as_color());
        card_str.push(suit.as_char());

        card_str.push_str(COL_NORMAL);
    }

    #[allow(dead_code)]
    pub fn cards(cards: [u64; 4], way: usize) {
        for (p, card) in cards.iter().enumerate() {
            let mut out_str = String::new();
            for c in 0..big2rules::deck::NUMBER_OF_CARDS {
                let bit = big2rules::deck::START_BIT + c;
                let dsp_card = card & (1 << bit);
                if dsp_card == 0 {
                    continue;
                }
                if way == 2 {
                    cards_to_utf8(CardNum::try_from(bit).unwrap(), &mut out_str);
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
            let bit = big2rules::deck::START_BIT + c;
            let dsp_card = cards & (1 << bit);
            if dsp_card == 0 {
                continue;
            }
            cards_to_utf8(CardNum::try_from(bit).unwrap(), &mut out_str);
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
        let line = if gs.is_valid_hand {
            if gs.sm.your_index == gs.sm.turn {
                "[ PLAY ]".white().on_green()
            } else {
                "[ PLAY ]".white().on_blue()
            }
        } else {
            "[ PLAY ]".white().on_dark_grey()
        };
        execute!(
            gs.srn,
            SavePosition,
            MoveTo(43, 1),
            Print(line),
            RestorePosition
        )
    }

    pub fn draw_btn_ready(gs: &mut big2rules::GameState) -> Result<()> {
        let line = if gs.i_am_ready {
            "[x] READY".white().on_dark_grey()
        } else {
            "[ ] READY".white().on_dark_blue()
        };
        execute!(
            gs.srn,
            SavePosition,
            MoveTo(66, 1),
            Print(line),
            RestorePosition
        )
    }

    pub fn draw_btn_pass(gs: &mut big2rules::GameState) -> Result<()> {
        let has_passed_this_cycle = gs.sm.players[gs.sm.your_index as usize].has_passed_this_cycle;

        let line = if has_passed_this_cycle {
            "[X] PASS".white().on_dark_grey()
        } else if gs.auto_pass {
            "[v] PASS".white().on_blue()
        } else {
            "[ ] PASS".white().on_red()
        };
        execute!(
            gs.srn,
            SavePosition,
            MoveTo(55, 1),
            Print(line),
            RestorePosition
        )?;
        Ok(())
    }

    #[must_use]
    pub fn cards_str(cards: Cards) -> String {
        let score = big2rules::rules::score_hand(cards);
        let odd_straight = score.is_some_and(|score| match score {
            ScoreKind::Straight(a) | ScoreKind::StraightFlush(a) => a.is_odd_straight(),
            _ => false,
        });

        // With odd_straight 23456 or A2345 we start at the `ACE` first.
        let mut bit: u64 = 1 << if odd_straight { 38 } else { 12 };
        let mut card_str = String::with_capacity(64);
        for _ in 12..64 {
            let card = bit & cards;

            if card != 0 {
                cards_to_utf8(CardNum::lowcard(Cards(bit)).unwrap(), &mut card_str);
                card_str.push(' ');
            }

            if bit == 1 << 63 {
                bit = 1 << 12;
            } else {
                bit <<= 1;
            }
        }
        card_str
    }

    pub fn board(gs: &mut big2rules::GameState) -> Result<()> {
        let name = gs.sm.players[gs.sm.action.player as usize]
            .name
            .as_str()
            .unwrap();
        let s = format!("{name:>16}: ");

        if gs.sm.action.action_type == net_legacy::StateMessageActionType::Pass {
            execute!(
                gs.srn,
                MoveTo(9, 0),
                Print(&s),
                Print("PASSED".white().on_dark_grey())
            )?;
        } else if gs.sm.action.action_type == net_legacy::StateMessageActionType::Play {
            let cards = gs.sm.action.cards.try_into().expect("Should not crash!");
            let card_str = cards_str(cards);
            execute!(gs.srn, MoveTo(9, 0), Print(&s), Print(card_str))?;
        } else {
            execute!(gs.srn, MoveTo(9, 0), Clear(ClearType::CurrentLine))?;
        }

        let s = format!("Rounds: {}/{}", gs.sm.round, gs.sm.num_rounds);
        execute!(gs.srn, MoveTo(0, 1), Print(s))?;

        let cards = gs.sm.board.try_into().expect("Should not crash!");
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
                let name = if let Ok(name) = player.name.as_str() {
                    if name.is_empty() {
                        "-- Empty Seat --"
                    } else {
                        name
                    }
                } else {
                    "Conv. Error"
                };
                print!(
                    "\r{}.{name:>16}{COL_NORMAL}: #{:2}",
                    p + 1,
                    player.num_cards
                );
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
            let name = if let Ok(name) = player.name.as_str() {
                if name.is_empty() {
                    "-- Empty Seat --"
                } else {
                    name
                }
            } else {
                "Conv. Error"
            };

            let s = format!("{name:>16}: ");

            let mut out_str = String::with_capacity(39);
            let mut out_sel_str = String::with_capacity(39);
            let n_cards = player.num_cards as usize;

            let has_passed = player.has_passed_this_cycle;
            let player_score = player.score;

            if p == gs.sm.your_index {
                let cards: Cards = (&gs.sm.your_hand).try_into().expect("Should not crash");
                info!("Cards: {cards:X}");
                for cardnum in cards {
                    if gs.cards_selected & cardnum.as_card() == 0 {
                        out_sel_str.push_str("  ");
                        cards_to_utf8(cardnum, &mut out_str);
                    } else {
                        cards_to_utf8(cardnum, &mut out_sel_str);
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
            let player_name = if p == gs.sm.turn {
                s.white().on_dark_green()
            } else if has_passed {
                s.on_dark_grey()
            } else {
                s.white()
            };

            // Cards
            execute!(
                gs.srn,
                MoveTo(0, 3 + row),
                Print(format!("{}.", p + 1)),
                Print(player_name),
                Print(format!("#{n_cards:2} {out_str}{number_of_cards}")),
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

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let cli_args = match parse_args(Arguments::from_env()) {
        Ok(args) => args,
        Err(e) => {
            println!("Invalid arguments! {e:?}");
            std::process::exit(1);
        }
    };

    let logfilename = if cli_args.app_mode == AppMode::Client {
        format!("{}.log", &cli_args.name)
    } else {
        String::from("big2.log")
    };

    let _ = WriteLogger::init(
        LevelFilter::Trace,
        Config::default(),
        File::create(logfilename).unwrap(),
    );

    if cli_args.app_mode == AppMode::HostOnly {
        let mut srv = big2rules::SrvGameState::new(cli_args.rounds);

        srv.deal(None);

        println!("Start game {}/{}", srv.round, srv.rounds);

        srv.play(srv.turn, Cards(0x1000)).unwrap();

        srv.pass(srv.turn).unwrap();

        println!("{}", srv.turn);

        error!("Currently not supported!");
        std::process::exit(1);
    }

    if cli_args.app_mode == AppMode::Host {
        error!("Currently not supported!");
        std::process::exit(1);
    }

    if cli_args.app_mode == AppMode::Client {
        let l = cli_args.socket_addr.len();
        let title = format!(
            "Name: {} Table {}",
            &cli_args.name,
            &cli_args.socket_addr.get(l - 1..l).unwrap()
        );

        let mut srn = std::io::stdout();
        display::init(&mut srn, &title).unwrap();

        let mut ts = match net_legacy::client::TcpClient::connect(&cli_args.socket_addr).await {
            Ok(ts) => ts,
            Err(e) => {
                let _ = display::close(srn);
                print!("{e}\r\n");
                std::process::exit(1);
            }
        };

        if let Err(e) = ts.send_join_msg(&cli_args.name).await {
            let _ = display::close(srn);
            print!("{e}\r\n");
            std::process::exit(1);
        }

        let mut gs = big2rules::GameState {
            srn,
            board: Cards::default(),
            board_score: None,
            cards_selected: Cards::default(),
            auto_pass: false,
            i_am_ready: true,
            is_valid_hand: false,
            hand: Cards::default(),
            sm: net_legacy::StateMessage::new(None),
        };

        let mut reader = EventStream::new();

        // Game loop
        'gameloop: loop {
            let mut user_event = reader.next().fuse();
            let mut update_display = false;

            select! {
                sm = ts.rx.recv().fuse() => {
                    match sm {
                        None => {
                            error!("Error: TCPStream Closed!");
                            break 'gameloop;
                        },
                        // Process new StateMessage
                        Some(sm) => {
                            gs.sm = sm;

                            trace!("TRAIL: {:16x}h", gs.sm.action_msg());
                            match gs.sm.action.action_type {
                                net_legacy::StateMessageActionType::Play => {
                                    let p = gs.sm.action.player;
                                    if let Some(name) = gs.sm.player_name(p) {
                                        let cards = gs.sm.action.cards.try_into().unwrap();
                                        let cards_str = display::cards_str(cards);
                                        trace!("PLAY: {name:>16}: {cards_str}");
                                    }
                                }
                                net_legacy::StateMessageActionType::Pass => {
                                    let p = gs.sm.action.player;
                                    if let Some(name) = gs.sm.player_name(p) {
                                        trace!("PLAY: {name:>16}: PASSED");
                                    }
                                }
                                net_legacy::StateMessageActionType::Update => {
                                    trace!("PLAY: UPDATE");
                                }
                                net_legacy::StateMessageActionType::Deal => {
                                    trace!("PLAY: DEAL: ROUND {}/{}", gs.sm.round, gs.sm.num_rounds);
                                }
                            };
                            if gs.sm.turn == -1 {
                                let mut dscore = Vec::<i16>::with_capacity(4);
                                let mut cardnum = Vec::<u8>::with_capacity(4);
                                let mut out = String::with_capacity(256);
                                for p in 0..4 {
                                    let score = gs.sm.players[p].delta_score;
                                    let name = if let Ok(name) = gs.sm.players[p].name.as_str() { name } else { "Conv. Error" };
                                    dscore.push(score as i16);
                                    cardnum.push(gs.sm.players[p].num_cards as u8);
                                    out.push_str(&format!(" {name} {score} "));
                                    if gs.sm.round == gs.sm.num_rounds {
                                        let score = gs.sm.players[p].score;
                                        out.push_str(&format!("[{score}] "));
                                    }
                                    out.push('|');
                                }
                                trace!("Score: {}", out);
                            }

                            let next_str = if gs.sm.turn == -1 {
                                if gs.sm.round == gs.sm.num_rounds {
                                    "The END!"
                                } else {
                                    "Waiting for users ready"
                                }
                            } else if let Some(name) = gs.sm.current_player_name() {
                                name
                            } else {
                                "Unknown"
                            };
                            trace!("toACT: {}", next_str);

                            let title: &str = &format!("TURN: {next_str}");
                            if let Err(e) = display::titlebar(&mut gs.srn, title) {
                                error!("DISPLAY TITLE ERROR {}", e);
                            }

                            if gs.sm.action.action_type == net_legacy::StateMessageActionType::Play
                                || gs.sm.action.action_type == net_legacy::StateMessageActionType::Pass
                            {
                                if let Err(e) = display::board(&mut gs) {
                                    error!("DISPLAY ERROR {e}");
                                }
                                let delay = if !cli_args.auto_play { 1000 } else { 10 };
                                let ten_millis = time::Duration::from_millis(delay);
                                thread::sleep(ten_millis);

                                if gs.sm.action.action_type == net_legacy::StateMessageActionType::Play {
                                    gs.sm.board = gs.sm.action.cards;
                                }
                                gs.sm.action.action_type = net_legacy::StateMessageActionType::Update;

                                // DISABLED FOR NOW!
                                // // Auto pass when hand count is less then board count
                                // if gs.sm.board.count != 0 && gs.sm.board.count > gs.sm.your_hand.count { info!("AUTO PASS: CARD COUNT"); gs.auto_pass = true; }

                                // // Auto pass when sigle card is lower then board.
                                // if gs.sm.board.count == 1 {
                                //     let boardcard = net_legacy::client::card_from_byte(gs.sm.board.data[0] );
                                //     let handcard = net_legacy::client::card_from_byte(gs.sm.your_hand.data[gs.sm.your_hand.count as usize -1]);
                                //     if  boardcard > handcard { info!("AUTO PASS: SINGLE B {:x} H {:x}", boardcard, handcard); gs.auto_pass = true; }
                                // }

                                // End of cycle?
                                if gs.sm.action.is_end_of_cycle {
                                    // Clear auto_pass and players[x].hasPassed.
                                    gs.auto_pass = false;
                                    for player in &mut gs.sm.players {
                                        player.has_passed_this_cycle = false;
                                    }
                                    // Clear board and scores.
                                    gs.sm.board = net_legacy::muon::InlineList8 {
                                        data: [0; 8],
                                        count: 0,
                                    };
                                    gs.board = Cards::default();
                                    gs.board_score = None;
                                    gs.i_am_ready = false;
                                    // Clear only the cards when it is not your turn.
                                    if gs.sm.turn != gs.sm.your_index {
                                        gs.cards_selected = Cards::default();
                                    }
                                    if let Err(e) = display::clear(&mut gs.srn) {
                                        error!("DISPLAY ERROR {}", e);
                                    }
                                    trace!("END OF THE CYCLE");
                                }
                            }

                            if gs.sm.action.action_type == net_legacy::StateMessageActionType::Deal {
                                gs.board = Cards::default();
                                gs.board_score = None;
                                gs.i_am_ready = false;
                                gs.cards_selected = Cards::default();
                                if let Err(e) = display::clear(&mut gs.srn) {
                                    error!("DISPLAY ERROR {}", e);
                                }
                                gs.sm.action.action_type = net_legacy::StateMessageActionType::Update;
                            }

                            if gs.sm.action.action_type == net_legacy::StateMessageActionType::Update {
                                gs.board = gs.sm.board.try_into().unwrap();
                                gs.board_score = big2rules::rules::score_hand(gs.board);
                                gs.hand = (&gs.sm.your_hand).try_into().unwrap();
                                let hand_score = big2rules::rules::score_hand(gs.hand);
                                gs.is_valid_hand = match (gs.board_score, hand_score) {
                                        (_, None) => false,
                                        (None, Some(_)) => true,
                                        (Some(b), Some(h)) => matches!(b.partial_cmp(&h), Some(Ordering::Greater)),
                                };
                                info!("is_valid: b{:?} vs h{:?} = {}", gs.board_score, hand_score, gs.is_valid_hand);
                                update_display = true;
                            }

                            if gs.sm.turn == gs.sm.your_index && gs.auto_pass {
                                gs.auto_pass = false;
                                let _ = ts.action_pass().await;
                            }

                            // println!("\n\n\r\n## B 0x{:16x} T {:2} ##", gs.board, gs.sm.turn);
                            // Auto play
                            if cli_args.auto_play {
                                for p in &gs.sm.players {
                                    if p.name.is_empty() {
                                        continue 'gameloop;
                                    }
                                }
                                if gs.sm.turn == -1
                                    && !gs.sm.players[gs.sm.your_index as usize].is_ready
                                    && !gs.i_am_ready
                                {
                                    // println!("\n\n\r\n## READY ###");
                                    let _ = ts.action_ready().await;
                                    gs.i_am_ready = true;
                                    continue;
                                }
                                if gs.sm.turn == gs.sm.your_index {
                                    if gs.sm.board.count > 1 {
                                        let _ = ts.action_pass().await;
                                    }
                                    let hand: Cards = (&gs.sm.your_hand).try_into().expect("Should not crash");
                                    let better_card = big2rules::rules::higher_single_card(gs.board.0, hand.0);
                                    // println!(
                                    //     "\n\n\r\n-- B 0x{:16x} H 0x{:16x} C 0x{:16x} --",
                                    //     gs.board, hand, better_card
                                    // );
                                    if better_card == 0 {
                                        let _ = ts.action_pass().await;
                                    } else {
                                        let _ = ts.action_play(better_card).await;
                                    }
                                    continue;
                                }
                            }
                        }
                    }
                },
                polled_event = user_event => {
                    // Poll user events
                    let user_event = match polled_event {
                        Some(Ok(event)) =>  match event {
                            Event::Key(key_event) => display::handle_key_events(key_event),
                            Event::Mouse(mouse_event) => display::handle_mouse_events(mouse_event),
                            Event::Resize(_, _) => UserEvent::Resize,
                            Event::FocusGained | Event::FocusLost | Event::Paste(_) => UserEvent::Nothing,
                        },
                        Some(Err(e)) => {
                            error!("Error: {:?}\r", e);
                            break 'gameloop;
                        },
                        None => break 'gameloop,
                    };

                    if user_event == display::UserEvent::Nothing {
                        continue;
                    }

                    // info!("user event: {user_event:?}");
                    let mut toggle_card = 0;

                    if user_event == display::UserEvent::Resize {
                        if let Err(e) = display::clear(&mut gs.srn) {
                            error!("DISPLAY ERROR {e}");
                        }
                        if let Err(e) = display::board(&mut gs) {
                            error!("DISPLAY ERROR {e}");
                        }
                        continue;
                    }

                    if user_event == display::UserEvent::Quit {
                        net_legacy::client::disconnect(ts);
                        break 'gameloop;
                    }

                    let is_inbetween: bool = gs.sm.turn == -1;

                    // Ready
                    if is_inbetween {
                        if !gs.i_am_ready && user_event == display::UserEvent::Ready {
                            gs.i_am_ready = true;
                            if ts.action_ready().await.is_err() {
                                continue;
                            }
                        }
                        continue;
                    } else {
                        let me_index = gs.sm.your_index;
                        let is_your_turn: bool = gs.sm.turn == me_index;
                        let you = &gs.sm.players[me_index as usize];

                        // (De)Select cards
                        match user_event {
                            display::UserEvent::ToggleCard1 => toggle_card = 1,
                            display::UserEvent::ToggleCard2 => toggle_card = 2,
                            display::UserEvent::ToggleCard3 => toggle_card = 3,
                            display::UserEvent::ToggleCard4 => toggle_card = 4,
                            display::UserEvent::ToggleCard5 => toggle_card = 5,
                            display::UserEvent::ToggleCard6 => toggle_card = 6,
                            display::UserEvent::ToggleCard7 => toggle_card = 7,
                            display::UserEvent::ToggleCard8 => toggle_card = 8,
                            display::UserEvent::ToggleCard9 => toggle_card = 9,
                            display::UserEvent::ToggleCard10 => toggle_card = 10,
                            display::UserEvent::ToggleCard11 => toggle_card = 11,
                            display::UserEvent::ToggleCard12 => toggle_card = 12,
                            display::UserEvent::ToggleCard13 => toggle_card = 13,
                            display::UserEvent::Clear => if gs.cards_selected != Cards(0) {
                                gs.cards_selected = Cards::default();
                                gs.is_valid_hand = false;
                                update_display = true;
                            },
                            display::UserEvent::Play => {
                                // Play hand
                                if gs.is_valid_hand {
                                    // println!("Play hand");
                                    gs.sm.action.action_type = net_legacy::StateMessageActionType::Play;

                                    if let Err(e) = ts.action_play(gs.cards_selected.0).await {
                                        error!("Could not send your hand to the server!\r\n{e}");
                                    }

                                    gs.cards_selected = Cards::default();
                                    gs.is_valid_hand = false;
                                }
                            },
                            display::UserEvent::Pass => {
                                if !you.has_passed_this_cycle {
                                    if is_your_turn {
                                        // Pass
                                        if ts.action_pass().await.is_err()
                                        {
                                            continue;
                                        }
                                    } else {
                                        // Pre Pass
                                        gs.auto_pass = !gs.auto_pass;
                                        update_display = true;
                                    }
                                }
                            }
                            display::UserEvent::Nothing
                            | display::UserEvent::Quit
                            | display::UserEvent::Ready
                            | display::UserEvent::Resize => (),
                        }

                        if toggle_card != 0 {
                            let mut val = gs.hand.into_iter();
                            if let Some(card) = val.nth(toggle_card - 1) {
                                info!("Select {card:?}");
                                gs.cards_selected ^= card.as_card();
                                let hand_score = big2rules::rules::score_hand(gs.cards_selected);
                                gs.is_valid_hand = match (gs.board_score, hand_score) {
                                        (_, None) => false,
                                        (None, Some(_)) => true,
                                        (Some(b), Some(h)) => matches!(h.partial_cmp(&b), Some(Ordering::Greater)),
                                };
                                info!("is_valid: b{:?} vs h{:?} = {}", gs.board_score, hand_score, gs.is_valid_hand);
                                update_display = true;
                            }
                        }
                    }
                }
            }

            if update_display {
                if let Err(e) = display::board(&mut gs) {
                    error!("DISPLAY ERROR {e}");
                }
            }
        }
        // close cli right way
        let _ = display::close(gs.srn);
    }
}

#[cfg(test)]
mod tests {
    use big2::{
        big2rules::cards::CardNum,
        legacy::{PlayerID, ServerState, ServerStatePlayers},
    };
    // use big2::big2rules::GameState;
    use crossterm::{execute, terminal::SetSize};

    use super::{display::cards_to_utf8, *};
    use std::ffi::OsString;

    fn to_vec(args: &[&str]) -> Vec<OsString> {
        args.iter().map(|s| (*s).to_string().into()).collect()
    }

    // valid argument tests
    #[test]
    fn argument_test_client_join_name() {
        let args = Arguments::from_vec(to_vec(&["--join", "10.10.10.10", "--name", "Test"]));
        let ar = parse_args(args).unwrap();
        let ans = CliArgs {
            name: String::from("Test"),
            app_mode: AppMode::Client,
            socket_addr: String::from("10.10.10.10:27191"),
            rounds: 8,
            host_port: 27191,
            auto_play: false,
        };
        assert_eq!(ar, ans);
    }

    #[test]
    fn argument_test_host_join_name() {
        let args = Arguments::from_vec(to_vec(&["--host", "--name", "IamL33T"]));
        let ar = parse_args(args).unwrap();
        let ans = CliArgs {
            name: String::from("IamL33T"),
            app_mode: AppMode::Host,
            socket_addr: String::new(),
            rounds: 8,
            host_port: 27191,
            auto_play: false,
        };
        assert_eq!(ar, ans);
    }
    #[test]
    fn argument_test_host_join_name_rounds10() {
        let args = Arguments::from_vec(to_vec(&["--host", "--name", "IamL33T", "--rounds", "10"]));
        let ar = parse_args(args).unwrap();
        let ans = CliArgs {
            name: String::from("IamL33T"),
            app_mode: AppMode::Host,
            socket_addr: String::new(),
            rounds: 10,
            host_port: 27191,
            auto_play: false,
        };
        assert_eq!(ar, ans);
    }

    // Invalid argument tests
    #[test]
    fn argument_test_host_join_name_rounds256() {
        let args = Arguments::from_vec(to_vec(&["--host", "--name", "IamL33T", "--rounds", "256"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::Utf8ArgumentParsingFailed { value: _, cause: _ }) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_test_too_long_name() {
        let args = Arguments::from_vec(to_vec(&["--host", "--name", "Morethensixteenchars"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_test_too_short_name() {
        let args = Arguments::from_vec(to_vec(&["--host", "--name", ""]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_test_name_no_value() {
        let args = Arguments::from_vec(to_vec(&["--host", "--name"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::OptionWithoutAValue(_)) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_test_spaces_in_name() {
        let args = Arguments::from_vec(to_vec(&["--host", "--name", "Space Me"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_test_join_host_name() {
        let args = Arguments::from_vec(to_vec(&[
            "--host",
            "--name",
            "ValidName",
            "--join",
            "10.10.10.10",
        ]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_test_host_name_invalid() {
        let args = Arguments::from_vec(to_vec(&[
            "--host",
            "--name",
            "ValidName",
            "--join",
            "10.10.10.10",
        ]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_unused_arguments_invalid() {
        let args = Arguments::from_vec(to_vec(&[
            "--invalidoption",
            "--name",
            "ValidName",
            "--join",
            "10.10.10.10",
        ]));
        let ar = parse_args(args);
        match ar {
            Err(paError::MissingArgument) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_test_join_no_value() {
        let args = Arguments::from_vec(to_vec(&["--join", "--name", "ValidName"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn argument_test_join_no_value_other_order() {
        let args = Arguments::from_vec(to_vec(&["--name", "ValidName", "--join"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::OptionWithoutAValue(_)) => (),
            _ => panic!("{ar:?}"),
        };
    }

    #[test]
    fn render_test() {
        let mut fake_stdio = Vec::<u8>::with_capacity(1024);

        execute!(fake_stdio, SetSize(80, 10),).unwrap();

        println!("output: {:?}", fake_stdio);
        println!("output: {}", String::from_utf8(fake_stdio).unwrap());

        let ss = ServerState {
            round: 1,
            rounds: 8,
            turn: None,
            player_id: PlayerID::try_from(1).unwrap(),
            player_hand: Cards(0x809),
            players: [
                ServerStatePlayers {
                    name: "Tikkie".into(),
                    score: 0,
                    num_cards: 13,
                },
                ServerStatePlayers {
                    name: "Tikkie".into(),
                    score: 0,
                    num_cards: 13,
                },
                ServerStatePlayers {
                    name: "Tikkie".into(),
                    score: 0,
                    num_cards: 13,
                },
                ServerStatePlayers {
                    name: "Tikkie".into(),
                    score: 0,
                    num_cards: 13,
                },
            ],
            board: None,
        };

        // let mut gs = GameState {
        //     sm: todo!(),
        //     srn: fake_stdio,
        //     board: todo!(),
        //     board_score: None,
        //     cards_selected: todo!(),
        //     auto_pass: todo!(),
        //     i_am_ready: todo!(),
        //     is_valid_hand: todo!(),
        //     hand: todo!(),
        // };
    }

    #[test]
    fn card_str_rendering() {
        let mut out_str = String::with_capacity(100);

        // "3 Diamond, Blue"
        cards_to_utf8(CardNum::try_from(12).unwrap(), &mut out_str);
        assert_eq!(out_str, "\u{1b}[30;47m3\u{1b}[34m\u{2666}\u{1b}[0m");

        out_str.clear();
        // "3 Clubs, Green"
        cards_to_utf8(CardNum::try_from(13).unwrap(), &mut out_str);
        assert_eq!(out_str, "\u{1b}[30;47m3\u{1b}[32m\u{2663}\u{1b}[0m");

        out_str.clear();
        // "3 Hearts, Red"
        cards_to_utf8(CardNum::try_from(14).unwrap(), &mut out_str);
        assert_eq!(out_str, "\u{1b}[30;47m3\u{1b}[31m\u{2665}\u{1b}[0m");

        out_str.clear();
        // "3 Spades, Black"
        cards_to_utf8(CardNum::try_from(15).unwrap(), &mut out_str);
        assert_eq!(out_str, "\u{1b}[30;47m3\u{1b}[30m\u{2660}\u{1b}[0m");

        out_str.clear();
        cards_to_utf8(CardNum::try_from(60).unwrap(), &mut out_str);
        assert_eq!(out_str, "\u{1b}[30;47m2\u{1b}[34m\u{2666}\u{1b}[0m");

        out_str.clear();
        cards_to_utf8(CardNum::try_from(61).unwrap(), &mut out_str);
        assert_eq!(out_str, "\u{1b}[30;47m2\u{1b}[32m\u{2663}\u{1b}[0m");

        out_str.clear();
        cards_to_utf8(CardNum::try_from(62).unwrap(), &mut out_str);
        assert_eq!(out_str, "\u{1b}[30;47m2\u{1b}[31m\u{2665}\u{1b}[0m");

        out_str.clear();
        cards_to_utf8(CardNum::try_from(63).unwrap(), &mut out_str);
        assert_eq!(out_str, "\u{1b}[30;47m2\u{1b}[30m\u{2660}\u{1b}[0m");

        // Pair Aces, Diamond and Heart
        let cards = Cards(0x0500_0000_0000_0000);
        out_str.clear();
        for cn in cards {
            cards_to_utf8(cn, &mut out_str);
        }
        assert_eq!(
            out_str,
            "\u{1b}[30;47mA\u{1b}[34m\u{2666}\u{1b}[0m\u{1b}[30;47mA\u{1b}[31m\u{2665}\u{1b}[0m"
        );

        // Pair Sixes, clubs and Heart
        let cards = Cards(0x600_0000);
        out_str.clear();
        for cn in cards {
            cards_to_utf8(cn, &mut out_str);
        }
        assert_eq!(
            out_str,
            "\u{1b}[30;47m6\u{1b}[32m\u{2663}\u{1b}[0m\u{1b}[30;47m6\u{1b}[31m\u{2665}\u{1b}[0m"
        );
    }
}
