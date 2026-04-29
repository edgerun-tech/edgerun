#!/usr/bin/env python3
"""Run ESP32-S3 Wi-Fi MMIO control sequences over the serial mux."""

import argparse
import os
import sys
import termios
import time
import tty

FLAG = 0x7E
ESC = 0x7D
ESC_XOR = 0x20
HEADER_LEN = 5
CRC_LEN = 2
CHANNEL_CONTROL = 0

CHANNELS = {
    0: "ctl",
    1: "log",
    2: "net",
    3: "wifi",
}

ALIASES = {
    "init": "wifiinit",
    "rx": "wifirx",
    "funs": "wififuns",
    "pbusdbg": "wifipbusdbg",
    "phyrx": "wifiphyrx",
    "pbus": "wifirxpbus",
    "start6": "wifistart6",
    "rftest": "wifirftest",
    "gate": "wifirxgate",
    "buf": "wifirxbuf",
    "romrx": "wifiromrx",
    "romphyrx": "wifiromphyrx",
    "dmaromrx": "wifidmaromrx",
    "dmaromphyrx": "wifidmaromphyrx",
    "macflt": "wifimacflt",
    "perflt": "wifirxper",
    "rx2440": "wifirx2440",
    "rfchan6": "wifirfchan6",
    "rfch-save": "wifirfchsave",
    "rfch-pre": "wifirfchpre",
    "rfch-mode": "wifirfchmode",
    "rfch-gain-pre": "wifirfchgainpre",
    "rfch-gain-ch": "wifirfchgainch",
    "rfch-post": "wifirfchpost",
    "rfch-restore": "wifirfchrestore",
    "phyparam": "wifiphyparam",
    "rfch-reg0": "wifirfchreg0",
    "txgain0": "wifitxgain0",
    "gainwrite0": "wifigainwrite0",
    "gainflat": "wifigainflat",
    "rfsub06c": "wifirfsub06c",
    "rfsub054": "wifirfsub054",
    "rfsub0c4": "wifirfsub0c4",
    "rfsub080": "wifirfsub080",
    "rfch-clone": "wifirfchclone",
}

PRESETS = {
    "rx6": ["init", "rx", "pbusdbg", "rx", "pbus", "start6", "phyrx", "rftest", "gate", "buf", "rx"],
    "rxrom": ["init", "rx", "romrx", "rx", "pbus", "start6", "romphyrx", "rftest", "gate", "buf", "rx"],
    "rxdmarom": ["init", "rx", "dmaromrx", "rx", "pbus", "start6", "dmaromphyrx", "rftest", "gate", "buf", "rx"],
    "rxdmaromfilter": [
        "init",
        "rx",
        "dmaromrx",
        "macflt",
        "perflt",
        "rx2440",
        "pbus",
        "dmaromphyrx",
        "rftest",
        "gate",
        "buf",
        "rx",
    ],
    "rxdmaromclone": [
        "rxdmaromfilter",
        "phyparam",
        "rfch-clone",
        "txgain0",
        "rfch-post",
        "rfch-restore",
        "rx",
    ],
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


def read_frame(fd: int, deadline: float):
    raw = bytearray()
    in_frame = False
    escaped = False
    while time.monotonic() < deadline:
        try:
            chunk = os.read(fd, 1024)
        except BlockingIOError:
            time.sleep(0.01)
            continue
        if not chunk:
            time.sleep(0.01)
            continue
        for byte in chunk:
            if byte == FLAG:
                if in_frame and raw:
                    decoded = decode_payload(raw)
                    if decoded is not None:
                        return decoded
                raw.clear()
                in_frame = True
                escaped = False
                continue
            if not in_frame:
                continue
            if escaped:
                raw.append(byte ^ ESC_XOR)
                escaped = False
            elif byte == ESC:
                escaped = True
            else:
                raw.append(byte)
    return None


def write_all(fd: int, data: bytes, deadline: float) -> bool:
    offset = 0
    while offset < len(data) and time.monotonic() < deadline:
        try:
            written = os.write(fd, data[offset:])
        except BlockingIOError:
            time.sleep(0.005)
            continue
        if written == 0:
            time.sleep(0.005)
            continue
        offset += written
    return offset == len(data)


def print_frame(channel: str, seq: int, payload: bytes) -> None:
    label = f"[{channel} #{seq}] "
    if payload and all(byte in b"\r\n\t" or 0x20 <= byte <= 0x7E for byte in payload):
        text = payload.decode(errors="replace")
    else:
        text = payload.hex()
    if text.endswith("\n"):
        sys.stdout.write(label + text)
    else:
        sys.stdout.write(label + text + "\n")
    sys.stdout.flush()


def expand_sequence(items):
    commands = []
    for item in items:
        for token in item.replace(",", " ").replace(";", " ").split():
            if token in PRESETS:
                commands.extend(expand_sequence(PRESETS[token]))
            elif token.isdigit():
                commands.append(f"wifi{token}")
            else:
                commands.append(ALIASES.get(token, token))
    return commands


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("port", nargs="?", default="/dev/ttyACM0")
    parser.add_argument("sequence", nargs="*", default=["rx6"])
    parser.add_argument("--delay", type=float, default=0.25)
    parser.add_argument("--timeout", type=float, default=2.0)
    args = parser.parse_args()

    commands = expand_sequence(args.sequence)
    fd = os.open(args.port, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        for seq, command in enumerate(commands):
            deadline = time.monotonic() + args.timeout
            if not write_all(fd, encode_frame(CHANNEL_CONTROL, seq, command.encode()), deadline):
                print(f"[host #{seq}] write-timeout command={command}", flush=True)
                time.sleep(args.delay)
                continue
            frame = None
            while time.monotonic() < deadline:
                candidate = read_frame(fd, deadline)
                if candidate is None:
                    break
                channel, got_seq, payload = candidate
                print_frame(channel, got_seq, payload)
                if channel == "ctl" and got_seq == seq:
                    frame = candidate
                    break
            if frame is None:
                print(f"[host #{seq}] timeout command={command}", flush=True)
            time.sleep(args.delay)
    finally:
        termios.tcsetattr(fd, termios.TCSANOW, old)
        os.close(fd)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
