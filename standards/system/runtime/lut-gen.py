#!/usr/bin/env python3
"""Generate the 256-byte character classification LUT for the unified runtime.

Bit assignments per byte index:
  bit 0: digit (0-9)
  bit 1: uppercase alpha (A-Z)
  bit 2: lowercase alpha (a-z)
  bit 3: tchar (RFC 7230 token chars)
  bit 4: hex digit (0-9, A-F, a-f)
  bit 5: whitespace (0x09, 0x0A, 0x0D, 0x20)
  bit 6: scheme byte (alphanumeric + + - .)
  bit 7: DNS label byte (alphanumeric + -)

Also generates the 256-byte lowercase LUT at 0x2000:
  maps each byte to its lowercase version (0x00 for non-alpha)
"""

LUT = [0] * 256
LOWER = [0] * 256

def bit(byte, b):
    LUT[byte] |= (1 << b)

def set_range(lo, hi, b):
    for i in range(lo, hi + 1):
        bit(i, b)

# bit 0: digit
set_range(0x30, 0x39, 0)

# bit 1: uppercase alpha
set_range(0x41, 0x5A, 1)

# bit 2: lowercase alpha
set_range(0x61, 0x7A, 2)

# bit 4: hex digit (0-9, A-F, a-f)
set_range(0x30, 0x39, 4)
set_range(0x41, 0x46, 4)
set_range(0x61, 0x66, 4)

# bit 5: whitespace
bit(0x09, 5)  # HT
bit(0x0A, 5)  # LF
bit(0x0D, 5)  # CR
bit(0x20, 5)  # SP

# bit 3: tchar (RFC 7230 §3.2.6)
# tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
#         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
tcars_chars = b"!#$%&'*+-.^_`|~"
for c in tcars_chars:
    bit(c, 3)
set_range(0x30, 0x39, 3)  # DIGIT
set_range(0x41, 0x5A, 3)  # ALPHA
set_range(0x61, 0x7A, 3)  # alpha

# bit 6: scheme byte (alphanumeric + + - .)
set_range(0x30, 0x39, 6)
set_range(0x41, 0x5A, 6)
set_range(0x61, 0x7A, 6)
bit(0x2B, 6)  # +
bit(0x2D, 6)  # -
bit(0x2E, 6)  # .

# bit 7: DNS label byte (alphanumeric + -)
set_range(0x30, 0x39, 7)
set_range(0x41, 0x5A, 7)
set_range(0x61, 0x7A, 7)
bit(0x2D, 7)  # -

# Lowercase LUT
for i in range(256):
    if LUT[i] & 2:  # uppercase
        LOWER[i] = i | 0x20
    elif LUT[i] & 4:  # lowercase
        LOWER[i] = i
    else:
        LOWER[i] = 0  # non-alpha

def hexdump(bytes):
    return ''.join(f'\\{b:02x}' for b in bytes)

print(f'(data (i32.const 0x1000) "{hexdump(LUT)}")')
print(f'(data (i32.const 0x2000) "{hexdump(LOWER)}")')
