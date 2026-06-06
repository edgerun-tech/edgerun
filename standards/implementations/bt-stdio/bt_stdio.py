#!/usr/bin/env python3
"""
EdgeRun BT-STDIO v1 — Bluetooth stdio tunnel with TPM hardware identity.
Protocol: er-bt-stdio-1
Build: edgerun/standards -> wat2wasm -> edgerun compiler -> native x86_64

Usage:
  Server: bt_stdio.py server [--spp-channel=1]
  Client: bt_stdio.py client <bdaddr> [--spp-channel=1]
  Identity: bt_stdio.py identity [--show]
"""

import sys
import os
import socket
import select
import struct
import hashlib
import hmac
import json
import subprocess
import threading
import time
import signal
import tempfile
from pathlib import Path
from typing import Optional, Callable
from cryptography.hazmat.primitives.ciphers.aead import ChaCha20Poly1305
from cryptography.hazmat.primitives.asymmetric.x25519 import X25519PrivateKey, X25519PublicKey
from cryptography.hazmat.primitives.kdf.hkdf import HKDF
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.serialization import (
    Encoding, PublicFormat, PrivateFormat, NoEncryption,
)

# ---- Protocol constants ----
FRAME_TYPE = {
    "IDENTITY_REQUEST": 0x01,
    "IDENTITY_RESPONSE": 0x02,
    "IDENTITY_CONFIRM": 0x03,
    "SESSION_INIT": 0x04,
    "SESSION_READY": 0x05,
    "DATA": 0x06,
    "CLOSE": 0x07,
    "ERROR": 0xFF,
}
FRAME_BY_TYPE = {v: k for k, v in FRAME_TYPE.items()}
HEADER_SIZE = 5
MAX_PAYLOAD = 4096
CONFIG_DIR = Path("/etc/bt-stdio")
TOFU_FILE = CONFIG_DIR / "trusted.json"

# ---- TPM Identity Management ----


