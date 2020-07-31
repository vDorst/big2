mod big2rules;
mod cli;

use std::io::{self, stdout, Write};
use crossterm::{
    queue,
    style::{self, Colorize, Print}, cursor, terminal, Result, QueueableCommand
};

struct game {
	board: u64,
	board_score: u64,
}

/*
fn draw_box<W>(w: &mut W) -> Result<()>
where
    W: Write,
{
	w
		.queue(cursor::MoveTo(0,0))?
		.queue(Print("+============+"))?
		.queue(cursor::MoveTo(0,1))?
		.queue(Print("|            |"))?
		.queue(cursor::MoveTo(0,2))?
		.queue(Print("+============+"))?
		.queue(cursor::MoveTo(0,5))?;
}
*/

fn add_player(name: String) -> big2rules::Player {
	return big2rules::Player {	name: name,
					score: 0,
					hand: 0x1FFF,
					has_passed: false,
				};
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
	gs.players.push( add_player("The King".to_string()) );
	gs.players.push( add_player("Nobody".to_string()) );

	println!("Player: {} {} first to act", gs.player_to_act, gs.players[gs.player_to_act].name);

	let me = &mut gs.players[gs.i_am_player];
	me.hand = cards[gs.i_am_player];

	let fp = &mut gs.players[gs.player_to_act];
	fp.hand &= !gs.board;

	// A2345
	gs.board = 0x8800_0000_0088_8000;
	gs.board_score = big2rules::rules::score_hand(gs.board);
	cli::display::board(&gs);
	// 23456
	gs.board = 0x1000_0000_0288_8000;
	gs.board_score = big2rules::rules::score_hand(gs.board);
	gs.player_to_act = 1;
	gs.cards_selected = cards[gs.i_am_player] & 0xF0_0000_0F00_0000;
	cli::display::board(&gs);
	// 34567
	gs.board = 0x0000_0000_1288_8000;
	gs.board_score = big2rules::rules::score_hand(gs.board);
	gs.player_to_act = 2;
	gs.players[gs.i_am_player].has_passed = true;
	cli::display::board(&gs);
	// 34567
	gs.board = 0x0000_0000_1842_1000;
	gs.board_score = big2rules::rules::score_hand(gs.board);
	gs.players[3].hand = 0x1f;
	gs.player_to_act = 0;
	gs.players[2].has_passed = true;
	gs.players[gs.i_am_player].hand &= !0xF040_00F0_0000_F000;
	
	gs.round = 2;
	gs.players[0].score = -2;
	gs.players[1].score = 8;
	gs.players[2].score = -10;
	gs.players[3].score = 4;
	
	cli::display::board(&gs);

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

