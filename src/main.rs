mod big2rules;
mod cli;
mod network;

use std::{fs::File, thread, time};

use log::error;
#[macro_use]
extern crate log;
extern crate simplelog;

use simplelog::*;

use pico_args::{Arguments, Error as paError};

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
        name: String::from(""),
        app_mode: AppMode::Client,
        socket_addr: String::from(""),
        rounds: 8,
        host_port: network::common::PORT,
        auto_play: args.contains("-auto-play"),
    };

    let join: Option<String> = args.opt_value_from_str("-join")?;

    let name: Option<String> = args.opt_value_from_str("-name")?;

    let be_host = args.contains("-host");

    let be_hostonly = args.contains("-host-only");

    if join.is_some() && (be_host || be_hostonly) {
        return Err(paError::ArgumentParsingFailed {
            cause: "-join combined with -host or -host-only is now allowed.".to_string(),
        });
    }

    if (join.is_some() || be_host) && name.is_none() {
        return Err(paError::ArgumentParsingFailed {
            cause: "-join or -host is missing -name".to_string(),
        });
    }

    if be_host {
        cli_args.app_mode = AppMode::Host;
    }

    if be_hostonly {
        cli_args.app_mode = AppMode::HostOnly;
    }

    if let Some(name) = name {
        if !(1..=16).contains(&name.len()) {
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
                join_addr.push_str(&network::common::PORT.to_string());
            }
            cli_args.socket_addr = join_addr;
            cli_args.app_mode = AppMode::Client;
        }
    }

    if be_host {
        let value: Option<u8> = args.opt_value_from_str("-rounds")?;
        cli_args.rounds = value.unwrap_or(8);

        let value: Option<u16> = args.opt_value_from_str("-port")?;
        cli_args.host_port = value.unwrap_or(network::common::PORT);
    }

    args.finish()?;

    Ok(cli_args)
}