class TPMIdentity:
    """Device identity rooted in TPM Endorsement Key."""

    def __init__(self):
        self._ek_pub = None
        self._ak_pub = None
        self._ak_handle = None
        self._ek_handle = None
        self._loaded = False
        self._id_hash = None
        self._primary_key_path = CONFIG_DIR / "primary.ctx"
        self._ak_key_path = CONFIG_DIR / "ak.key"

    def ensure_identity(self) -> bytes:
        """Create or load TPM identity. Returns SHA-256 hash of the AK public key."""
        if self._loaded and self._id_hash:
            return self._id_hash

        CONFIG_DIR.mkdir(parents=True, exist_ok=True)

        # Create primary key under EK owner hierarchy
        if not self._primary_key_path.exists():
            self._run_tpm(
                "tpm2_createprimary",
                "-C", "o",
                "-G", "ecc",
                "-g", "sha256",
                "-c", str(self._primary_key_path),
            )

        if not self._ak_key_path.exists():
            self._run_tpm(
                "tpm2_create",
                "-C", str(self._primary_key_path),
                "-G", "ecc",
                "-g", "sha256",
                "-u", str(CONFIG_DIR / "ak.pub"),
                "-r", str(self._ak_key_path),
                "-a", "fixedtpm|fixedparent|sensitivedataorigin|sign",
            )

        # Load the AK and get its public key
        ak_ctx = CONFIG_DIR / "ak.ctx"
        if not ak_ctx.exists():
            self._run_tpm(
                "tpm2_load",
                "-C", str(self._primary_key_path),
                "-u", str(CONFIG_DIR / "ak.pub"),
                "-r", str(self._ak_key_path),
                "-c", str(ak_ctx),
                "-P", "",
            )

        # Read public key
        self._run_tpm(
            "tpm2_readpublic",
            "-c", str(ak_ctx),
            "-f", "pem",
            "-o", str(CONFIG_DIR / "identity.pub"),
        )

        pub_data = (CONFIG_DIR / "identity.pub").read_bytes()
        self._id_hash = hashlib.sha256(pub_data).digest()
        self._ak_handle = ak_ctx
        self._loaded = True
        return self._id_hash

    def sign(self, data: bytes) -> bytes:
        """Sign data with TPM identity key. Returns raw signature (r||s for ECC)."""
        sig_file = CONFIG_DIR / "sig.bin"
        msg_file = CONFIG_DIR / "msg.bin"
        msg_file.write_bytes(data)
        self._run_tpm(
            "tpm2_sign",
            "-c", str(self._ak_handle),
            "-g", "sha256",
            "-f", "tss",
            "-m", str(msg_file),
            "-s", str(sig_file),
            "-o", str(sig_file),
        )
        sig = sig_file.read_bytes()
        # Extract raw r||s from TSS signature blob
        return self._extract_ecdsa_sig(sig)

    def verify(self, data: bytes, signature: bytes, pubkey_pem: bytes) -> bool:
        """Verify signature against a given public key using TPM."""
        sig_file = CONFIG_DIR / "verify_sig.bin"
        pub_file = CONFIG_DIR / "verify_pub.pem"
        msg_file = CONFIG_DIR / "verify_msg.bin"
        msg_file.write_bytes(data)
        pub_file.write_bytes(pubkey_pem)
        # Convert raw r||s to TSS format
        tss_sig = self._make_tss_sig(signature)
        sig_file.write_bytes(tss_sig)
        try:
            self._run_tpm(
                "tpm2_verifysignature",
                "-c", str(pub_file),
                "-g", "sha256",
                "-m", str(msg_file),
                "-f", "tss",
                "-s", str(sig_file),
            )
            return True
        except subprocess.CalledProcessError:
            return False

    def get_public_key_pem(self) -> bytes:
        """Get the identity public key in PEM format."""
        pub_file = CONFIG_DIR / "identity.pub"
        if not pub_file.exists():
            self.ensure_identity()
        return pub_file.read_bytes()

    def get_public_key_hash(self) -> bytes:
        """SHA-256 hash of public key."""
        if self._id_hash:
            return self._id_hash
        return hashlib.sha256(self.get_public_key_pem()).digest()

    def get_ek_pub(self) -> bytes:
        """Get EK public key."""
        ek_pub = CONFIG_DIR / "ek.pub"
        if not ek_pub.exists():
            self._run_tpm("tpm2_createek", "-c", str(CONFIG_DIR / "ek.ctx"), "-G", "ecc")
            self._run_tpm("tpm2_readpublic", "-c", str(CONFIG_DIR / "ek.ctx"),
                         "-f", "pem", "-o", str(ek_pub))
        return ek_pub.read_bytes()

    def _run_tpm(self, *args) -> str:
        """Run a tpm2 command with sudo."""
        cmd = ["sudo"] + list(args)
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if result.returncode != 0:
            raise RuntimeError(f"TPM command failed: {' '.join(cmd)}\n{result.stderr}")
        return result.stdout

    def _extract_ecdsa_sig(self, tss_sig: bytes) -> bytes:
        """Extract raw r||s from TSS ECDSA signature blob."""
        # TSS ECDSA sig: 2-byte scheme + 2-byte hash + 4-byte count + for each sig:
        #   2-byte size + signature bytes
        offset = 8  # skip scheme + hash + count
        r_size = struct.unpack(">H", tss_sig[offset:offset + 2])[0]
        r = tss_sig[offset + 2:offset + 2 + r_size]
        s_size = struct.unpack(">H", tss_sig[offset + 2 + r_size:offset + 4 + r_size])[0]
        s = tss_sig[offset + 4 + r_size:offset + 4 + r_size + s_size]
        return r + s

    def _make_tss_sig(self, raw_sig: bytes) -> bytes:
        """Convert raw r||s ECDSA sig to TSS format."""
        r_len = len(raw_sig) // 2
        r = raw_sig[:r_len]
        s = raw_sig[r_len:]
        tss = struct.pack(">HH", 0x18, 8)  # scheme=ECDSA(0x18), hash=SHA256(8)
        tss += struct.pack(">I", 2)  # 2 signatures
        tss += struct.pack(">H", len(r)) + r
        tss += struct.pack(">H", len(s)) + s
        return tss


# ---- Trust Manager (TOFU) ----

