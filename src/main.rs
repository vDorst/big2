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

fn main() -> Result<()> {
	let cards: [u64; 4] = big2rules::deck::deal();

	//cli::display::cards(cards, 2);
	//for p in 0..cards.len() {
	//	big2rules::rules::get_numbers(cards[p]);
	//}
	
	let mut stdout = stdout();

	stdout
		.queue(cursor::MoveTo(5,5))?
		.queue(Print("Styled text here."))?;
	stdout.flush()?;
	

	draw_box(&mut stdout);
	
	stdout.flush()?;	
	
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

