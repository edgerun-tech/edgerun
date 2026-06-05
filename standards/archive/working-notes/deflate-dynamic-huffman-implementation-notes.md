# deflate-inflate.wat dynamic-Huffman implementation notes

Status: dynamic Huffman did not land in this pass because `deflate-inflate.wat`
already had concurrent fixed-Huffman edits in progress. The safe result is a
valid stored + fixed baseline with `standard_id = 300067`; BTYPE=2 should be
integrated into that same module, not split into a parallel module.

## Integration point

Use the existing bit-reader globals and helpers:

- `$br_init`
- `$br_read`
- `$br_align_byte`
- `$br_consumed_bytes`
- `$reverse_bits`

In `deflate_inflate_raw`, dispatch BTYPE=2 immediately after the BTYPE=3
reserved-block rejection and before the fixed-Huffman BTYPE=1 branch.

Dynamic decode should update the same local state as stored/fixed:

- `$written`
- `$block_count`
- `$crc`
- `$adler`

On EOB, increment `$block_count`, write `last_block_type = 2`, and use
`$br_consumed_bytes` for `consumed` when `bfinal = 1`.

## Scratch layout

The module has 6 memory pages. Existing runners use input near `1024`, output
near `140000`, and record near `260000`, so keep dynamic scratch above `300000`.

Recommended bounded scratch:

```text
300000..300063  code-length counts[16] u32
300064..300127  code-length offsets[16] u32
300128..300203  code-length symbols[19] u32
300256..300319  litlen counts[16] u32
300320..300383  litlen offsets[16] u32
300384..301535  litlen symbols[288] u32
301600..301663  distance counts[16] u32
301664..301727  distance offsets[16] u32
301728..301855  distance symbols[32] u32
301900..302219  dynamic code lengths[320] u8
```

## Required helpers

Add a bounded canonical-table builder:

```text
build_table(lens_ptr, symbol_count, symbols_ptr, counts_ptr, offsets_ptr, maxbits) -> status
```

It must:

- reject any code length greater than `maxbits`;
- count lengths 1..maxbits;
- reject oversubscribed trees;
- reject no-code trees for literal/length and distance alphabets;
- allow incomplete only where DEFLATE permits it without ambiguous decode;
- build symbols sorted by canonical code length/order.

Add canonical decode:

```text
decode_symbol(symbols_ptr, counts_ptr, maxbits) -> symbol_or_negative
```

Use the puff-style incremental decode over LSB-first bits:

```text
code = 0
first = 0
index = 0
for len in 1..maxbits:
  bit = br_read(1)
  code |= bit
  count = counts[len]
  if code - first < count:
    return symbols[index + (code - first)]
  index += count
  first = (first + count) << 1
  code <<= 1
```

Return `-1` for input-short and `-3` for invalid/no-code.

## Dynamic block steps

1. Read `HLIT = br_read(5) + 257`, `HDIST = br_read(5) + 1`, `HCLEN = br_read(4) + 4`.
2. Validate `HLIT <= 286`, `HDIST <= 32`, `HCLEN <= 19`.
3. Zero 19 code-length lens slots.
4. Read `HCLEN` 3-bit code lengths in DEFLATE order:
   `16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15`.
5. Build the code-length table with `maxbits = 7`.
6. Decode `HLIT + HDIST` lengths into the shared lens array.
7. Implement repeat opcodes:
   - `16`: repeat previous length `3 + br_read(2)` times; reject at index 0.
   - `17`: repeat zero `3 + br_read(3)` times.
   - `18`: repeat zero `11 + br_read(7)` times.
8. Reject repeat overflow beyond `HLIT + HDIST`.
9. Require literal/length symbol 256 EOB to have a nonzero length.
10. Build lit/len and distance tables with `maxbits = 15`.
11. Decode symbols:
    - `<256`: emit literal, update CRC32 and Adler32.
    - `256`: EOB success.
    - `257..285`: length base/extra, then distance symbol/base/extra.
12. Reject invalid length symbols, invalid distance symbols, distance before
    output start, distance greater than 32768, output-short, and limit-exceeded.
13. Match-copy byte by byte from current output so overlapping copies work.
14. If input ends before EOB, return `1`; if a symbol path is structurally
    invalid, return `3`.

## Smoke vectors

Dynamic BTYPE=2 success vector generated with Node `zlib.deflateRawSync`:

```js
const dynamicHex =
  "cdcb470100410803404b942d202790c5bf84b371f31fbb05768673f486d491b6f241d2a10991b70ed037636d1d99576c6c4b9301c8b19ffc0f";
const dynamicText =
  "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
  "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
  "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
  "27badc98";
```

Add these smoke cases to `standards/runners/deflate-inflate-smoke.js` once the
implementation lands:

- inflate `dynamicHex` to `dynamicText`, record `lastBlockType = 2`;
- corrupt one byte in the dynamic header/code-length section and expect `3`;
- truncate the final byte and expect missing-EOB/input-short rejection;
- run a compact dynamic distance-copy vector if the generated stream uses a
  length/distance pair; otherwise add a second generated vector that repeats a
  nontrivial phrase and assert output bytes.

After landing, run:

```bash
node standards/runners/deflate-inflate-smoke.js
node standards/runners/compression-zlib-composition-smoke.js
node standards/runners/compression-gzip-composition-smoke.js
```

Then rerun manifest coverage and duplicate `standard_id` checks.