class TrustManager:
    """Manage trusted device identities using TOFU."""

    def __init__(self):
        self._trusted = {}
        self._load()

    def _load(self):
        if TOFU_FILE.exists():
            self._trusted = json.loads(TOFU_FILE.read_text())

    def _save(self):
        TOFU_FILE.parent.mkdir(parents=True, exist_ok=True)
        TOFU_FILE.write_text(json.dumps(self._trusted, indent=2))

    def is_trusted(self, bdaddr: str, pubkey_hash: str) -> bool:
        """Check if device is trusted. Returns True and stores on first use."""
        # Normalize BD_ADDR
        bdaddr = bdaddr.upper()
        if bdaddr not in self._trusted:
            # TOFU: trust on first use
            self._trusted[bdaddr] = {
                "first_seen": time.time(),
                "pubkey_hash": pubkey_hash,
                "alias": "",
            }
            self._save()
            return True
        # Verify hash matches
        stored = self._trusted[bdaddr]
        if stored["pubkey_hash"] != pubkey_hash:
            return False
        return True

    def get_trusted_pubkey(self, bdaddr: str) -> Optional[str]:
        bdaddr = bdaddr.upper()
        if bdaddr in self._trusted:
            return self._trusted[bdaddr]["pubkey_hash"]
        return None


# ---- Crypto ---

class CryptoSession:
    """ChaCha20-Poly1305 encrypted session."""

    def __init__(self, key: bytes):
        self._aead = ChaCha20Poly1305(key)
        self._send_nonce = 0
        self._recv_nonce = 0

    @staticmethod
    def generate_key() -> bytes:
        return os.urandom(32)

    @staticmethod
    def derive_session(private: X25519PrivateKey, peer_public: X25519PublicKey) -> bytes:
        shared = private.exchange(peer_public)
        return HKDF(
            algorithm=hashes.SHA256(),
            length=32,
            salt=None,
            info=b"bt-stdio-v1-session",
        ).derive(shared)

    def encrypt(self, plaintext: bytes) -> bytes:
        nonce = struct.pack(">QQ", self._send_nonce, 0)
        self._send_nonce += 1
        return self._aead.encrypt(nonce, plaintext, None)

    def decrypt(self, ciphertext: bytes) -> bytes:
        nonce = struct.pack(">QQ", self._recv_nonce, 0)
        self._recv_nonce += 1
        return self._aead.decrypt(nonce, ciphertext, None)


# ---- Protocol Framing ----

def encode_frame(frame_type: int, payload: bytes = b"") -> bytes:
    length = len(payload)
    header = struct.pack(">IB", length, frame_type)
    return header + payload


def decode_frame(data: bytes) -> tuple:
    """Returns (frame_type, payload, remaining_data) or raises."""
    if len(data) < HEADER_SIZE:
        raise ValueError("Frame too short")
    length, frame_type = struct.unpack(">IB", data[:HEADER_SIZE])
    if len(data) < HEADER_SIZE + length:
        raise ValueError("Incomplete frame")
    payload = data[HEADER_SIZE:HEADER_SIZE + length]
    remaining = data[HEADER_SIZE + length:]
    return frame_type, payload, remaining


def read_frame(sock: socket.socket, timeout: float = 30.0) -> tuple:
    """Read one frame from a socket. Returns (frame_type, payload)."""
    sock.settimeout(timeout)
    header = b""
    while len(header) < HEADER_SIZE:
        chunk = sock.recv(HEADER_SIZE - len(header))
        if not chunk:
            raise ConnectionError("Connection closed")
        header += chunk
    length, frame_type = struct.unpack(">IB", header)
    payload = b""
    while len(payload) < length:
        chunk = sock.recv(length - len(payload))
        if not chunk:
            raise ConnectionError("Connection closed")
        payload += chunk
    return frame_type, payload


# ---- Handshake ----

