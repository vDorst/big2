#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod big2rules;
mod cli;
mod client;

use std::{
    io::{stdout, Write, Read},
    time::Duration,
    env,
    thread,
    time,
};

use crossterm::{
    queue,
    style::{self, Colorize, Print}, Result, QueueableCommand,
    terminal::{disable_raw_mode, enable_raw_mode},
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    cursor::position,
    execute,
};

use clap::{Arg, App};

#[derive(PartialEq)]
enum AppMode {
    ERROR,
    HOSTONLY,
    HOST,
    CLIENT,
}

struct cycle {
    has_hand: Vec::<u8>,
    can_pass: bool,
}

struct cli_args {
    name: String,
    app_mode: AppMode,
    socket_addr: String,
}

fn parse_args() -> cli_args {
    let mut arg = cli_args {
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
            .default_value("27191")
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
            if !join_addr.contains(":") { join_addr.push_str(":27191"); }
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

fn main() -> Result<()> {
    let &buffer: &[u8; 224] = &[5, 0, 0, 0, 0xe0, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0,
    0x15, 7, 0x37, 0x28, 0x38, 0x39, 0xa, 0x2b, 0x3b, 0x2c, 0x1d, 0x3d, 2, 0, 0,
    0, 0xd, 0, 0, 0, 0x54, 0x69, 0x6b, 0x6b, 0x69, 0x65, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0x68, 0x6f, 0x73, 0x74,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
    0x52, 0x65, 0x6e, 0x65, 0x31, 0x32, 0x33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0xb,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0x52, 0x65, 0x6e, 0x65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0,
    0, 0, 0, 0, 0, 0, 0xd, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0x16, 0x26, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];


    //Request: [5, 0, 0, 0, 224, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 3, 19, 51, 52, 37, 6, 22, 38, 59, 12, 44, 61, 62, 0, 0, 0, 13, 0, 0, 0, 114, 117, 115, 116, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 82, 101, 110, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 23, 55, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    // PLay
    let &buffer: &[u8; 224] = &[5, 0, 0, 0, 224, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 3, 19, 51, 52, 37, 6, 22, 38, 59, 12, 44, 61, 62, 0, 0, 0, 13, 0, 0, 0, 114, 117, 115, 116, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 82, 101, 110, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 23, 55, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 8, 24, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];

    // Passed
    // let &buffer: &[u8; 224] = &[5, 0, 0, 0, 224, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 19, 51, 52, 37, 6, 22, 38, 59, 12, 44, 61, 62, 0, 0, 0, 13, 0, 0, 0, 114, 117, 115, 116, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 82, 101, 110, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 8, 24, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    // Plauer leaves
    //let &buffer: &[u8; 224] = &[5, 0, 0, 0, 224, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 19, 51, 52, 37, 6, 22, 38, 59, 12, 44, 61, 62, 0, 0, 0, 13, 0, 0, 0, 114, 117, 115, 116, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 82, 101, 110, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 8, 24, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    // player joins
    // Request: [5, 0, 0, 0, 224, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 19, 51, 52, 37, 6, 22, 38, 59, 12, 44, 61, 62, 0, 0, 0, 13, 0, 0, 0, 114, 117, 115, 116, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 82, 101, 110, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 78, 105, 99, 107, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 8, 24, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]



    let mut SM: client::StateMessage = bincode::deserialize(&buffer).unwrap();

    //  SM.players[2].name.count = 0;

    let p = SM.yourIndex as usize;
    SM.players[p].hasPassedThisCycle = false;
    SM.players[p].deltaScore = -20;
    SM.players[p].score = 10;
    SM.players[p].isReady = false;
    SM.turn = SM.yourIndex;
    SM.turn = 2;

    let p = 3;
    SM.players[p].hasPassedThisCycle = false;
    SM.players[p].deltaScore = 20;
    SM.players[p].score = -5;
    SM.players[p].isReady = false;

    let mut gs = big2rules::GameState {
        board: 0,
        board_score: 0,
        cards_selected: 0,
        auto_pass: false,
        i_am_ready: true,
        is_valid_hand: false,
        hand: 0,
        hand_score: 0,
        sm: SM,
    };

    // cli::display::board(&gs);
    // std::process::exit(1);

    let cli_args = parse_args();
    if cli_args.app_mode == AppMode::ERROR {
        std::process::exit(1);
    }

    enable_raw_mode()?;
    let mut stdout = stdout();
    // execute!(stdout, EnableMouseCapture)?;

    if cli_args.app_mode == AppMode::CLIENT {
        let client = client::client::TcpClient::connect(cli_args.socket_addr);

        if let Err(e) = client {
            print!("{}\r\n", e);
            // execute!(stdout, DisableMouseCapture)?;
            disable_raw_mode();
            std::process::exit(1);
        }

        let mut ts = client.unwrap();

        ts.send_join_msg(&cli_args.name)?;


        let empty_buffer = &[0u8; std::mem::size_of::<client::StateMessage>()];
        gs.sm = bincode::deserialize(empty_buffer).unwrap();
        let mut update: usize = 0;

        loop {
            match ts.check_buffer(&mut gs.sm) {
                Ok(ret) => update = ret,
                Err(e) => {
                    println!("Error {:?}", e);
                    std::process::exit(1);
                },
            }

            // Process new StateMessage
            if update == 1 {
                update = 0;
                // Update display
                cli::display::board(&gs);


                if gs.sm.action.action_type == client::StateMessage_ActionType::PLAY
                   || gs.sm.action.action_type == client::StateMessage_ActionType::PASS {
                    let ten_millis = time::Duration::from_millis(1000);

                    if gs.sm.action.action_type == client::StateMessage_ActionType::PLAY {
                        gs.sm.board = gs.sm.action.cards.clone();
                    }
                    gs.sm.action.action_type = client::StateMessage_ActionType::UPDATE;

                    // End of cycle?
                    if gs.sm.action.isEndOfCycle {
                        // Clear auto_pass and players[x].hasPassed.
                        gs.auto_pass = false;
                        for p in 0..4 {
                            gs.sm.players[p].hasPassedThisCycle = false;
                        }
                        // Clear board and scores.
                        gs.sm.board = client::muon_InlineList8 { data: [0; 8], count: 0, };
                        gs.board = 0;
                        gs.board_score = 0;
                        gs.i_am_ready = false;
                    }

                    gs.board = client::client::IL8_to_card(&gs.sm.board);
                    gs.board_score = big2rules::rules::score_hand(gs.board);
                    gs.is_valid_hand = (gs.hand_score > gs.board_score) && (gs.board == 0 || gs.board.count_ones() == gs.cards_selected.count_ones());

                    thread::sleep(ten_millis);
                    cli::display::board(&gs);
                }
            }

            // Pass / Auto Pass
            let p = gs.sm.yourIndex as usize;
            if gs.sm.turn == gs.sm.yourIndex && gs.auto_pass && !gs.sm.players[p].hasPassedThisCycle {
                ts.Action_Pass()?;
                continue;
            }

            // Poll user events
            if poll(Duration::from_millis(100))? {
                // It's guaranteed that read() wont block if `poll` returns `Ok(true)`
                let user_event = read()?;
                let mut toggle_card = 0;

                if user_event == Event::Key(KeyCode::Esc.into()) {
                    break;
                }

                match user_event {
                    Event::Key(user_event) => println!("{:?}", user_event),
                    Event::Mouse(user_event) => println!("{:?}", user_event),
                    Event::Resize(width, height) => println!("New size {}x{}", width, height),
                }

                // Pass / Auto Pass
                let p = gs.sm.yourIndex as usize;
                if user_event == Event::Key(KeyCode::Char('/').into()) &&
                   !gs.sm.players[p].hasPassedThisCycle {
                    if gs.sm.turn != gs.sm.yourIndex {
                        gs.auto_pass = !gs.auto_pass
                    } else {
                        ts.Action_Pass()?;
                        continue;
                    }
                }

                // Ready Signal
                if user_event == Event::Key(KeyCode::Char('r').into()) &&
                    !gs.i_am_ready && gs.sm.turn == -1 {
                    gs.i_am_ready = true;
                    ts.Action_Ready()?;
                    continue;
                }

                // (De)Select cards
                if user_event == Event::Key(KeyCode::Char('1').into()) { toggle_card = 1; }
                if user_event == Event::Key(KeyCode::Char('2').into()) { toggle_card = 2; }
                if user_event == Event::Key(KeyCode::Char('3').into()) { toggle_card = 3; }
                if user_event == Event::Key(KeyCode::Char('4').into()) { toggle_card = 4; }
                if user_event == Event::Key(KeyCode::Char('5').into()) { toggle_card = 5; }
                if user_event == Event::Key(KeyCode::Char('6').into()) { toggle_card = 6; }
                if user_event == Event::Key(KeyCode::Char('7').into()) { toggle_card = 7; }
                if user_event == Event::Key(KeyCode::Char('8').into()) { toggle_card = 8; }
                if user_event == Event::Key(KeyCode::Char('9').into()) { toggle_card = 9; }
                if user_event == Event::Key(KeyCode::Char('0').into()) { toggle_card = 10; }
                if user_event == Event::Key(KeyCode::Char('-').into()) { toggle_card = 11; }
                if user_event == Event::Key(KeyCode::Char('=').into()) { toggle_card = 12; }
                if user_event == Event::Key(KeyCode::Backspace.into()) { toggle_card = 13; }
                if user_event == Event::Key(KeyCode::Char('`').into()) {
                    gs.cards_selected = 0;
                    gs.hand_score = 0;
                }

                if toggle_card != 0 {
                    if toggle_card > gs.sm.yourHand.count as usize { break; }
                    let card = client::client::card_from_byte(gs.sm.yourHand.data[toggle_card - 1]);
                    gs.cards_selected ^= card;
                    gs.hand_score = big2rules::rules::score_hand(gs.cards_selected);
                }

                gs.is_valid_hand = (gs.hand_score > gs.board_score) && (gs.board == 0 || gs.board.count_ones() == gs.cards_selected.count_ones());

                // Play hand
                if user_event == Event::Key(KeyCode::Enter.into()) && gs.is_valid_hand && gs.sm.turn == gs.sm.yourIndex {
                    println!("Play hand");
                    gs.sm.action.action_type = client::StateMessage_ActionType::PLAY;

                    let hand = client::client::IL8_from_card(gs.cards_selected);
                    ts.Action_Play(&hand);
                    continue;
                }

                cli::display::board(&gs);
            }
        }
    }
    // execute!(stdout, DisableMouseCapture)?;
    disable_raw_mode();

    Ok(())
}
