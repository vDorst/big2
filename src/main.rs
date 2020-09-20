mod big2rules;
mod cli;
mod network;

use std::{fs::File, thread, time};

use log::error;
#[macro_use]
extern crate log;
extern crate simplelog;

use simplelog::*;

use clap::{App, Arg};

#[derive(PartialEq)]
enum AppMode {
    ERROR,
    HOSTONLY,
    HOST,
    CLIENT,
}

struct CliArgs {
    name: String,
    app_mode: AppMode,
    socket_addr: String,
}

fn parse_args() -> CliArgs {
    let mut arg = CliArgs {
        name: "To less arguments".to_string(),
        app_mode: AppMode::ERROR,
        socket_addr: "".to_string(),
    };

    let matches = App::new("big2")
        .version("v4")
        .about("CLI version of big2")
        .arg(
            Arg::with_name("join")
                .short("j")
                .long("join")
                .help("Join a server")
                .value_name("addr")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("host")
                .long("host")
                .short("h")
                .help("Be the host")
                .required_unless("join"),
        )
        .arg(
            Arg::with_name("name")
                .long("name")
                .short("n")
                .value_name("name")
                .required_unless("hostonly")
                .help("Your name, max length 16 bytes.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("hostonly")
                .short("o")
                .long("hostonly")
                .requires("host")
                .help("Be host only"),
        )
        .arg(
            Arg::with_name("rounds")
                .short("r")
                .long("rounds")
                .help("number of rounds")
                .value_name("rounds")
                .default_value("8")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .help("Set host listen port")
                .value_name("port")
                .takes_value(true),
        )
        .get_matches();

    let hostonly = matches.is_present("hostonly");
    let join = matches.is_present("join");
    let be_host = matches.is_present("host");

    arg.name = "Missing -host or -client or -hostonly".to_string();

    if join || be_host {
        let name: String = matches.value_of("name").unwrap_or("").to_string();
        println!("Hello {}", name);
        arg.name = name;
    }

    if join {
        let mut join_addr = matches.value_of("join").unwrap_or("").to_string();
        if join_addr != "" {
            // append default port is not provided.
            if !join_addr.contains(":") {
                join_addr.push(':');
                join_addr.push_str(&network::common::PORT.to_string());
            }
            arg.socket_addr = join_addr;
            arg.app_mode = AppMode::CLIENT;
        }
    }

    if hostonly {
        arg.app_mode = AppMode::HOSTONLY;
    }

    if be_host {
        arg.app_mode = AppMode::HOST;
    }

    return arg;
}

fn main() {
    let cli_args = parse_args();
    if cli_args.app_mode == AppMode::ERROR {
        std::process::exit(1);
    }

    let _ = WriteLogger::init(
        LevelFilter::Trace,
        Config::default(),
        File::create("big2.log").unwrap(),
    );

    struct GameStateServer {
        cards: [u64; 4],
        cards_played: u64,
        sm: network::StateMessage,
    }

    if cli_args.app_mode == AppMode::HOSTONLY {
        let mut gs = GameStateServer {
            cards: big2rules::deck::deal(),
            cards_played: 0,
            sm: network::StateMessage::new(None),
        };
    }

    if cli_args.app_mode == AppMode::HOST {
        error!("Currently not supported!");
        std::process::exit(1);
    }

    if cli_args.app_mode == AppMode::CLIENT {
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

        if let Err(e) = ts.send_join_msg(&cli_args.name) {
            let _ = cli::display::close(srn);
            print!("{}\r\n", e);
            std::process::exit(1);
        }

        let mut gs = big2rules::GameState {
            srn: srn,
            board: 0,
            board_score: 0,
            cards_selected: 0,
            auto_pass: false,
            i_am_ready: true,
            is_valid_hand: false,
            hand_score: 0,
            sm: network::StateMessage::new(None),
        };

        loop {
            let ret = ts.check_buffer();
            if let Err(e) = ret {
                let _ = cli::display::close(gs.srn);
                error!("Error {:?}", e);
                std::process::exit(1);
            }
            let buffer_sm = ret.unwrap();

            // Process new StateMessage
            if buffer_sm.is_some() {
                gs.sm = buffer_sm.unwrap();
                match gs.sm.action.action_type {
                    network::StateMessageActionType::PLAY => {
                        let p = gs.sm.action.player;
                        if p >= 0 && p <= 3 {
                            let cards = network::muon::inline8_to_card(&gs.sm.action.cards);
                            let player = &gs.sm.players[p as usize];
                            let name = cli::display::name_from_muon_string16(&player.name);
                            let cards_str = cli::display::cards_str(cards);
                            trace!("PLAY: {:>16}: {}", name, cards_str);
                        }
                    }
                    network::StateMessageActionType::PASS => {
                        let p = gs.sm.action.player;
                        if p >= 0 && p <= 3 {
                            let player = &gs.sm.players[p as usize];
                            let name = cli::display::name_from_muon_string16(&player.name);
                            trace!("PLAY: {:>16}: PASSED", name);
                        }
                    }
                    network::StateMessageActionType::UPDATE => {
                        trace!("PLAY: UPDATE");
                    }
                    network::StateMessageActionType::DEAL => {
                        trace!("PLAY: DEAL: ROUND {}/{}", gs.sm.round, gs.sm.num_rounds);
                    }
                };

                let next_str: String = if gs.sm.turn == -1 {
                    if gs.sm.round == gs.sm.num_rounds {
                        String::from("The END!")
                    } else {
                        String::from("Waiting for users ready")
                    }
                } else {
                    let name = gs.sm.current_player_name();
                    if name.is_none() {
                        String::from("Unknown")
                    } else {
                        name.unwrap()
                    }
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
                    let ten_millis = time::Duration::from_millis(1000);
                    thread::sleep(ten_millis);

                    if gs.sm.action.action_type == network::StateMessageActionType::PLAY {
                        gs.sm.board = gs.sm.action.cards.clone();
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
                    gs.board = network::muon::inline8_to_card(&gs.sm.board);
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
            }

            // Poll user events
            let user_event = cli::display::poll_user_events();
            if user_event != cli::display::UserEvent::NOTHING {
                let mut toggle_card = 0;

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
                        if user_event == cli::display::UserEvent::PASS && !you.has_passed_this_cycle
                        {
                            if ts.action_pass().is_err() {
                                continue;
                            }
                        }

                        // Play hand
                        if user_event == cli::display::UserEvent::PLAY && gs.is_valid_hand {
                            // println!("Play hand");
                            gs.sm.action.action_type = network::StateMessageActionType::PLAY;

                            let hand = network::muon::inline8_from_card(gs.cards_selected);
                            if let Err(e) = ts.action_play(&hand) {
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
    }
}