def server_handshake(client: socket.socket, identity: TPMIdentity, trust: TrustManager,
                     client_bdaddr: str) -> CryptoSession:
    """Perform mutual attestation handshake. Returns CryptoSession."""
    # 1. Receive IDENTITY_REQUEST
    ft, _ = read_frame(client)
    assert ft == FRAME_TYPE["IDENTITY_REQUEST"], f"Expected IDENTITY_REQUEST, got {ft}"

    # 2. Send IDENTITY_RESPONSE with our public key
    my_pub = identity.get_public_key_pem()
    my_hash = identity.get_public_key_hash().hex()
    send_frame(client, FRAME_TYPE["IDENTITY_RESPONSE"], my_pub)

    # 3. Receive IDENTITY_CONFIRM with client's pubkey + signature
    ft, payload = read_frame(client)
    assert ft == FRAME_TYPE["IDENTITY_CONFIRM"]
    # Payload: pubkey_pem + b"\n\n" + signature
    sep = b"\n\n"
    sep_pos = payload.find(sep)
    if sep_pos < 0:
        send_frame(client, FRAME_TYPE["ERROR"], b"Bad confirm format")
        raise ValueError("Bad IDENTITY_CONFIRM format")
    client_pub = payload[:sep_pos]
    client_sig = payload[sep_pos + len(sep):]
    client_hash = hashlib.sha256(client_pub).hexdigest()

    # Verify client's signature over challenge (their pubkey hash)
    challenge = client_pub  # simplified: sign the pubkey itself
    if not identity.verify(challenge, client_sig, client_pub):
        send_frame(client, FRAME_TYPE["ERROR"], b"Signature verification failed")
        raise PermissionError("Client signature invalid")

    # TOFU: trust the client
    if not trust.is_trusted(client_bdaddr, client_hash):
        send_frame(client, FRAME_TYPE["ERROR"], b"Device not trusted")
        raise PermissionError("Client not trusted")

    # 4. Send our signature for client to verify
    my_challenge = my_pub
    my_sig = identity.sign(my_challenge)
    send_frame(client, FRAME_TYPE["IDENTITY_RESPONSE"], my_pub + b"\n\n" + my_sig)

    # 5. Receive SESSION_INIT (client's ephemeral X25519 pubkey)
    ft, payload = read_frame(client)
    assert ft == FRAME_TYPE["SESSION_INIT"]
    client_eph = X25519PublicKey.from_public_bytes(payload)

    # Generate our ephemeral key and compute shared secret
    server_eph = X25519PrivateKey.generate()
    shared = CryptoSession.derive_session(server_eph, client_eph)

    # Send SESSION_READY with our ephemeral pubkey
    send_frame(client, FRAME_TYPE["SESSION_READY"],
               server_eph.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw))

    return CryptoSession(shared)


def client_handshake(sock: socket.socket, identity: TPMIdentity, server_bdaddr: str,
                     trust: TrustManager) -> CryptoSession:
    """Perform client-side handshake. Returns CryptoSession."""
    # 1. Send IDENTITY_REQUEST
    send_frame(sock, FRAME_TYPE["IDENTITY_REQUEST"])

    # 2. Receive IDENTITY_RESPONSE (server's pubkey)
    ft, server_pub = read_frame(sock)
    assert ft == FRAME_TYPE["IDENTITY_RESPONSE"]
    server_hash = hashlib.sha256(server_pub).hexdigest()

    # TOFU: trust the server
    if not trust.is_trusted(server_bdaddr, server_hash):
        send_frame(sock, FRAME_TYPE["ERROR"], b"Server not trusted")
        raise PermissionError("Server not trusted")

    # 3. Send IDENTITY_CONFIRM (our pubkey + signature)
    my_pub = identity.get_public_key_pem()
    my_sig = identity.sign(my_pub)  # sign our pubkey
    send_frame(sock, FRAME_TYPE["IDENTITY_CONFIRM"], my_pub + b"\n\n" + my_sig)

    # 4. Receive server's signature verification
    ft, payload = read_frame(sock)
    assert ft == FRAME_TYPE["IDENTITY_RESPONSE"]
    sep = b"\n\n"
    sep_pos = payload.find(sep)
    if sep_pos < 0:
        raise ValueError("Bad server response format")
    server_pub2 = payload[:sep_pos]
    server_sig = payload[sep_pos + len(sep):]
    if server_pub2 != server_pub:
        raise ValueError("Server pubkey mismatch")
    if not identity.verify(server_pub2, server_sig, server_pub):
        raise PermissionError("Server signature invalid")

    # 5. Send SESSION_INIT (our ephemeral X25519 pubkey)
    client_eph = X25519PrivateKey.generate()
    send_frame(sock, FRAME_TYPE["SESSION_INIT"],
               client_eph.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw))

    # 6. Receive SESSION_READY (server's ephemeral pubkey)
    ft, payload = read_frame(sock)
    assert ft == FRAME_TYPE["SESSION_READY"]
    server_eph = X25519PublicKey.from_public_bytes(payload)

    shared = CryptoSession.derive_session(client_eph, server_eph)
    return CryptoSession(shared)


