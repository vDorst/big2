pub const SUITS: [u8; 4]  = [0x0, 0x1, 0x2, 0x3];
pub const RANKS: [u8; 13] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

pub mod rules {
	pub fn get_numbers(hand: u64) {
		let mut ranks: [u32; 16] = [0; 16];
		let mut straigth: u64 = 0;
		let mut tripps: u32 = 0;
		let mut quads:  u32 = 0;
		let mut straigths: u32 = 0;
		let mut doubles: u32 = 0;
		
		for r in crate::big2rules::RANKS.iter() {
			let idx: usize = (*r).into();
			let bitmask = 0xF << (r << 2);
			let rankmask = hand & bitmask;
			ranks[idx] = rankmask.count_ones();
			if ranks[idx] != 0 { straigth |= 1 << r; }
			if ranks[idx] == 2 { doubles += 1; }	
			if ranks[idx] == 3 { tripps += 1; }
			if ranks[idx] == 4 { quads += 1; }
		}
		let mut mask = 0b11111;
		for _ in 4..16 {
			if straigth & mask == mask { straigths += 1; }
			mask <<= 1;
		}
		// A2345
		mask = 0b1100_0000_0011_1000;
		if straigth & mask == mask { straigths += 1;};
		// 23456
		mask = 0b1000_0000_0111_1000;
		if straigth & mask == mask { straigths += 1;};
		
		let mut flushs:u32 = 0;
		//       2AKQ JT98 7654 3xxx
		mask = 0x1111_1111_1111_1000;
		for _ in 0..4 {
			let cnt = (hand & mask).count_ones();
			// println!("{:64b} {} {}", hand & mask, cnt, cnt / 5);
			flushs += cnt / 5;
			mask <<= 1;
		}
		
		let fullhouse = std::cmp::min(doubles, tripps) + std::cmp::min(doubles, quads) + std::cmp::min(tripps, quads);
		println!("R{:x?} S{:16b} {:x} D{:x} T{:x} Q{:x} FH{:x} FL{:x}", ranks, straigth, straigths, doubles, tripps, quads, fullhouse, flushs);
	}
}
