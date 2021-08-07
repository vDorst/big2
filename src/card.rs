pub struct Cards(pub u64);

impl Cards {
    pub fn to_bit(&self) -> u32 {
        self.0.trailing_zeros()
    }
    pub fn rank(&self) -> u32 {
        self.to_bit() >> 2
    }
    pub fn suit(&self) -> u32 {
        self.to_bit() & 0x3
    }
}

impl ToString for Cards {
    fn to_string(&self) -> String {
        let mut card_str = String::with_capacity(2);
        match self.rank() {
            3 => card_str.push('3'),
            4 => card_str.push('4'),
            5 => card_str.push('5'),
            6 => card_str.push('6'),
            7 => card_str.push('7'),
            8 => card_str.push('8'),
            9 => card_str.push('9'),
            10 => card_str.push('T'),
            11 => card_str.push('J'),
            12 => card_str.push('Q'),
            13 => card_str.push('K'),
            14 => card_str.push('A'),
            15 => card_str.push('2'),
            _ => card_str.push('?'),
        }

        match self.suit() {
            0 => card_str.push('\u{2666}'),
            1 => card_str.push('\u{2663}'),
            2 => card_str.push('\u{2665}'),
            3 => card_str.push('\u{2660}'),
            _ => (),
        }

        card_str
    }
}

impl Iterator for Cards {
    type Item = (u32, u64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        let bit = self.0.trailing_zeros();
        let mask = 1 << bit;
        self.0 = self.0 ^ mask;
        Some((bit, mask))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.0.count_ones() as usize))
    }
}