def send_frame(sock: socket.socket, frame_type: int, payload: bytes = b""):
    data = encode_frame(frame_type, payload)
    sock.sendall(data)


# ---- Tunnel ----

def tunnel_stdio(sock: socket.socket, crypto: CryptoSession,
                 local_stdin: bool = False, local_stdout: bool = False):
    """Bidirectional stdio tunnel over encrypted stream."""

    def read_loop():
        buf = b""
        while True:
            try:
                chunk = sock.recv(8192)
            except (ConnectionError, OSError):
                break
            if not chunk:
                break
            buf += chunk
            while len(buf) >= HEADER_SIZE:
                try:
                    ft, payload, buf = decode_frame(buf)
                except ValueError:
                    break
                if ft == FRAME_TYPE["DATA"]:
                    try:
                        plain = crypto.decrypt(payload)
                    except Exception:
                        continue
                    if local_stdout:
                        sys.stdout.buffer.write(plain)
                        sys.stdout.buffer.flush()
                elif ft == FRAME_TYPE["CLOSE"]:
                    return
                elif ft == FRAME_TYPE["ERROR"]:
                    sys.stderr.buffer.write(b"Remote error: " + payload + b"\n")
                    sys.stderr.buffer.flush()
                    return

    def write_loop():
        while True:
            data = sys.stdin.buffer.read(4096)
            if not data:
                break
            encrypted = crypto.encrypt(data)
            send_frame(sock, FRAME_TYPE["DATA"], encrypted)
        send_frame(sock, FRAME_TYPE["CLOSE"])

    reader = threading.Thread(target=read_loop, daemon=True)
    reader.start()
    write_loop()
    reader.join(timeout=3)


# ---- SPP Transport ----

class SPPTransport:
    """Classic Bluetooth SPP transport."""

    def __init__(self, channel: int = 1):
        self.channel = channel
        self.sock = None

    def listen(self) -> socket.socket:
        self.sock = socket.socket(
            socket.AF_BLUETOOTH, socket.SOCK_STREAM, socket.BTPROTO_RFCOMM)
        self.sock.bind(("00:00:00:00:00:00", self.channel))
        self.sock.listen(1)
        self.sock.settimeout(10)
        return self.sock

    def accept(self) -> tuple:
        return self.sock.accept()

    def connect(self, bdaddr: str) -> socket.socket:
        self.sock = socket.socket(
            socket.AF_BLUETOOTH, socket.SOCK_STREAM, socket.BTPROTO_RFCOMM)
        self.sock.settimeout(30)
        self.sock.connect((bdaddr, self.channel))
        return self.sock

    def close(self):
        if self.sock:
            self.sock.close()


# ---- BLE GATT Transport (stub - uses bluetoothctl for now) ----

class BLETransport:
    """BLE GATT transport (server only). Requires bluetoothd --experimental."""

    def __init__(self):
        self._running = False

    def start(self):
        """Start BLE GATT service via bluetoothctl."""
        # For now, BLE is handled separately via the bridge
        pass


# ---- Server ----

def run_server(spp_channel: int = 1):
    identity = TPMIdentity()
    trust = TrustManager()
    identity.ensure_identity()

    my_hash = identity.get_public_key_hash().hex()
    print(f"BT-STDIO Server v1 — Identity: {my_hash}", flush=True)
    print(f"Listening on SPP channel {spp_channel}", flush=True)

    transport = SPPTransport(spp_channel)
    transport.listen()

    while True:
        try:
            client, addr = transport.accept()
            bdaddr = addr if isinstance(addr, str) else addr[0]
            print(f"Connection from {bdaddr}", flush=True)
        except socket.timeout:
            continue
        except Exception as e:
            print(f"Accept error: {e}", flush=True)
            continue

        def handle_client():
            try:
                crypto = server_handshake(client, identity, trust, bdaddr)
                print(f"Secure session established with {bdaddr}", flush=True)
                tunnel_stdio(client, crypto, local_stdout=True)
            except (PermissionError, ValueError, ConnectionError) as e:
                print(f"Session error from {bdaddr}: {e}", flush=True)
            finally:
                try:
                    client.close()
                except Exception:
                    pass

        t = threading.Thread(target=handle_client, daemon=True)
        t.start()


