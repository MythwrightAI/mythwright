//! SHA-256, for the sum that goes beside a package.
//!
//! The sum was `sha256sum`'s, which is a program of one system. The
//! tooling stands on the standard library alone, so this is the function
//! itself as its standard writes it (FIPS 180-4), held to that standard's
//! own examples by the tests. It is for saying which file a file is, and
//! is nothing anybody's secret rests on.

/// The first 32 bits of the fractional parts of the cube roots of the
/// first 64 primes.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// The first 32 bits of the fractional parts of the square roots of the
/// first 8 primes: what the sum of nothing begins from.
const BEGINS: [u32; 8] =
    [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];

/// A sum being made of something handed over a piece at a time, so that a
/// file is never held whole to be summed.
pub struct Sha256 {
    state: [u32; 8],
    /// What was handed over and does not fill a block yet.
    held: Vec<u8>,
    /// How many bytes were handed over in all.
    length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Sha256 { state: BEGINS, held: Vec::with_capacity(64), length: 0 }
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Self::default()
    }

    fn block(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (word, bytes) in w.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_be_bytes(*bytes);
        }
        for t in 16..64 {
            let s0 = w[t - 15].rotate_right(7) ^ w[t - 15].rotate_right(18) ^ (w[t - 15] >> 3);
            let s1 = w[t - 2].rotate_right(17) ^ w[t - 2].rotate_right(19) ^ (w[t - 2] >> 10);
            w[t] = w[t - 16].wrapping_add(s0).wrapping_add(w[t - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for (constant, word) in K.iter().zip(w) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ (!e & g);
            let first = h.wrapping_add(s1).wrapping_add(choice).wrapping_add(*constant).wrapping_add(word);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let second = s0.wrapping_add(majority);
            (h, g, f, e, d, c, b, a) = (g, f, e, d.wrapping_add(first), c, b, a, first.wrapping_add(second));
        }
        for (kept, worked) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *kept = kept.wrapping_add(worked);
        }
    }

    /// More of what is being summed.
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        if !self.held.is_empty() {
            let wanted = (64 - self.held.len()).min(bytes.len());
            self.held.extend_from_slice(&bytes[..wanted]);
            bytes = &bytes[wanted..];
            if self.held.len() < 64 {
                return;
            }
            let held = std::mem::take(&mut self.held);
            for block in held.as_chunks::<64>().0 {
                self.block(block);
            }
        }
        let (blocks, rest) = bytes.as_chunks::<64>();
        for block in blocks {
            self.block(block);
        }
        self.held.extend_from_slice(rest);
    }

    /// The sum of everything handed over, as the 64 letters it is written in.
    pub fn finish(mut self) -> String {
        let bits = self.length.wrapping_mul(8);
        let mut last = std::mem::take(&mut self.held);
        last.push(0x80);
        while last.len() % 64 != 56 {
            last.push(0);
        }
        last.extend_from_slice(&bits.to_be_bytes());
        for block in last.as_chunks::<64>().0 {
            self.block(block);
        }
        self.state.iter().map(|word| format!("{word:08x}")).collect()
    }
}

/// The sum of `bytes`, as the 64 letters it is written in.
pub fn hex(bytes: &[u8]) -> String {
    let mut sum = Sha256::new();
    sum.update(bytes);
    sum.finish()
}

/// The sum of everything `from` gives, read a piece at a time.
pub fn of(mut from: impl std::io::Read) -> std::io::Result<String> {
    let mut sum = Sha256::new();
    let mut piece = vec![0u8; 1 << 16];
    loop {
        match from.read(&mut piece)? {
            0 => return Ok(sum.finish()),
            read => sum.update(&piece[..read]),
        }
    }
}

/// The line that goes in the file beside what was summed, in the form
/// `sha256sum` writes and `sha256sum -c` (and `shasum -a 256 -c`) reads:
/// the sum, two spaces, the file's name.
pub fn line(sum: &str, name: &str) -> String {
    format!("{sum}  {name}\n")
}
