#!/usr/bin/env python3
"""Inspect TCL AC OTA firmware containers.

This intentionally avoids decoding secrets or contacting TCL/AWS. It is a local
binary triage tool for the recovered system.bin OTA payload.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import math
import re
import struct
from pathlib import Path


AMEBA_PT_PATTERN = bytes.fromhex("999996963FCC66FCC033CC03E5DC3162")

IMAGE_TYPES = {
    0: "PARTAB",
    1: "BOOT",
    2: "FWHS_S",
    3: "FWHS_NS",
    4: "FWLS",
    5: "ISP",
    6: "VOE",
    7: "WLN",
    8: "XIP",
    9: "CPFW",
    10: "WOWLN",
    11: "CINIT",
    63: "UNKNOWN",
}

KEYWORDS = [
    b"Ameba",
    b"RT8720",
    b"RTL8710",
    b"aws",
    b"mqtt",
    b"shadow",
    b"ota",
    b"tcl_tsl",
    b"TSL",
    b"work_mode",
    b"temperature",
    b"wind",
    b"local",
    b"lan",
    b"udp",
    b"tcp",
    b"HTTP",
    b"WEBSOCKET",
    b"BLE",
    b"AUTH_PASS",
]


def printable(data: bytes) -> str:
    return "".join(chr(b) if 32 <= b < 127 else "." for b in data)


def entropy(data: bytes) -> float:
    if not data:
        return 0.0
    counts = collections.Counter(data)
    length = len(data)
    return -sum((n / length) * math.log2(n / length) for n in counts.values())


def parse_len_prefixed_ascii(data: bytes, offset: int) -> tuple[int, bytes] | None:
    if offset + 2 > len(data):
        return None
    length = int.from_bytes(data[offset : offset + 2], "little")
    end = offset + 2 + length
    if length > 256 or end > len(data):
        return None
    value = data[offset + 2 : end]
    if any(b < 32 or b > 126 for b in value):
        return None
    return end, value


def iter_thos_headers(data: bytes):
    start = 0
    while True:
        offset = data.find(b"THOS", start)
        if offset < 0:
            return
        first = parse_len_prefixed_ascii(data, offset + 6)
        if first is None:
            yield offset, None
            start = offset + 4
            continue
        second = parse_len_prefixed_ascii(data, first[0])
        payload_len = None
        payload_md5 = None
        next_offset = first[0]
        if second is not None:
            next_offset = second[0]
            if next_offset + 36 <= len(data):
                payload_len = int.from_bytes(data[next_offset : next_offset + 4], "little")
                raw_md5 = data[next_offset + 4 : next_offset + 36]
                if re.fullmatch(rb"[0-9a-fA-F]{32}", raw_md5):
                    payload_md5 = raw_md5.decode("ascii").lower()
        yield offset, {
            "kind": data[offset + 4 : offset + 6],
            "version_1": first[1],
            "version_2": second[1] if second else None,
            "meta_end": next_offset,
            "payload_len": payload_len,
            "payload_md5": payload_md5,
        }
        start = offset + 4


def candidate_vectors(data: bytes):
    for offset in range(0, len(data) - 8, 4):
        sp, pc = struct.unpack_from("<II", data, offset)
        if not (0x20000000 <= sp <= 0x20080000):
            continue
        if not pc & 1:
            continue
        target = pc & ~1
        if 0x08000000 <= target <= 0x0CFFFFFF or 0x10000000 <= target <= 0x10100000:
            yield offset, sp, pc


def candidate_ameba_headers(data: bytes):
    for offset in range(0, len(data) - 0x60, 4):
        length, next_offset = struct.unpack_from("<II", data, offset)
        image_type = data[offset + 8]
        encrypted = data[offset + 9]
        key_index = data[offset + 10]
        flags = data[offset + 11]
        if image_type not in IMAGE_TYPES:
            continue
        if encrypted not in (0, 1) or flags > 3:
            continue
        if not (0 < length < len(data)):
            continue
        if next_offset != 0xFFFFFFFF and not (0 < next_offset < len(data)):
            continue
        yield offset, length, next_offset, IMAGE_TYPES[image_type], encrypted, key_index, flags


def strings_with_offsets(data: bytes, min_len: int = 4):
    current = bytearray()
    start = 0
    for index, byte in enumerate(data):
        if 32 <= byte < 127 or byte in (9,):
            if not current:
                start = index
            current.append(byte)
        else:
            if len(current) >= min_len:
                yield start, bytes(current)
            current.clear()
    if len(current) >= min_len:
        yield start, bytes(current)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("firmware", type=Path)
    parser.add_argument("--max-strings", type=int, default=120)
    args = parser.parse_args()

    data = args.firmware.read_bytes()
    print(f"path: {args.firmware}")
    print(f"size: {len(data)} bytes / 0x{len(data):x}")
    print(f"md5: {hashlib.md5(data).hexdigest()}")
    print(f"sha256: {hashlib.sha256(data).hexdigest()}")

    print("\nTHOS headers:")
    for offset, header in iter_thos_headers(data):
        if header is None:
            print(f"  0x{offset:08x}: THOS marker, not a clean top-level header")
            continue
        print(f"  0x{offset:08x}: kind={header['kind'].hex()} meta_end=0x{header['meta_end']:x}")
        print(f"    version_1={header['version_1'].decode('ascii', 'replace')}")
        if header["version_2"] is not None:
            print(f"    version_2={header['version_2'].decode('ascii', 'replace')}")
        if header["payload_len"] is not None:
            print(f"    payload_len={header['payload_len']} / 0x{header['payload_len']:x}")
        if header["payload_md5"] is not None:
            print(f"    payload_md5={header['payload_md5']}")
            if 0x100 < len(data):
                actual = hashlib.md5(data[0x100:]).hexdigest()
                print(f"    md5(system.bin[0x100..])={actual} match={actual == header['payload_md5']}")

    print("\nKnown patterns:")
    ptable_offsets = [hex(i) for i in range(len(data)) if data.startswith(AMEBA_PT_PATTERN, i)]
    print(f"  AmebaZ2 partition-table pattern offsets: {ptable_offsets or 'none'}")
    for needle in KEYWORDS:
        offsets = []
        start = 0
        while True:
            found = data.lower().find(needle.lower(), start)
            if found < 0:
                break
            offsets.append(found)
            start = found + 1
        if offsets:
            preview = ", ".join(f"0x{o:x}" for o in offsets[:12])
            suffix = "" if len(offsets) <= 12 else f" ... ({len(offsets)} total)"
            print(f"  {needle.decode('ascii', 'replace')}: {preview}{suffix}")

    print("\nEntropy by 64 KiB window:")
    for offset in range(0, len(data), 0x10000):
        chunk = data[offset : offset + 0x10000]
        ascii_ratio = sum(32 <= b < 127 or b in (9, 10, 13) for b in chunk) / len(chunk)
        print(
            f"  0x{offset:06x}-0x{offset + len(chunk):06x}: "
            f"entropy={entropy(chunk):.3f} ascii={ascii_ratio:.1%}"
        )

    print("\nVector table candidates:")
    vectors = list(candidate_vectors(data))
    for offset, sp, pc in vectors[:32]:
        print(f"  file=0x{offset:08x} sp=0x{sp:08x} pc=0x{pc:08x}")
    if len(vectors) > 32:
        print(f"  ... {len(vectors)} total")

    print("\nAmeba image-header candidates, word-aligned:")
    headers = list(candidate_ameba_headers(data))
    for offset, length, next_offset, image_type, encrypted, key_index, flags in headers[:48]:
        print(
            f"  file=0x{offset:08x} len=0x{length:x} next=0x{next_offset:x} "
            f"type={image_type} enc={encrypted} key=0x{key_index:02x} flags=0x{flags:x}"
        )
    if len(headers) > 48:
        print(f"  ... {len(headers)} total")

    print("\nInteresting strings:")
    matched = []
    for offset, value in strings_with_offsets(data):
        low = value.lower()
        if any(needle.lower() in low for needle in KEYWORDS):
            matched.append((offset, value))
    for offset, value in matched[: args.max_strings]:
        print(f"  0x{offset:08x}: {printable(value)}")
    if len(matched) > args.max_strings:
        print(f"  ... {len(matched)} total")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
