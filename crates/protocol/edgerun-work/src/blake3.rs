pub const OUT_LEN: usize = 32;
pub const KEY_LEN: usize = 32;
pub const BLOCK_LEN: usize = 64;
pub const CHUNK_LEN: usize = 1024;
pub const CONTENT_CHUNK_LEN: usize = 4096;

const CHUNK_START: u32 = 1 << 0;
const CHUNK_END: u32 = 1 << 1;
const PARENT: u32 = 1 << 2;
const ROOT: u32 = 1 << 3;

const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

const MSG_SCHEDULE: [[usize; 16]; 7] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8],
    [3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1],
    [10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6],
    [12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4],
    [9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7],
    [11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13],
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Hash(pub [u8; OUT_LEN]);

impl Hash {
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; OUT_LEN] {
        &self.0
    }

    pub fn to_hex(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        let mut i = 0;
        while i < OUT_LEN {
            out[i * 2] = hex_digit(self.0[i] >> 4);
            out[i * 2 + 1] = hex_digit(self.0[i] & 0x0f);
            i += 1;
        }
        out
    }
}

#[inline]
const fn hex_digit(n: u8) -> u8 {
    if n < 10 { b'0' + n } else { b'a' + n - 10 }
}

#[derive(Clone, Copy)]
struct Output {
    input_cv: [u32; 8],
    block_words: [u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
}

impl Output {
    fn chaining_value(&self) -> [u32; 8] {
        let words = compress(
            &self.input_cv,
            &self.block_words,
            self.counter,
            self.block_len,
            self.flags,
        );
        let mut cv = [0u32; 8];
        cv.copy_from_slice(&words[..8]);
        cv
    }

    fn root_hash(&self) -> Hash {
        let words = compress(
            &self.input_cv,
            &self.block_words,
            0,
            self.block_len,
            self.flags | ROOT,
        );
        let mut out = [0u8; OUT_LEN];
        let mut i = 0;
        while i < 8 {
            out[i * 4..i * 4 + 4].copy_from_slice(&words[i].to_le_bytes());
            i += 1;
        }
        Hash(out)
    }
}

#[derive(Clone)]
struct ChunkState {
    cv: [u32; 8],
    chunk_counter: u64,
    buf: [u8; BLOCK_LEN],
    buf_len: u8,
    blocks_compressed: u8,
    flags: u32,
}

impl ChunkState {
    fn new(key: [u32; 8], chunk_counter: u64, flags: u32) -> Self {
        Self {
            cv: key,
            chunk_counter,
            buf: [0; BLOCK_LEN],
            buf_len: 0,
            blocks_compressed: 0,
            flags,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        (self.blocks_compressed as usize) * BLOCK_LEN + self.buf_len as usize
    }

    fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            if self.buf_len as usize == BLOCK_LEN {
                let block_words = words_from_block(&self.buf);
                self.cv = first_8(compress(
                    &self.cv,
                    &block_words,
                    self.chunk_counter,
                    BLOCK_LEN as u32,
                    self.flags | self.start_flag(),
                ));
                self.blocks_compressed += 1;
                self.buf = [0; BLOCK_LEN];
                self.buf_len = 0;
            }
            let want = BLOCK_LEN - self.buf_len as usize;
            let take = min_usize(want, input.len());
            let start = self.buf_len as usize;
            self.buf[start..start + take].copy_from_slice(&input[..take]);
            self.buf_len += take as u8;
            input = &input[take..];
        }
    }

    fn output(&self) -> Output {
        let block_words = words_from_block(&self.buf);
        Output {
            input_cv: self.cv,
            block_words,
            counter: self.chunk_counter,
            block_len: self.buf_len as u32,
            flags: self.flags | self.start_flag() | CHUNK_END,
        }
    }

    #[inline]
    fn start_flag(&self) -> u32 {
        if self.blocks_compressed == 0 {
            CHUNK_START
        } else {
            0
        }
    }
}

pub struct Hasher {
    key: [u32; 8],
    chunk: ChunkState,
    cv_stack: [[u32; 8]; 64],
    cv_stack_len: usize,
    flags: u32,
}

impl Hasher {
    pub fn new() -> Self {
        Self::new_internal(IV, 0)
    }

    fn new_internal(key: [u32; 8], flags: u32) -> Self {
        Self {
            key,
            chunk: ChunkState::new(key, 0, flags),
            cv_stack: [[0; 8]; 64],
            cv_stack_len: 0,
            flags,
        }
    }

