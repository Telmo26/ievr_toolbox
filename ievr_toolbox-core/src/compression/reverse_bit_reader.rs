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
