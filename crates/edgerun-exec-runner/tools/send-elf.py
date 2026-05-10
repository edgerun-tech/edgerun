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
MSG_EXEC_BEGIN = 2
MSG_EXEC_CHUNK = 3
MSG_EXEC_END = 4
MSG_STDOUT = 10
MSG_STDERR = 11
MSG_EXIT = 12
MSG_ERROR = 13
MSG_LOG = 14
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


def build_exec_begin(path, argv, timeout_ms):
    data = open(path, "rb").read()
    digest = hashlib.sha256(data).digest()
    job_id = os.urandom(32)
    payload = bytearray()
    payload.extend(job_id)
    payload.extend(struct.pack("<Q", len(data)))
    payload.extend(digest)
    payload.extend(struct.pack("<Q", timeout_ms))
    payload.extend(struct.pack("<H", len(argv)))
    payload.extend(struct.pack("<H", 0))
    for arg in argv:
        put_string(payload, arg)
    return bytes(payload), data, digest.hex()


def main():
    if len(sys.argv) < 3:
        print("usage: send-elf.py RUNNER_PATH ELF_PATH [args...]", file=sys.stderr)
        print("example: send-elf.py target/release/edgerun-exec-runner ./hello arg1", file=sys.stderr)
        return 2

    runner = sys.argv[1]
    elf = sys.argv[2]
    argv = ["./program"] + sys.argv[3:]
    begin, data, digest = build_exec_begin(elf, argv, timeout_ms=0)
    print(f"sending {elf} bytes={len(data)} sha256={digest}", file=sys.stderr)

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
    proc.stdin.write(frame(MSG_EXEC_BEGIN, seq, begin))
    for off in range(0, len(data), CHUNK):
        proc.stdin.write(frame(MSG_EXEC_CHUNK, seq, data[off:off + CHUNK]))
    proc.stdin.write(frame(MSG_EXEC_END, seq, b""))
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
            break
        elif msg == MSG_EXIT:
            code = struct.unpack("<i", payload[:4])[0]
            print(f"[runner:exit] {code}", file=sys.stderr)
            proc.stdin.close()
            proc.terminate()
            return code
        else:
            print(f"[runner:unknown {msg}] {payload!r}", file=sys.stderr)

    proc.terminate()
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