    pub fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            if self.chunk.len() == CHUNK_LEN {
                let chunk_cv = self.chunk.output().chaining_value();
                let total_chunks = self.chunk.chunk_counter + 1;
                self.add_chunk_cv(chunk_cv, total_chunks);
                self.chunk = ChunkState::new(self.key, total_chunks, self.flags);
            }
            let want = CHUNK_LEN - self.chunk.len();
            let take = min_usize(want, input.len());
            self.chunk.update(&input[..take]);
            input = &input[take..];
        }
    }

    pub fn finalize(&self) -> Hash {
        let mut output = self.chunk.output();
        let mut i = self.cv_stack_len;
        while i > 0 {
            i -= 1;
            output = parent_output(
                self.cv_stack[i],
                output.chaining_value(),
                self.key,
                self.flags,
            );
        }
        output.root_hash()
    }

    fn add_chunk_cv(&mut self, mut new_cv: [u32; 8], mut total_chunks: u64) {
        while (total_chunks & 1) == 0 {
            self.cv_stack_len -= 1;
            new_cv = parent_cv(
                self.cv_stack[self.cv_stack_len],
                new_cv,
                self.key,
                self.flags,
            );
            total_chunks >>= 1;
        }
        self.cv_stack[self.cv_stack_len] = new_cv;
        self.cv_stack_len += 1;
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

pub fn hash(input: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(input);
    hasher.finalize()
}

pub fn hash_4096_chunk(chunk: &[u8]) -> Hash {
    assert!(chunk.len() <= CONTENT_CHUNK_LEN);
    hash(chunk)
}

pub fn hash_4096_chunks(input: &[u8], out: &mut [Hash]) -> usize {
    let needed = chunk_count_4096(input.len());
    assert!(out.len() >= needed);
    let mut written = 0;
    let mut offset = 0;
    while offset < input.len() || (input.is_empty() && written == 0) {
        let end = min_usize(offset + CONTENT_CHUNK_LEN, input.len());
        out[written] = hash_4096_chunk(&input[offset..end]);
        written += 1;
        if end == input.len() {
            break;
        }
        offset = end;
    }
    written
}

pub const fn chunk_count_4096(len: usize) -> usize {
    if len == 0 {
        1
    } else {
        (len + CONTENT_CHUNK_LEN - 1) / CONTENT_CHUNK_LEN
    }
}

fn parent_cv(left: [u32; 8], right: [u32; 8], key: [u32; 8], flags: u32) -> [u32; 8] {
    parent_output(left, right, key, flags).chaining_value()
}

fn parent_output(left: [u32; 8], right: [u32; 8], key: [u32; 8], flags: u32) -> Output {
    let mut block_words = [0u32; 16];
    block_words[..8].copy_from_slice(&left);
    block_words[8..].copy_from_slice(&right);
    Output {
        input_cv: key,
        block_words,
        counter: 0,
        block_len: BLOCK_LEN as u32,
        flags: flags | PARENT,
    }
}

fn words_from_block(block: &[u8; BLOCK_LEN]) -> [u32; 16] {
    let mut words = [0u32; 16];
    let mut i = 0;
    while i < 16 {
        words[i] = u32::from_le_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
        i += 1;
    }
    words
}

#[inline]
fn first_8(words: [u32; 16]) -> [u32; 8] {
    let mut out = [0u32; 8];
    out.copy_from_slice(&words[..8]);
    out
}

fn compress(
    cv: &[u32; 8],
    block_words: &[u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
) -> [u32; 16] {
    let mut state = [0u32; 16];
    state[..8].copy_from_slice(cv);
    state[8..12].copy_from_slice(&IV[..4]);
    state[12] = counter as u32;
    state[13] = (counter >> 32) as u32;
    state[14] = block_len;
    state[15] = flags;

    let mut r = 0;
    while r < 7 {
        round(&mut state, block_words, &MSG_SCHEDULE[r]);
        r += 1;
    }

    let mut out = [0u32; 16];
    let mut i = 0;
    while i < 8 {
        out[i] = state[i] ^ state[i + 8];
        out[i + 8] = state[i + 8] ^ cv[i];
        i += 1;
    }
    out
}

fn round(state: &mut [u32; 16], m: &[u32; 16], s: &[usize; 16]) {
    g(state, 0, 4, 8, 12, m[s[0]], m[s[1]]);
    g(state, 1, 5, 9, 13, m[s[2]], m[s[3]]);
    g(state, 2, 6, 10, 14, m[s[4]], m[s[5]]);
    g(state, 3, 7, 11, 15, m[s[6]], m[s[7]]);
    g(state, 0, 5, 10, 15, m[s[8]], m[s[9]]);
    g(state, 1, 6, 11, 12, m[s[10]], m[s[11]]);
    g(state, 2, 7, 8, 13, m[s[12]], m[s[13]]);
    g(state, 3, 4, 9, 14, m[s[14]], m[s[15]]);
}

#[inline]
fn g(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(mx);
    state[d] = (state[d] ^ state[a]).rotate_right(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(12);
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(my);
    state[d] = (state[d] ^ state[a]).rotate_right(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(7);
}

#[inline]
const fn min_usize(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(hash: Hash) -> std::string::String {
        std::string::String::from_utf8(hash.to_hex().to_vec()).unwrap()
    }

    #[test]
    fn known_empty() {
        assert_eq!(
            hex(hash(b"")),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9ad c112b7cc9a93cae41f3262".replace(' ', "")
        );
    }

    #[test]
    fn known_abc() {
        assert_eq!(
            hex(hash(b"abc")),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    #[test]
    fn one_shot_equals_streaming() {
        let mut data = [0u8; 8193];
        for (i, b) in data.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(7);
        }
        let mut hasher = Hasher::new();
        for part in data.chunks(17) {
            hasher.update(part);
        }
        assert_eq!(hash(&data), hasher.finalize());
    }

    #[test]
    fn chunk_4096_count_and_hashes() {
        let data = [42u8; 9000];
        let mut out = [Hash([0; 32]); 3];
        let n = hash_4096_chunks(&data, &mut out);
        assert_eq!(n, 3);
        assert_eq!(out[0], hash(&data[..4096]));
        assert_eq!(out[1], hash(&data[4096..8192]));
        assert_eq!(out[2], hash(&data[8192..]));
    }
}
