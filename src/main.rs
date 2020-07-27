use rand::Rng;

static SUITS: [u8; 4]  = [0x00, 0x10, 0x20, 0x30];
static RANKS: [u8; 13] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

pub struct Cards {
	rank: Vec<u8>,
}

fn cards_to_str(card: u8, card_str: &mut String) {
	//                       0123456789ABCDEF
	let rank_str: Vec<u8> = "???3456789TJQKA2".into();
	let rank: usize;
	let suit: u8;

	rank = (card & 0x0F).into();
	suit = (card & 0x30).into();

	card_str.push(rank_str[rank] as char);

	if suit == SUITS[0] { card_str.push_str("\u{2666}"); }
	if suit == SUITS[1] { card_str.push_str("\u{2663}"); }    		
	if suit == SUITS[2] { card_str.push_str("\u{2665}"); } 
	if suit == SUITS[3] { card_str.push_str("\u{2660}"); }
}

fn diplay_cards(cards: Cards) {
	let mut c: u8 = 0;
	
    	for r in cards.rank {
    		let mut out_str = String::from("");
    		cards_to_str(r, &mut out_str);
    		c += 1;
    		if c == 13 {
    			c = 0;
    			out_str.push('\n'); 
    		} else {
    			out_str.push(' ');
    		}
   		print!("{}", out_str);
    	}
}

pub fn gen_cards() -> Cards {
	let mut rng = rand::thread_rng();
	let mut ranks = Vec::with_capacity(52);
	let mut o: usize;
	
	// Create Cards
	for s in SUITS.iter() {
		for r in RANKS.iter() {
			ranks.push(r + s);
		}
	}
	
	// Randomize/shuffle the cards
	for _ in 0..128 {
		for c in 0..ranks.len() {
			o = rng.gen_range(0, ranks.len());
			ranks.swap(c, o);
		}
	}
	
	Cards {
		rank: ranks,
	}
}	

fn main() {
    let cards = gen_cards();
    
    diplay_cards(cards);
}
