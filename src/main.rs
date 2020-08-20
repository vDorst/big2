mod big2rules;
mod cli;
mod client;

use std::{
    time::Duration,
    thread,
    time,
};

use clap::{Arg, App};

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
        .arg(Arg::with_name("join")
            .short("j")
            .long("join")
            .help("Join a server")
            .value_name("addr")
            .takes_value(true))
        .arg(Arg::with_name("host")
            .long("host")
            .short("h")
            .help("Be the host")
            .required_unless("join"))
        .arg(Arg::with_name("name")
            .long("name")
            .short("n")
            .value_name("name")
            .required_unless("hostonly")
            .help("Your name, max length 16 bytes.")
            .takes_value(true))
        .arg(Arg::with_name("hostonly")
            .short("o")
            .long("hostonly")
            .requires("host")
            .help("Be host only"))
        .arg(Arg::with_name("rounds")
            .short("r")
            .long("rounds")
            .help("number of rounds")
            .value_name("rounds")
            .default_value("8")
            .takes_value(true))
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Set host listen port")
            .value_name("port")
            .takes_value(true))
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
                join_addr.push_str(&client::client::PORT.to_string());
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

    if cli_args.app_mode == AppMode::HOST ||
       cli_args.app_mode == AppMode::HOSTONLY {
        println!("Currently not supported!");
        std::process::exit(1);
    }

	let l = cli_args.socket_addr.len();
    let title = format!("Name: {} Table {}", &cli_args.name, &cli_args.socket_addr.get(l-1..l).unwrap());

    let srn = cli::display::init(&title).unwrap();    

    if cli_args.app_mode == AppMode::CLIENT {
        let client = client::client::TcpClient::connect(cli_args.socket_addr);

        if let Err(e) = client {
            print!("{}\r\n", e);
            let _ = cli::display::close(srn);
            std::process::exit(1);
        }

        let mut ts = client.unwrap();

        if let Err(e) = ts.send_join_msg(&cli_args.name) {
            print!("{}\r\n", e);
            let _ = cli::display::close(srn);
            std::process::exit(1);
        }

        let empty_buffer = &[0u8; std::mem::size_of::<client::StateMessage>()];
        let mut gs = big2rules::GameState {
            srn: srn,
            board: 0,
            board_score: 0,
            cards_selected: 0,
            auto_pass: false,
            i_am_ready: true,
            is_valid_hand: false,
            hand: 0,
            hand_score: 0,
            sm: bincode::deserialize(empty_buffer).unwrap(),
            counter: 0,
        };

        loop {
            let update: usize;
            match ts.check_buffer(&mut gs.sm) {
                Ok(ret) => { update = ret; gs.counter += 1; },
                Err(e) => {
                    println!("Error {:?}", e);
                    std::process::exit(1);
                },
            }

            // Process new StateMessage
            if update == 1 {
                // print!("\u{1b}[15;0f");
                // println!("Data count {}", gs.counter);
                // Update display
                
                if gs.sm.action.action_type == client::StateMessageActionType::PLAY
                   || gs.sm.action.action_type == client::StateMessageActionType::PASS {

                    cli::display::board(&mut gs);
                    let ten_millis = time::Duration::from_millis(1000);
                    thread::sleep(ten_millis);                    

                    if gs.sm.action.action_type == client::StateMessageActionType::PLAY {
                        gs.sm.board = gs.sm.action.cards.clone();
                    }
                    gs.sm.action.action_type = client::StateMessageActionType::UPDATE;

                    // End of cycle?
                    if gs.sm.action.is_end_of_cycle {
                        // Clear auto_pass and players[x].hasPassed.
                        gs.auto_pass = false;
                        for p in 0..4 {
                            gs.sm.players[p].has_passed_this_cycle = false;
                        }
                        // Clear board and scores.
                        gs.sm.board = client::MuonInlineList8 { data: [0; 8], count: 0, };
                        gs.board = 0;
                        gs.board_score = 0;
                        gs.i_am_ready = false;
						gs.cards_selected = 0;
						gs.hand = 0;
                        gs.hand_score = 0;
                        cli::display::clear(&mut gs.srn);
                    }
                }
               
                if gs.sm.action.action_type == client::StateMessageActionType::DEAL {
                    gs.board = 0;
                    gs.board_score = 0;
                    gs.i_am_ready = false;
                    gs.cards_selected = 0;
                    gs.hand = 0;
                    gs.hand_score = 0;
                    cli::display::clear(&mut gs.srn);
                    gs.sm.action.action_type = client::StateMessageActionType::UPDATE;
                }

                if gs.sm.action.action_type == client::StateMessageActionType::UPDATE {
                    gs.board = client::client::muon_inline8_to_card(&gs.sm.board);
                    gs.board_score = big2rules::rules::score_hand(gs.board);
                    gs.is_valid_hand = (gs.hand_score > gs.board_score) && (gs.board == 0 || gs.board.count_ones() == gs.cards_selected.count_ones());
                    
                    if let Err(e) = cli::display::board(&mut gs) {
                        print!("DISPLAY ERROR {}", e);
                    }
                }
    
                // Pass / Auto Pass
                let p = gs.sm.your_index as usize;
                if gs.sm.turn == gs.sm.your_index && gs.auto_pass && !gs.sm.players[p].has_passed_this_cycle {
                    if ts.action_pass().is_err() { continue; }
                    continue;
                }
            }

            // Poll user events
            let user_event = cli::display::poll_user_events(&mut gs);
            if user_event != cli::display::UserEvent::NOTHING {
                let mut toggle_card = 0;

                if user_event == cli::display::UserEvent::RESIZE {
                        cli::display::clear(&mut gs.srn);
                        cli::display::board(&mut gs); 
                        continue;
                }

                 if user_event == cli::display::UserEvent::QUIT {
                    client::client::disconnect(ts);
                    break;
                }

                let is_inbetween: bool = gs.sm.turn == -1;

                // Ready
                if is_inbetween {
                    if !gs.i_am_ready && user_event == cli::display::UserEvent::READY {
                        gs.i_am_ready = true;
                        if ts.action_ready().is_err() { continue; }
                    }
                    continue;
                } else {
                    // (De)Select cards
                    if user_event == cli::display::UserEvent::TOGGLECARD1 { toggle_card = 1; }
                    if user_event == cli::display::UserEvent::TOGGLECARD2 { toggle_card = 2; }
                    if user_event == cli::display::UserEvent::TOGGLECARD3 { toggle_card = 3; }
                    if user_event == cli::display::UserEvent::TOGGLECARD4 { toggle_card = 4; }
                    if user_event == cli::display::UserEvent::TOGGLECARD5 { toggle_card = 5; }
                    if user_event == cli::display::UserEvent::TOGGLECARD6 { toggle_card = 6; }
                    if user_event == cli::display::UserEvent::TOGGLECARD7 { toggle_card = 7; }
                    if user_event == cli::display::UserEvent::TOGGLECARD8 { toggle_card = 8; }
                    if user_event == cli::display::UserEvent::TOGGLECARD9 { toggle_card = 9; }
                    if user_event == cli::display::UserEvent::TOGGLECARD10 { toggle_card = 10; }
                    if user_event == cli::display::UserEvent::TOGGLECARD11 { toggle_card = 11; }
                    if user_event == cli::display::UserEvent::TOGGLECARD12 { toggle_card = 12; }
                    if user_event == cli::display::UserEvent::TOGGLECARD13 { toggle_card = 13; }
                    if user_event == cli::display::UserEvent::CLEAR && 
                       gs.cards_selected != 0 {
                        gs.cards_selected = 0;
                        gs.hand_score = 0;
                        gs.is_valid_hand = false;
                        cli::display::board(&mut gs);
                    }

                    let me_index = gs.sm.your_index;
                    let is_your_turn: bool = gs.sm.turn == me_index;

                    if toggle_card != 0 {
                        if toggle_card > gs.sm.your_hand.count as usize { continue; }
                        let card = client::client::card_from_byte(gs.sm.your_hand.data[toggle_card - 1]);
                        gs.cards_selected ^= card;
                        gs.hand_score = big2rules::rules::score_hand(gs.cards_selected);
                        gs.is_valid_hand = is_your_turn && (gs.hand_score > gs.board_score) &&
                                          (gs.board == 0 || gs.board.count_ones() == gs.cards_selected.count_ones());
                        cli::display::board(&mut gs);
                    }

                    let you = &gs.sm.players[me_index as usize];
                    if is_your_turn {
                        // Pass
                        if user_event == cli::display::UserEvent::PASS && 
                           !you.has_passed_this_cycle {
                            if ts.action_pass().is_err() { continue; }
                        }

                        // Play hand
                        if user_event == cli::display::UserEvent::PLAY &&
                           gs.is_valid_hand {
                            // println!("Play hand");
                            gs.sm.action.action_type = client::StateMessageActionType::PLAY;

                            let hand = client::client::muon_inline8_from_card(gs.cards_selected);
                            if let Err(e) = ts.action_play(&hand) { println!("Could not send your hand to the server!\r\n{}", e); }
                            
                            gs.cards_selected = 0;
                            gs.hand_score = 0;
                            gs.is_valid_hand = false;
                        }
                    } else {
                        // Pre Pass
                        if user_event == cli::display::UserEvent::PASS &&
                           !you.has_passed_this_cycle {
                            gs.auto_pass = !gs.auto_pass;
                            cli::display::board(&mut gs);
                        }
                    }
                }
            }
        }
        cli::display::close(gs.srn);
    }
}