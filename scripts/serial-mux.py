#!/usr/bin/env python3
"""Decode Edgerun serial mux frames from an ESP32-S3 USB serial port."""

import argparse
import os
import sys
import termios
import tty

FLAG = 0x7E
ESC = 0x7D
ESC_XOR = 0x20
HEADER_LEN = 5
CRC_LEN = 2

CHANNELS = {
    0: "ctl",
    1: "log",
    2: "net",
    3: "wifi",
}

CHANNEL_NAMES = {
    "control": 0,
    "ctl": 0,
    "log": 1,
    "net": 2,
    "wifi": 3,
}


def crc16(data: bytes) -> int:
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8408
            else:
                crc >>= 1
    return (~crc) & 0xFFFF


def decode_payload(raw: bytearray):
    if len(raw) < HEADER_LEN + CRC_LEN:
        return None
    body = raw[:-CRC_LEN]
    got = raw[-2] | (raw[-1] << 8)
    if crc16(body) != got:
        return ("bad-crc", 0, b"")
    length = body[1] | (body[2] << 8)
    if len(body) != HEADER_LEN + length:
        return ("bad-len", 0, b"")
    channel = body[0]
    seq = body[3] | (body[4] << 8)
    return (CHANNELS.get(channel, f"ch{channel}"), seq, bytes(body[HEADER_LEN:]))


def encode_frame(channel: int, seq: int, payload: bytes) -> bytes:
    body = bytearray()
    body.append(channel & 0xFF)
    body.extend(len(payload).to_bytes(2, "little"))
    body.extend((seq & 0xFFFF).to_bytes(2, "little"))
    body.extend(payload)
    body.extend(crc16(body).to_bytes(2, "little"))

    encoded = bytearray([FLAG])
    for byte in body:
        if byte in (FLAG, ESC):
            encoded.append(ESC)
            encoded.append(byte ^ ESC_XOR)
        else:
            encoded.append(byte)
    encoded.append(FLAG)
    return bytes(encoded)


def iter_frames(fd):
    raw = bytearray()
    in_frame = False
    escaped = False
    while True:
        chunk = os.read(fd, 1024)
        if not chunk:
            return
        for byte in chunk:
            if byte == FLAG:
                if in_frame and raw:
                    decoded = decode_payload(raw)
                    if decoded is not None:
                        yield decoded
                raw.clear()
                in_frame = True
                escaped = False
                continue
            if not in_frame:
                sys.stdout.buffer.write(bytes([byte]))
                sys.stdout.buffer.flush()
                continue
            if escaped:
                raw.append(byte ^ ESC_XOR)
                escaped = False
            elif byte == ESC:
                escaped = True
            else:
                raw.append(byte)


def write_payload(channel: str, seq: int, payload: bytes):
    if channel == "log":
        sys.stdout.buffer.write(payload)
        sys.stdout.buffer.flush()
        return
    label = f"[{channel} #{seq}] ".encode()
    sys.stdout.buffer.write(label)
    if payload and all(byte in b"\r\n\t" or 0x20 <= byte <= 0x7E for byte in payload):
        sys.stdout.buffer.write(payload)
        if not payload.endswith(b"\n"):
            sys.stdout.buffer.write(b"\n")
    else:
        sys.stdout.buffer.write(payload.hex().encode())
        sys.stdout.buffer.write(b"\n")
    sys.stdout.buffer.flush()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("port", nargs="?", default="/dev/ttyACM0")
    parser.add_argument("--send", help="send one UTF-8 payload before monitoring")
    parser.add_argument("--send-hex", help="send one hex payload before monitoring")
    parser.add_argument(
        "--channel",
        default="control",
        choices=sorted(CHANNEL_NAMES),
        help="channel used with --send or --send-hex",
    )
    parser.add_argument("--once", action="store_true", help="exit after the first decoded frame")
    args = parser.parse_args()

    fd = os.open(args.port, os.O_RDWR | os.O_NOCTTY)
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        if args.send is not None or args.send_hex is not None:
            if args.send_hex is not None:
                payload = bytes.fromhex(args.send_hex)
            else:
                payload = args.send.encode()
            os.write(fd, encode_frame(CHANNEL_NAMES[args.channel], 0, payload))
        for channel, seq, payload in iter_frames(fd):
            write_payload(channel, seq, payload)
            if args.once:
                return 0
    finally:
        termios.tcsetattr(fd, termios.TCSANOW, old)
        os.close(fd)


if __name__ == "__main__":
    raise SystemExit(main())