fn main() {
    let cli_args = parse_args(Arguments::from_env());
    if let Err(e) = cli_args {
        println!("Invalid arguments! {:?}", e);
        std::process::exit(1);
    }
    let cli_args = cli_args.unwrap();

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

        srv.play(srv.turn, 0x1000).unwrap();

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

        let srn = cli::display::init(&title).unwrap();

        let client = network::client::TcpClient::connect(cli_args.socket_addr);

        if let Err(e) = client {
            let _ = cli::display::close(srn);
            print!("{}\r\n", e);
            std::process::exit(1);
        }

        let mut ts = client.unwrap();

        if let Err(e) = ts.send_join_msg(cli_args.name) {
            let _ = cli::display::close(srn);
            print!("{}\r\n", e);
            std::process::exit(1);
        }

        let mut gs = big2rules::GameState {
            srn,
            board: 0,
            board_score: 0,
            cards_selected: 0,
            auto_pass: false,
            i_am_ready: true,
            is_valid_hand: false,
            hand_score: 0,
            sm: network::StateMessage::new(None),
        };

        // Game loop
        'gameloop: loop {
            let ret = ts.check_buffer();
            if let Err(e) = ret {
                error!("Error: TCPStream: {:?}", e);
                break 'gameloop;
            }
            let buffer_sm = ret.unwrap();

            // Process new StateMessage
            if let Some(buffer) = buffer_sm {
                gs.sm = buffer;
                trace!("TRAIL: {:16x}h", gs.sm.action_msg());
                match gs.sm.action.action_type {
                    network::StateMessageActionType::PLAY => {
                        let p = gs.sm.action.player;
                        let name = gs.sm.player_name(p);
                        if name.is_some() {
                            let cards = gs.sm.action.cards.into_card().unwrap();
                            let cards_str = cli::display::cards_str(cards);
                            trace!("PLAY: {:>16}: {}", name.unwrap(), cards_str);
                        }
                    }
                    network::StateMessageActionType::PASS => {
                        let p = gs.sm.action.player;
                        let name = gs.sm.player_name(p);
                        if name.is_some() {
                            trace!("PLAY: {:>16}: PASSED", name.unwrap());
                        }
                    }
                    network::StateMessageActionType::UPDATE => {
                        trace!("PLAY: UPDATE");
                    }
                    network::StateMessageActionType::DEAL => {
                        trace!("PLAY: DEAL: ROUND {}/{}", gs.sm.round, gs.sm.num_rounds);
                    }
                };
                if gs.sm.turn == -1 {
                    let mut dscore = Vec::<i16>::with_capacity(4);
                    let mut cardnum = Vec::<u8>::with_capacity(4);
                    let mut out = String::with_capacity(256);
                    for p in 0..4 {
                        let score = gs.sm.players[p].delta_score;
                        let name = gs.sm.players[p].name.to_string();
                        dscore.push(score as i16);
                        cardnum.push(gs.sm.players[p].num_cards as u8);
                        out.push_str(&format!(" {} {} ", name, score));
                        if gs.sm.round == gs.sm.num_rounds {
                            let score = gs.sm.players[p].score;
                            out.push_str(&format!("[{}] ", score));
                        }
                        out.push('|');
                    }
                    trace!("Score: {}", out);
                }

                let next_str: String = if gs.sm.turn == -1 {
                    if gs.sm.round == gs.sm.num_rounds {
                        String::from("The END!")
                    } else {
                        String::from("Waiting for users ready")
                    }
                } else {
                    gs.sm
                        .current_player_name()
                        .unwrap_or_else(|| String::from("Unknown"))
                };
                trace!("toACT: {}", next_str);

                let title: &str = &format!("TURN: {}", next_str);
                if let Err(e) = cli::display::titlebar(&mut gs.srn, title) {
                    error!("DISPLAY TITLE ERROR {}", e);
                }

                if gs.sm.action.action_type == network::StateMessageActionType::PLAY
                    || gs.sm.action.action_type == network::StateMessageActionType::PASS
                {
                    if let Err(e) = cli::display::board(&mut gs) {
                        error!("DISPLAY ERROR {}", e);
                    }
                    let delay = if !cli_args.auto_play { 1000 } else { 10 };
                    let ten_millis = time::Duration::from_millis(delay);
                    thread::sleep(ten_millis);

                    if gs.sm.action.action_type == network::StateMessageActionType::PLAY {
                        gs.sm.board = gs.sm.action.cards;
                    }
                    gs.sm.action.action_type = network::StateMessageActionType::UPDATE;

                    // DISABLED FOR NOW!
                    // // Auto pass when hand count is less then board count
                    // if gs.sm.board.count != 0 && gs.sm.board.count > gs.sm.your_hand.count { info!("AUTO PASS: CARD COUNT"); gs.auto_pass = true; }

                    // // Auto pass when sigle card is lower then board.
                    // if gs.sm.board.count == 1 {
                    //     let boardcard = network::client::card_from_byte(gs.sm.board.data[0] );
                    //     let handcard = network::client::card_from_byte(gs.sm.your_hand.data[gs.sm.your_hand.count as usize -1]);
                    //     if  boardcard > handcard { info!("AUTO PASS: SINGLE B {:x} H {:x}", boardcard, handcard); gs.auto_pass = true; }
                    // }

                    // End of cycle?
                    if gs.sm.action.is_end_of_cycle {
                        // Clear auto_pass and players[x].hasPassed.
                        gs.auto_pass = false;
                        for p in 0..4 {
                            gs.sm.players[p].has_passed_this_cycle = false;
                        }
                        // Clear board and scores.
                        gs.sm.board = network::muon::InlineList8 {
                            data: [0; 8],
                            count: 0,
                        };
                        gs.board = 0;
                        gs.board_score = 0;
                        gs.i_am_ready = false;
                        // Clear only the cards when it is not your turn.
                        if gs.sm.turn != gs.sm.your_index {
                            gs.cards_selected = 0;
                        }
                        gs.hand_score = big2rules::rules::score_hand(gs.cards_selected);
                        if let Err(e) = cli::display::clear(&mut gs.srn) {
                            error!("DISPLAY ERROR {}", e);
                        }
                        trace!("END OF THE CYCLE");
                    }
                }

                if gs.sm.action.action_type == network::StateMessageActionType::DEAL {
                    gs.board = 0;
                    gs.board_score = 0;
                    gs.i_am_ready = false;
                    gs.cards_selected = 0;
                    gs.hand_score = 0;
                    if let Err(e) = cli::display::clear(&mut gs.srn) {
                        error!("DISPLAY ERROR {}", e);
                    }
                    gs.sm.action.action_type = network::StateMessageActionType::UPDATE;
                }

                if gs.sm.action.action_type == network::StateMessageActionType::UPDATE {
                    gs.board = gs.sm.board.into_card().unwrap();
                    gs.board_score = big2rules::rules::score_hand(gs.board);
                    gs.is_valid_hand = (gs.hand_score > gs.board_score)
                        && (gs.board == 0
                            || gs.board.count_ones() == gs.cards_selected.count_ones());

                    if let Err(e) = cli::display::board(&mut gs) {
                        error!("DISPLAY ERROR {}", e);
                    }
                }

                // Pass / Auto Pass
                let p = gs.sm.your_index as usize;
                if gs.sm.turn == gs.sm.your_index
                    && gs.auto_pass
                    && !gs.sm.players[p].has_passed_this_cycle
                {
                    if ts.action_pass().is_err() {
                        continue;
                    }
                    continue;
                }

                // println!("\n\n\r\n## B 0x{:16x} T {:2} ##", gs.board, gs.sm.turn);
                // Auto play
                if cli_args.auto_play {
                    for p in gs.sm.players.iter() {
                        if p.name.count == 0 {
                            continue 'gameloop;
                        }
                    }
                    if gs.sm.turn == -1
                        && !gs.sm.players[gs.sm.your_index as usize].is_ready
                        && !gs.i_am_ready
                    {
                        // println!("\n\n\r\n## READY ###");
                        let _ = ts.action_ready();
                        gs.i_am_ready = true;
                        continue;
                    }
                    if gs.sm.turn == gs.sm.your_index {
                        if gs.sm.board.count > 1 {
                            let _ = ts.action_pass();
                        }
                        let hand = gs.sm.your_hand.to_card();
                        let better_card = big2rules::rules::higher_single_card(gs.board, hand);
                        // println!(
                        //     "\n\n\r\n-- B 0x{:16x} H 0x{:16x} C 0x{:16x} --",
                        //     gs.board, hand, better_card
                        // );
                        if better_card == 0 {
                            let _ = ts.action_pass();
                        } else {
                            let _ = ts.action_play(better_card);
                        }
                        continue;
                    }
                }
            }

            // Poll user events
            let user_event = cli::display::poll_user_events();
            if user_event != cli::display::UserEvent::NOTHING {
                let mut toggle_card = 0;

                debug!("USEREVENT: {:?}", user_event);

                if user_event == cli::display::UserEvent::RESIZE {
                    if let Err(e) = cli::display::clear(&mut gs.srn) {
                        error!("DISPLAY ERROR {}", e);
                    }
                    if let Err(e) = cli::display::board(&mut gs) {
                        error!("DISPLAY ERROR {}", e);
                    }
                    continue;
                }

                if user_event == cli::display::UserEvent::QUIT {
                    info!("User QUIT!");
                    network::client::disconnect(ts);
                    break;
                }

                let is_inbetween: bool = gs.sm.turn == -1;

                // Ready
                if is_inbetween {
                    if !gs.i_am_ready && user_event == cli::display::UserEvent::READY {
                        gs.i_am_ready = true;
                        if ts.action_ready().is_err() {
                            continue;
                        }
                    }
                    continue;
                } else {
                    // (De)Select cards
                    if user_event == cli::display::UserEvent::TOGGLECARD1 {
                        toggle_card = 1;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD2 {
                        toggle_card = 2;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD3 {
                        toggle_card = 3;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD4 {
                        toggle_card = 4;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD5 {
                        toggle_card = 5;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD6 {
                        toggle_card = 6;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD7 {
                        toggle_card = 7;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD8 {
                        toggle_card = 8;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD9 {
                        toggle_card = 9;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD10 {
                        toggle_card = 10;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD11 {
                        toggle_card = 11;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD12 {
                        toggle_card = 12;
                    }
                    if user_event == cli::display::UserEvent::TOGGLECARD13 {
                        toggle_card = 13;
                    }
                    if user_event == cli::display::UserEvent::CLEAR && gs.cards_selected != 0 {
                        gs.cards_selected = 0;
                        gs.hand_score = 0;
                        gs.is_valid_hand = false;
                        if let Err(e) = cli::display::board(&mut gs) {
                            error!("DISPLAY ERROR {}", e);
                        }
                    }

                    let me_index = gs.sm.your_index;
                    let is_your_turn: bool = gs.sm.turn == me_index;

                    if toggle_card != 0 {
                        if toggle_card > gs.sm.your_hand.count as usize {
                            continue;
                        }
                        let card =
                            network::muon::card_from_byte(gs.sm.your_hand.data[toggle_card - 1]);
                        gs.cards_selected ^= card;
                        gs.hand_score = big2rules::rules::score_hand(gs.cards_selected);
                        gs.is_valid_hand = is_your_turn
                            && (gs.hand_score > gs.board_score)
                            && (gs.board == 0
                                || gs.board.count_ones() == gs.cards_selected.count_ones());
                        if let Err(e) = cli::display::board(&mut gs) {
                            error!("DISPLAY ERROR {}", e);
                        }
                    }

                    let you = &gs.sm.players[me_index as usize];
                    if is_your_turn {
                        // Pass
                        if user_event == cli::display::UserEvent::PASS
                            && !you.has_passed_this_cycle
                            && ts.action_pass().is_err()
                        {
                            continue;
                        }

                        // Play hand
                        if user_event == cli::display::UserEvent::PLAY && gs.is_valid_hand {
                            // println!("Play hand");
                            gs.sm.action.action_type = network::StateMessageActionType::PLAY;

                            if let Err(e) = ts.action_play(gs.cards_selected) {
                                println!("Could not send your hand to the server!\r\n{}", e);
                            }

                            gs.cards_selected = 0;
                            gs.hand_score = 0;
                            gs.is_valid_hand = false;
                        }
                    } else {
                        // Pre Pass
                        if user_event == cli::display::UserEvent::PASS && !you.has_passed_this_cycle
                        {
                            gs.auto_pass = !gs.auto_pass;
                            if let Err(e) = cli::display::board(&mut gs) {
                                error!("DISPLAY ERROR {}", e);
                            }
                        }
                    }
                }
            }
        }
        if let Err(e) = cli::display::board(&mut gs) {
            error!("DISPLAY ERROR {}", e);
        }

        // close cli right way
        let _ = cli::display::close(gs.srn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn to_vec(args: &[&str]) -> Vec<OsString> {
        args.iter().map(|s| s.to_string().into()).collect()
    }

    // valid argument tests

    #[test]
    fn argument_test_client_join_name() {
        let args = Arguments::from_vec(to_vec(&["-join", "10.10.10.10", "-name", "Test"]));
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
        let args = Arguments::from_vec(to_vec(&["-host", "-name", "IamL33T"]));
        let ar = parse_args(args).unwrap();
        let ans = CliArgs {
            name: String::from("IamL33T"),
            app_mode: AppMode::Host,
            socket_addr: String::from(""),
            rounds: 8,
            host_port: 27191,
            auto_play: false,
        };
        assert_eq!(ar, ans);
    }
    #[test]
    fn argument_test_host_join_name_rounds10() {
        let args = Arguments::from_vec(to_vec(&["-host", "-name", "IamL33T", "-rounds", "10"]));
        let ar = parse_args(args).unwrap();
        let ans = CliArgs {
            name: String::from("IamL33T"),
            app_mode: AppMode::Host,
            socket_addr: String::from(""),
            rounds: 10,
            host_port: 27191,
            auto_play: false,
        };
        assert_eq!(ar, ans);
    }

    // Invalid argument tests

    #[test]
    fn argument_test_host_join_name_rounds256() {
        let args = Arguments::from_vec(to_vec(&["-host", "-name", "IamL33T", "-rounds", "256"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::Utf8ArgumentParsingFailed { value: _, cause: _ }) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_test_too_long_name() {
        let args = Arguments::from_vec(to_vec(&["-host", "-name", "Morethensixteenchars"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_test_too_short_name() {
        let args = Arguments::from_vec(to_vec(&["-host", "-name", ""]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_test_name_no_value() {
        let args = Arguments::from_vec(to_vec(&["-host", "-name"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::OptionWithoutAValue(_)) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_test_spaces_in_name() {
        let args = Arguments::from_vec(to_vec(&["-host", "-name", "Space Me"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_test_join_host_name() {
        let args = Arguments::from_vec(to_vec(&[
            "-host",
            "-name",
            "ValidName",
            "-join",
            "10.10.10.10",
        ]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_test_host_name_invalid() {
        let args = Arguments::from_vec(to_vec(&[
            "-host",
            "-name",
            "ValidName",
            "-join",
            "10.10.10.10",
        ]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_unused_arguments_invalid() {
        let args = Arguments::from_vec(to_vec(&[
            "-invalidoption",
            "-name",
            "ValidName",
            "-join",
            "10.10.10.10",
        ]));
        let ar = parse_args(args);
        match ar {
            Err(paError::UnusedArgsLeft { 0: _ }) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_test_join_no_value() {
        let args = Arguments::from_vec(to_vec(&["-join", "-name", "ValidName"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::ArgumentParsingFailed { cause: _ }) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }

    #[test]
    fn argument_test_join_no_value_other_order() {
        let args = Arguments::from_vec(to_vec(&["-name", "ValidName", "-join"]));
        let ar = parse_args(args);
        match ar {
            Err(paError::OptionWithoutAValue(_)) => assert!(true),
            _ => {
                println!("{:?}", ar);
                assert!(false)
            }
        };
    }
}
