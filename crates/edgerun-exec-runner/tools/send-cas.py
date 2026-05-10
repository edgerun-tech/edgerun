#!/usr/bin/env python3
import hashlib
import os
import struct
import subprocess
import sys
import threading

MAGIC = b"ERXR"
VERSION = 1
MSG_HELLO = 1
MSG_STDOUT = 10
MSG_STDERR = 11
MSG_EXIT = 12
MSG_ERROR = 13
MSG_LOG = 14
MSG_PUT_BEGIN = 20
MSG_PUT_CHUNK = 21
MSG_PUT_END = 22
MSG_PUT_OK = 23
MSG_EXEC = 24
MSG_BLOB_EXISTS = 25
CHUNK = 64 * 1024


def frame(msg_type, seq, payload=b""):
    return MAGIC + struct.pack("<HHQI", VERSION, msg_type, seq, len(payload)) + struct.pack("<I", 0) + payload


def read_frame(stream):
    header = stream.read(24)
    if len(header) == 0:
        return None
    if len(header) != 24:
        raise RuntimeError("short frame header")
    magic, version, msg_type, seq, length, _reserved = struct.unpack("<4sHHQII", header)
    if magic != MAGIC:
        raise RuntimeError(f"bad magic {magic!r}")
    if version != VERSION:
        raise RuntimeError(f"bad version {version}")
    payload = stream.read(length)
    if len(payload) != length:
        raise RuntimeError("short payload")
    return msg_type, seq, payload


def put_string(buf, text):
    data = text.encode()
    if len(data) > 65535:
        raise ValueError("string too long")
    buf.extend(struct.pack("<H", len(data)))
    buf.extend(data)


def put_begin_payload(data, mode=0o700):
    digest = hashlib.sha256(data).digest()
    payload = bytearray()
    payload.extend(digest)
    payload.extend(struct.pack("<Q", len(data)))
    payload.extend(struct.pack("<I", mode))
    return bytes(payload), digest


def exec_payload(digest, argv, timeout_ms=0):
    payload = bytearray()
    payload.extend(digest)
    payload.extend(struct.pack("<Q", timeout_ms))
    payload.extend(struct.pack("<H", len(argv)))
    payload.extend(struct.pack("<H", 0))
    for arg in argv:
        put_string(payload, arg)
    return bytes(payload)


def main():
    if len(sys.argv) < 4:
        print("usage: send-cas.py RUNNER_PATH put|run|put-run ELF_PATH [args...]", file=sys.stderr)
        return 2

    runner = sys.argv[1]
    action = sys.argv[2]
    elf = sys.argv[3]
    argv = ["./program"] + sys.argv[4:]
    data = open(elf, "rb").read()
    begin, digest = put_begin_payload(data)
    digest_hex = digest.hex()
    print(f"blob sha256:{digest_hex} bytes={len(data)} action={action}", file=sys.stderr)

    proc = subprocess.Popen([runner], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def stderr_thread():
        assert proc.stderr is not None
        for chunk in iter(lambda: proc.stderr.read(4096), b""):
            sys.stderr.buffer.write(chunk)
            sys.stderr.buffer.flush()

    threading.Thread(target=stderr_thread, daemon=True).start()

    assert proc.stdin is not None
    assert proc.stdout is not None
    hello = read_frame(proc.stdout)
    if hello:
        msg, _seq, payload = hello
        print(f"runner hello type={msg} payload={payload!r}", file=sys.stderr)

    seq = 1
    if action in ("put", "put-run"):
        proc.stdin.write(frame(MSG_PUT_BEGIN, seq, begin))
        for off in range(0, len(data), CHUNK):
            proc.stdin.write(frame(MSG_PUT_CHUNK, seq, data[off:off + CHUNK]))
        proc.stdin.write(frame(MSG_PUT_END, seq, b""))
        proc.stdin.flush()

        while True:
            msg, _seq, payload = read_frame(proc.stdout)
            if msg == MSG_PUT_OK:
                print(f"[put-ok] {payload.decode(errors='replace')}", file=sys.stderr)
                break
            if msg == MSG_BLOB_EXISTS:
                print(f"[blob-exists] {payload.decode(errors='replace')}", file=sys.stderr)
                break
            if msg == MSG_ERROR:
                print(f"[runner:error] {payload.decode(errors='replace')}", file=sys.stderr)
                proc.terminate()
                return 1
            print(f"[runner:{msg}] {payload!r}", file=sys.stderr)

    if action in ("run", "put-run"):
        seq += 1
        proc.stdin.write(frame(MSG_EXEC, seq, exec_payload(digest, argv)))
        proc.stdin.flush()
        while True:
            item = read_frame(proc.stdout)
            if item is None:
                raise RuntimeError("runner exited before EXIT")
            msg, _seq, payload = item
            if msg == MSG_STDOUT:
                sys.stdout.buffer.write(payload)
                sys.stdout.buffer.flush()
            elif msg == MSG_STDERR:
                sys.stderr.buffer.write(payload)
                sys.stderr.buffer.flush()
            elif msg == MSG_LOG:
                print(f"[runner] {payload.decode(errors='replace')}", file=sys.stderr)
            elif msg == MSG_ERROR:
                print(f"[runner:error] {payload.decode(errors='replace')}", file=sys.stderr)
                proc.terminate()
                return 1
            elif msg == MSG_EXIT:
                code = struct.unpack("<i", payload[:4])[0]
                print(f"[runner:exit] {code}", file=sys.stderr)
                proc.stdin.close()
                proc.terminate()
                return code
            else:
                print(f"[runner:unknown {msg}] {payload!r}", file=sys.stderr)

    proc.stdin.close()
    proc.terminate()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
