// /// A helper to read bits from a byte slice in reverse order (End -> Start).
// /// Mirrors the logic of the C# `GetNextBit` and pointer decrements.
// pub struct ReverseBitReader<'a> {
//     data: &'a [u8],
//     cursor: usize,   // Points to the byte *after* the one we are currently reading
//     bits_left: u8,   // Bits remaining in the current byte (0-7)
//     current_byte: u8,
// }

// impl<'a> ReverseBitReader<'a> {
//     pub fn new(data: &'a [u8], start_offset: usize) -> Self {
//         Self {
//             data,
//             cursor: start_offset,
//             bits_left: 0, // 0 forces a read on the first call
//             current_byte: 0,
//         }
//     }

//     /// Reads a single bit (MSB to LSB).
//     pub fn read_bit(&mut self) -> u8 {
//         if self.bits_left == 0 {
//             // Move cursor back and load the next byte
//             self.cursor -= 1;
//             self.current_byte = self.data[self.cursor];
//             self.bits_left = 8;
//         }

//         self.bits_left -= 1;
//         (self.current_byte >> self.bits_left) & 1
//     }

//     /// Reads `n` bits and returns them as a u32.
//     /// Replaces the complex unrolled `Read13`/`ReadMax8` logic.
//     pub fn read_bits(&mut self, n: usize) -> u32 {
//         let mut result = 0;
//         for _ in 0..n {
//             result = (result << 1) | (self.read_bit() as u32);
//         }
//         result
//     }
// }

pub struct ReverseBitReader<'a> {
    data: &'a [u8],
    cursor: usize,
    bit_buf: u64,
    bits_left: u32,
}

impl<'a> ReverseBitReader<'a> {
    #[inline(always)]
    pub fn new(data: &'a [u8], start_offset: usize) -> Self {
        Self {
            data,
            cursor: start_offset,
            bit_buf: 0,
            bits_left: 0,
        }
    }

    #[inline(always)]
    fn refill(&mut self) {
        debug_assert!(self.cursor > 0, "ReverseBitReader underflow");

        self.cursor -= 1;
        let byte = self.data[self.cursor] as u64;

        // Shift buffer left and append byte at the bottom
        self.bit_buf = (self.bit_buf << 8) | byte;
        self.bits_left += 8;
    }

    #[inline(always)]
    pub fn read_bits(&mut self, n: u32) -> u32 {
        while self.bits_left < n {
            self.refill();
        }

        let shift = self.bits_left - n; // We want the n first bits
        let mask = (1u64 << n) - 1; // Corresponding mask
        let result = (self.bit_buf >> shift) & mask;

        self.bits_left -= n;
        self.bit_buf &= (1u64 << self.bits_left) - 1; // Updating the buffer

        result as u32
    }

    #[inline(always)]
    pub fn read_bit(&mut self) -> u32 {
        self.read_bits(1)
    }
}