# ---- Client ----

def run_client(bdaddr: str, spp_channel: int = 1, command: Optional[list] = None):
    identity = TPMIdentity()
    trust = TrustManager()
    identity.ensure_identity()

    my_hash = identity.get_public_key_hash().hex()
    print(f"BT-STDIO Client v1 — Identity: {my_hash}", flush=True)

    transport = SPPTransport(spp_channel)
    sock = transport.connect(bdaddr)
    print(f"Connected to {bdaddr}", flush=True)

    try:
        crypto = client_handshake(sock, identity, bdaddr, trust)
        print("Secure session established", flush=True)

        if command:
            # Execute local command, tunnel its stdio
            proc = subprocess.Popen(
                command,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
            )

            def pipe_stdin():
                while True:
                    data = proc.stdout.read(4096)
                    if not data:
                        break
                    encrypted = crypto.encrypt(data)
                    send_frame(sock, FRAME_TYPE["DATA"], encrypted)
                send_frame(sock, FRAME_TYPE["CLOSE"])

            def pipe_remote():
                buf = b""
                while True:
                    try:
                        chunk = sock.recv(8192)
                    except (ConnectionError, OSError):
                        break
                    if not chunk:
                        break
                    buf += chunk
                    while len(buf) >= HEADER_SIZE:
                        try:
                            ft, payload, buf = decode_frame(buf)
                        except ValueError:
                            break
                        if ft == FRAME_TYPE["DATA"]:
                            try:
                                plain = crypto.decrypt(payload)
                            except Exception:
                                continue
                            proc.stdin.write(plain)
                            proc.stdin.flush()
                        elif ft == FRAME_TYPE["CLOSE"]:
                            proc.terminate()
                            return

            t1 = threading.Thread(target=pipe_stdin, daemon=True)
            t2 = threading.Thread(target=pipe_remote, daemon=True)
            t1.start()
            t2.start()
            t1.join()
            t2.join(timeout=5)
            proc.wait(5)
        else:
            tunnel_stdio(sock, crypto, local_stdin=True, local_stdout=True)
    finally:
        sock.close()


# ---- CLI ----

def show_identity():
    identity = TPMIdentity()
    pub_hash = identity.ensure_identity().hex()
    pub_key = identity.get_public_key_pem()
    print(f"Device Identity: {pub_hash}")
    print(f"Public Key (PEM):")
    print(pub_key.decode())

    ek_pub = identity.get_ek_pub()
    ek_hash = hashlib.sha256(ek_pub).hexdigest()
    print(f"EK Fingerprint: {ek_hash}")


def main():
    if len(sys.argv) < 2:
        print("Usage:")
        print(f"  {sys.argv[0]} server [--spp-channel=N]")
        print(f"  {sys.argv[0]} client <bdaddr> [--spp-channel=N] [-- command...]")
        print(f"  {sys.argv[0]} identity [--show]")
        sys.exit(1)

    mode = sys.argv[1]
    spp_channel = 1
    command = None

    extra_args = sys.argv[2:]
    filtered = []
    for arg in extra_args:
        if arg.startswith("--spp-channel="):
            spp_channel = int(arg.split("=")[1])
        elif arg == "--":
            command = []
        elif command is not None:
            command.append(arg)
        else:
            filtered.append(arg)

    if mode == "server":
        run_server(spp_channel)
    elif mode == "client":
        if not filtered:
            print("Usage: bt_stdio.py client <bdaddr>")
            sys.exit(1)
        bdaddr = filtered[0]
        run_client(bdaddr, spp_channel, command)
    elif mode == "identity":
        show_identity()
    else:
        print(f"Unknown mode: {mode}")
        sys.exit(1)


if __name__ == "__main__":
    main()
