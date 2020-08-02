mod big2rules;
mod cli;

use std::{
	io::{self, stdout, Write},
	time::Duration,
};

use crossterm::{
    queue,
    style::{self, Colorize, Print}, Result, QueueableCommand,
    terminal::{disable_raw_mode, enable_raw_mode},
    cursor::position,
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
};

struct game {
	board: u64,
	board_score: u64,
}

fn add_player(name: String) -> big2rules::Player {
	return big2rules::Player {	name: name,
					score: 0,
					hand: 0x1FFF,
					has_passed: false,
				};
}

struct cycle {
	has_hand: Vec::<u8>,
	can_pass: bool,
}

fn main() -> Result<()> {
	let cards: [u64; 4] = big2rules::deck::deal();
	let mut gs: big2rules::GameState = big2rules::GameState {
		players: Vec::<big2rules::Player>::with_capacity(4),
		round: 1,
		rounds: 8,
		board: 0,
		board_score: 0,
		i_am_player: 0,
		player_to_act: 0,
		cards_selected: 0,
		hand_score: 0,
		is_valid_hand: false,
	};
	
	// find first player to act which as a 3 of diamonds.
	for p in 0..4 {
		if cards[p] & 0x1000 != 0 {
			gs.player_to_act = p;
			break;
		}
	}

	
	gs.players.push( add_player("Pietje".to_string()) );
	gs.players.push( add_player("René".to_string()) );
	gs.i_am_player = 1;
	gs.player_to_act = 1;
	gs.players.push( add_player("The King".to_string()) );
	gs.players.push( add_player("Nobody".to_string()) );

	println!("Player: {} {} first to act", gs.player_to_act, gs.players[gs.player_to_act].name);

	let me = &mut gs.players[gs.i_am_player];
	me.hand = cards[gs.i_am_player];

	let fp = &mut gs.players[gs.player_to_act];
	fp.hand &= !gs.board;

	enable_raw_mode()?;
	let mut stdout = stdout();
	execute!(stdout, EnableMouseCapture)?;

	cli::display::board(&gs);

 	let mut this_cycle = cycle { has_hand: Vec::<u8>::with_capacity(13), can_pass: true, };

	let me = &mut gs.players[gs.i_am_player];
	me.hand = cards[gs.i_am_player];	
	for bit in 12..64 {
		if me.hand & (1 << bit) != 0 { this_cycle.has_hand.push(bit); }
	}
		
		
	loop {
		// poll user events
		if poll(Duration::from_millis(1_000))? {
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
			
			if user_event == Event::Key(KeyCode::Char('/').into()) &&
			   this_cycle.can_pass {
				gs.players[gs.i_am_player].has_passed = !gs.players[gs.i_am_player].has_passed;
			}

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
			if user_event == Event::Key(KeyCode::Backspace.into())    { toggle_card = 13; }
			if user_event == Event::Key(KeyCode::Char('`').into()) {
				gs.cards_selected = 0;
				gs.hand_score = 0;
			}
			
			if (toggle_card != 0) {
				if toggle_card > this_cycle.has_hand.len() { break; }
				gs.cards_selected ^= 1 << (this_cycle.has_hand[toggle_card - 1] as u64);
				gs.hand_score = big2rules::rules::score_hand(gs.cards_selected);
			}
			
			gs.is_valid_hand = (gs.hand_score > gs.board_score) && (gs.board == 0 || gs.board.count_ones() ==  gs.cards_selected.count_ones());
			
			if user_event == Event::Key(KeyCode::Enter.into()) && gs.is_valid_hand { 
				gs.board = gs.cards_selected;
				gs.players[gs.i_am_player].hand &= !gs.cards_selected;
				gs.cards_selected = 0;
				gs.board_score = gs.hand_score;
				gs.cards_selected = 0;
				gs.players[gs.i_am_player].has_passed = false;
				gs.is_valid_hand = false;
			}
			cli::display::board(&gs);		
		}
	}

	execute!(stdout, DisableMouseCapture)?;
	disable_raw_mode();
	
	//cli::display::cards(cards, 2);
	//for p in 0..cards.len() {
	//	big2rules::rules::get_numbers(cards[p]);
	//}
/*
	let mut stdout = stdout();

	stdout
		.queue(cursor::MoveTo(5,5))?
		.queue(Print("Styled text here."))?;
	stdout.flush()?;

	// draw_box(&mut stdout);

	//stdout.flush()?;
*/

	Ok(())
	//let name: String = "1234567890ABCDF".to_string();
	//let score: i32 = -100;
	//let cc = 13;
	//let cc_str: String = String::from_utf8(vec![b'#'; cc]).unwrap();
	//println!("\n {:16} PASS\n [{:13}] $ {}", name, cc_str, score);
	//println!("\n {:16}\n [{:13}] $ {}      5♠ 7♥ 8♣ T♠ J♠ Q♠ ", name, cc_str, score);
	//println!("\n {:16}\n [{:13}] $ {}                                ", name, cc_str, score);
	//println!("\n {:16}           4\n [{:13}] $ {:4}  3♣ ♦ 4♠ 5♠ 7♥ 8♣ T♠ J♠ Q♠ K♥ K♠ A♥ A♠", name, cc_str, score);
}

