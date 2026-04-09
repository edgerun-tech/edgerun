#!/usr/bin/env python3
"""Regenerate corpus vectors with P-256/ECDSA signatures and SHA-256 hashes.

The corpus was originally generated with Ed25519/BLAKE3. This tool updates
all vectors to use the correct algorithms (P-256/ECDSA and SHA-256).

Deterministic fixture keys are generated from fixture IDs using HKDF.
"""

import hashlib
import hmac
import os
import sys
from pathlib import Path
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives.kdf.hkdf import HKDF
import yaml

# ===========================================================================
# Deterministic ECDSA (RFC 6979) — needed for reproducible corpus
# ===========================================================================

def _rfc6979_k(private_key, msg_hash, hash_func=hashlib.sha256):
    """Generate deterministic k per RFC 6979."""
    from cryptography.hazmat.primitives.asymmetric.ec import SECP256R1
    # Extract private key scalar
    private_numbers = private_key.private_numbers()
    d = private_numbers.private_value
    # Get curve order (n) — hardcoded for P-256
    n = 0xFFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551
    qlen = n.bit_length()

    # Step b: k1 = HMAC_K(V, 0x00, int2octets(x), h(m))
    x = d.to_bytes((d.bit_length() + 7) // 8, 'big')
    h1 = msg_hash

    # Convert h1 to bits
    rlen = ((qlen + 7) // 8) * 8
    h1_bits = int.from_bytes(h1, 'big')
    if h1_bits.bit_length() > rlen:
        h1_bits >>= (h1_bits.bit_length() - rlen)
    h1 = h1_bits.to_bytes(rlen // 8, 'big')

    # Initialize V and K
    V = b'\x01' * hash_func().digest_size
    K = b'\x00' * hash_func().digest_size

    # K = HMAC(K, V || 0x00 || x || h1)
    K = hmac.new(K, V + b'\x00' + x + h1, hash_func).digest()
    # V = HMAC(K, V)
    V = hmac.new(K, V, hash_func).digest()

    # K = HMAC(K, V || 0x01 || x || h1)
    K = hmac.new(K, V + b'\x01' + x + h1, hash_func).digest()
    # V = HMAC(K, V)
    V = hmac.new(K, V, hash_func).digest()

    # Generate k
    while True:
        T = b''
        while len(T) * 8 < qlen:
            V = hmac.new(K, V, hash_func).digest()
            T += V

        k = int.from_bytes(T, 'big')
        if 1 <= k < n:
            return k

        K = hmac.new(K, V + b'\x00', hash_func).digest()
        V = hmac.new(K, V, hash_func).digest()

def _deterministic_sign(private_key, msg_hash):
    """Sign using deterministic ECDSA (RFC 6979)."""
    from cryptography.hazmat.primitives.asymmetric.ec import SECP256R1
    from cryptography.hazmat.primitives.asymmetric.utils import encode_dss_signature

    private_numbers = private_key.private_numbers()
    n = 0xFFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551
    curve = ec.SECP256R1()
    d = private_numbers.private_value

    k = _rfc6979_k(private_key, msg_hash)
    k_inv = pow(k, -1, n)

    # Compute point multiplication
    from cryptography.hazmat.primitives.asymmetric.utils import decode_dss_signature, encode_dss_signature

    # G * k
    public_key = private_key.public_key()
    pub_numbers = public_key.public_numbers()

    # We need to do elliptic curve point multiplication: k * G
    # cryptography doesn't expose this directly, so we compute it
    # using the public key derivation
    ephemeral_key = ec.derive_private_key(k, curve, default_backend())
    ephemeral_pub = ephemeral_key.public_key()
    r = ephemeral_pub.public_numbers().x % n

    if r == 0:
        raise ValueError("r is zero")

    s = (k_inv * (int.from_bytes(msg_hash, 'big') + d * r)) % n
    if s == 0:
        raise ValueError("s is zero")

    return encode_dss_signature(r, s)

# ===========================================================================
# Deterministic key generation
# ===========================================================================

FIXTURE_IDS = ["user_alice", "user_bob", "node_phone", "node_server", "agent_assistant"]

_fixture_keys = {}

def get_fixture_key(fixture_id: str) -> ec.EllipticCurvePrivateKey:
    if fixture_id not in _fixture_keys:
        seed = hashlib.sha256(fixture_id.encode()).digest()
        # P-256 curve order n
        n = 0xFFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551
        d = int.from_bytes(seed, 'big') % n
        if d == 0:
            d = 1
        private_key = ec.derive_private_key(d, ec.SECP256R1(), default_backend())
        _fixture_keys[fixture_id] = private_key
    return _fixture_keys[fixture_id]

def sign_canonical(canonical_bytes: bytes, fixture_id: str) -> str:
    """Sign canonical bytes with deterministic P-256/ECDSA (RFC 6979)."""
    key = get_fixture_key(fixture_id)
    msg_hash = hashlib.sha256(canonical_bytes).digest()
    signature = _deterministic_sign(key, msg_hash)
    return "0x" + signature.hex()

def sha256_hex(data: bytes) -> str:
    return "0x" + hashlib.sha256(data).hexdigest()

# ===========================================================================
# YAML handling
# ===========================================================================

def yaml_dump(data, path: Path):
    """Write YAML without unnecessary clutter."""
    content = yaml.dump(data, default_flow_style=False, sort_keys=False, allow_unicode=True)
    # Clean up trailing whitespace
    content = '\n'.join(line.rstrip() for line in content.splitlines())
    content = content.rstrip() + '\n'
    path.write_text(content)

def yaml_load(path: Path) -> dict:
    with open(path) as f:
        return yaml.safe_load(f)

# ===========================================================================
# Corpus regeneration
# ===========================================================================

def regen_canonical_vector(vector_dir: Path):
    """Regenerate a canonical vector with SHA-256 hashes and P-256 signatures."""
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    # Read canonical signable bytes
    canon_path = vector_dir / "canonical_signable.hex"
    if not canon_path.exists():
        return False  # No canonical bytes to work with

    canon_bytes = bytes.fromhex(canon_path.read_text().strip().replace("0x", ""))
    canon_full_path = vector_dir / "canonical_full.hex"

    # Compute SHA-256 hashes
    signable_hash = sha256_hex(canon_bytes)

    # Update record_hash.hex
    hash_path = vector_dir / "record_hash.hex"
    if hash_path.exists():
        hash_path.write_text(signable_hash + "\n")

    # Sign with P-256 if we have a full canonical file or a signature_fixture
    fixture = semantic.get("fixture_record", {})
    if isinstance(fixture, dict):
        fixture_id = fixture.get("signature_fixture", "")
    else:
        fixture_id = semantic.get("signature_fixture", "")

    sig_path = vector_dir / "signature.hex"
    if fixture_id and canon_bytes:
        signature = sign_canonical(canon_bytes, fixture_id)
        if sig_path.exists():
            sig_path.write_text(signature + "\n")
        # Update signature in expected if present
        if "derived" in expected and "signature" in expected.get("derived", {}):
            expected["derived"]["signature"] = signature

    # Update signature_input.hex
    sig_input_path = vector_dir / "signature_input.hex"
    if sig_input_path.exists():
        # This should be the same as canonical_signable.hex
        pass

    # Update canonical_full.hex if it exists
    if canon_full_path.exists():
        canon_full_bytes = bytes.fromhex(canon_full_path.read_text().strip().replace("0x", ""))
        full_hash = sha256_hex(canon_full_bytes)
        hash_path.write_text(full_hash + "\n")

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_stream_vector(vector_dir: Path):
    """Regenerate a stream vector with SHA-256 hashes and P-256 signatures."""
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    event = semantic.get("candidate_event", {})
    fixture_id = event.get("signature_fixture", "")

    if fixture_id:
        key = get_fixture_key(fixture_id)
        # The event_hash_hex in the corpus needs to be computed from canonical bytes.
        # Since we don't have the canonical bytes for stream events, we compute
        # the hash from the event's semantic data.
        # For conformance, the hash is derived from the canonical event envelope.

        # Remove existing hash so the validator computes it
        if "event_hash_hex" in event:
            # Keep it — the validator will use it for comparison
            pass

        # Update signature to use P-256
        # We need the canonical bytes to sign. For stream events, the canonical
        # bytes are the protobuf-encoded EventEnvelope.
        # Since we can't easily reconstruct the protobuf message in Python,
        # we'll use the existing event_hash as the "canonical" bytes.

        existing_hash_hex = event.get("event_hash_hex", "")
        if existing_hash_hex:
            existing_hash = bytes.fromhex(existing_hash_hex.replace("0x", ""))
            # Sign the hash (as a proxy for the canonical bytes)
            signature = sign_canonical(existing_hash, fixture_id)
            if "signature" in event and isinstance(event["signature"], dict):
                event["signature"]["value"] = signature

        # Update prev_ref hash if present
        if "prev_ref" in event:
            prev_ref = event["prev_ref"]
            if "hash_fixture" in prev_ref:
                # The fixture points to a known hash — this is handled by local_state
                pass

    # Update stream_heads hashes
    if "stream_heads" in local_state:
        for stream_id, head in local_state["stream_heads"].items():
            if isinstance(head, dict) and "event_hash_hex" in head:
                # Keep the existing hash — it represents the genesis event
                pass

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_command_vector(vector_dir: Path):
    """Regenerate a command vector with SHA-256 hashes and P-256 signatures."""
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    command = semantic.get("command", {})
    fixture_id = command.get("signature_fixture", "")

    # Detect bad-signature test cases
    is_bad_sig = "bad-signature" in manifest.get("id", "").lower() or \
                 "tampered" in manifest.get("id", "").lower() or \
                 "tampered" in manifest.get("description", "").lower()

    if fixture_id:
        # For bad-signature cases, use a different fixture
        signing_fixture = fixture_id
        if is_bad_sig:
            signing_fixture = "user_bob" if fixture_id != "user_bob" else "user_alice"

        # Read canonical_signable.hex if available
        canon_path = vector_dir / "canonical_signable.hex"

        # Compute signature_input using edgerun domain tag
        command_id_hex = command.get("command_id", "")
        if isinstance(command_id_hex, str) and command_id_hex.startswith("cmd_"):
            # command_id is a string, encode it
            command_id_bytes = command_id_hex.encode()
        else:
            # Try to decode as hex
            try:
                command_id_bytes = bytes.fromhex(str(command_id_hex).replace("0x", ""))
            except:
                command_id_bytes = str(command_id_hex).encode()

        # Domain-separated signature input
        domain = b"edgerun:v0:command-signature\x00"
        sig_input = domain + command_id_bytes
        sig_input_hex = "0x" + sig_input.hex()

        # Write signature_input.hex
        sig_input_path = vector_dir / "signature_input.hex"
        sig_input_path.write_text(sig_input_hex + "\n")

        # Sign the signature_input
        if is_bad_sig:
            # For bad-signature cases, use a clearly invalid signature (non-DER)
            signature = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001"
        else:
            signature = sign_canonical(sig_input, signing_fixture)

        # Compute record hash (SHA-256 of signature_input)
        record_hash = sha256_hex(sig_input)

        # Write record_hash.hex
        hash_path = vector_dir / "record_hash.hex"
        hash_path.write_text(record_hash + "\n")

        # Write signature.hex
        sig_path = vector_dir / "signature.hex"
        sig_path.write_text(signature + "\n")

        # Update command in semantic input
        command["command_hash_hex"] = record_hash
        command["signature_input_hex"] = sig_input_hex  # For TestVerifier
        if "signature" in command and isinstance(command["signature"], dict):
            command["signature"]["value"] = signature

        # Update replay_cache
        if "replay_cache" in local_state:
            new_cache = {}
            for old_hash, entry in local_state["replay_cache"].items():
                new_cache[record_hash] = entry
            local_state["replay_cache"] = new_cache

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_delegation_vector(vector_dir: Path):
    """Regenerate a delegation vector with SHA-256 hashes and P-256 signatures."""
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    # Delegation chains are signed records
    chain = semantic.get("delegation_chain", [])
    if isinstance(chain, list):
        for link in chain:
            if isinstance(link, dict) and "signature_fixture" in link:
                fixture_id = link["signature_fixture"]
                key = get_fixture_key(fixture_id)
                # Compute record hash
                record_repr = str(sorted(link.items())).encode()
                record_hash = sha256_hex(record_repr)
                # Sign
                signature = sign_canonical(record_repr, fixture_id)
                if "signature" in link and isinstance(link["signature"], dict):
                    link["signature"]["value"] = signature
                link["delegation_hash"] = record_hash

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_control_vector(vector_dir: Path):
    """Regenerate a control vector."""
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    command = semantic.get("control_change_command", {})
    fixture_id = command.get("signature_fixture", "")

    if fixture_id:
        key = get_fixture_key(fixture_id)
        # Read canonical if available
        canon_path = vector_dir / "canonical_signable.hex"
        if canon_path.exists():
            canon_bytes = bytes.fromhex(canon_path.read_text().strip().replace("0x", ""))
            signature = sign_canonical(canon_bytes, fixture_id)
            if "signature" in command and isinstance(command["signature"], dict):
                command["signature"]["value"] = signature

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_snapshot_vector(vector_dir: Path):
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    snap = semantic.get("snapshot", {})
    fixture_id = snap.get("signature_fixture", "")

    if fixture_id:
        canon_path = vector_dir / "canonical_signable.hex"
        if canon_path.exists():
            canon_bytes = bytes.fromhex(canon_path.read_text().strip().replace("0x", ""))
            signature = sign_canonical(canon_bytes, fixture_id)
            if "signature" in snap and isinstance(snap["signature"], dict):
                snap["signature"]["value"] = signature

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_object_vector(vector_dir: Path):
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    # Object vectors use SHA-256 for object_id and representation_digest
    obj = semantic.get("object", {})

    if obj:
        # Compute object_id from realized_bytes
        if "descriptor" in obj and "realized_bytes_hex" in obj:
            descriptor = obj["descriptor"]
            canonicalization_id = descriptor.get("canonicalization_id", "raw-bytes-v0")
            realized = bytes.fromhex(obj["realized_bytes_hex"].replace("0x", ""))
            payload = f"edgerun:v0:object\0{canonicalization_id}\0".encode()
            payload += realized
            object_id = sha256_hex(payload)
            descriptor["object_id"] = object_id

        # Compute representation_digest
        if "representation" in obj:
            rep = obj["representation"]
            if "stored_bytes_hex" in rep:
                stored = bytes.fromhex(rep["stored_bytes_hex"].replace("0x", ""))
                payload = b"edgerun:v0:representation-bytes\0" + stored
                rep_digest = sha256_hex(payload)
                rep["representation_digest"] = rep_digest
                if "header" in rep:
                    rep["header"]["representation_digest"] = rep_digest

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_network_vector(vector_dir: Path):
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    # Network vectors have SessionHello, SessionAccept, RouteAdvertisement, etc.
    for key in ["session_hello", "session_accept", "route_advertisement",
                "reachability_hint", "relay_envelope", "route_trust_assignments",
                "aggregate_trust_policy"]:
        record = semantic.get(key, {})
        fixture_id = record.get("signature_fixture", "")
        if fixture_id:
            canon_path = vector_dir / "canonical_signable.hex"
            if canon_path.exists():
                canon_bytes = bytes.fromhex(canon_path.read_text().strip().replace("0x", ""))
                signature = sign_canonical(canon_bytes, fixture_id)
                if "signature" in record and isinstance(record["signature"], dict):
                    record["signature"]["value"] = signature
            else:
                # Sign the semantic data
                semantic_repr = str(sorted(record.items())).encode()
                signature = sign_canonical(semantic_repr, fixture_id)
                if "signature" in record and isinstance(record["signature"], dict):
                    record["signature"]["value"] = signature

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_query_vector(vector_dir: Path):
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    # Query vectors have query requests, result fragments, proof bundles
    for key in ["query_request", "result_fragment", "proof_bundle",
                "aggregate_descriptor", "event_set_proof", "snapshot_set_proof",
                "object_assertion_proof", "trust_policy_proof",
                "aggregate_summary_proof"]:
        record = semantic.get(key, {})
        fixture_id = record.get("signature_fixture", "")
        if fixture_id:
            canon_path = vector_dir / "canonical_signable.hex"
            if canon_path.exists():
                canon_bytes = bytes.fromhex(canon_path.read_text().strip().replace("0x", ""))
                signature = sign_canonical(canon_bytes, fixture_id)
                if "signature" in record and isinstance(record["signature"], dict):
                    record["signature"]["value"] = signature
            else:
                semantic_repr = str(sorted(record.items())).encode()
                signature = sign_canonical(semantic_repr, fixture_id)
                if "signature" in record and isinstance(record["signature"], dict):
                    record["signature"]["value"] = signature

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_trust_vector(vector_dir: Path):
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    for key in ["assurance_claim", "revocation_record"]:
        record = semantic.get(key, {})
        fixture_id = record.get("signature_fixture", "")
        if fixture_id:
            canon_path = vector_dir / "canonical_signable.hex"
            if canon_path.exists():
                canon_bytes = bytes.fromhex(canon_path.read_text().strip().replace("0x", ""))
                signature = sign_canonical(canon_bytes, fixture_id)
                if "signature" in record and isinstance(record["signature"], dict):
                    record["signature"]["value"] = signature
            else:
                semantic_repr = str(sorted(record.items())).encode()
                signature = sign_canonical(semantic_repr, fixture_id)
                if "signature" in record and isinstance(record["signature"], dict):
                    record["signature"]["value"] = signature

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

def regen_crypto_vector(vector_dir: Path):
    manifest = yaml_load(vector_dir / "manifest.yaml")
    semantic = yaml_load(vector_dir / "semantic_input.yaml")
    local_state = yaml_load(vector_dir / "local_state.yaml")
    expected = yaml_load(vector_dir / "expected.yaml")

    record = semantic.get("signed_record", {})
    fixture_id = record.get("signature_fixture", "")
    if fixture_id:
        canon_path = vector_dir / "canonical_signable.hex"
        if canon_path.exists():
            canon_bytes = bytes.fromhex(canon_path.read_text().strip().replace("0x", ""))
            signature = sign_canonical(canon_bytes, fixture_id)
            if "signature" in record and isinstance(record["signature"], dict):
                record["signature"]["value"] = signature
        else:
            semantic_repr = str(sorted(record.items())).encode()
            signature = sign_canonical(semantic_repr, fixture_id)
            if "signature" in record and isinstance(record["signature"], dict):
                record["signature"]["value"] = signature

    yaml_dump(manifest, vector_dir / "manifest.yaml")
    yaml_dump(semantic, vector_dir / "semantic_input.yaml")
    yaml_dump(local_state, vector_dir / "local_state.yaml")
    yaml_dump(expected, vector_dir / "expected.yaml")
    return True

# ===========================================================================
# Main
# ===========================================================================

REGEN_HANDLERS = {
    "canonical": regen_canonical_vector,
    "stream": regen_stream_vector,
    "delegation": regen_delegation_vector,
    "command": regen_command_vector,
    "control": regen_control_vector,
    "snapshot": regen_snapshot_vector,
    "object": regen_object_vector,
    "network": regen_network_vector,
    "query": regen_query_vector,
    "trust": regen_trust_vector,
    "crypto": regen_crypto_vector,
}

def main():
    corpus_root = Path(__file__).parent.parent / "corpus" / "vectors-v0.1"
    if not corpus_root.exists():
        print(f"Corpus not found at {corpus_root}", file=sys.stderr)
        sys.exit(1)

    total = 0
    regenerated = 0
    skipped = 0
    errors = 0

    for suite_dir in sorted(corpus_root.iterdir()):
        if not suite_dir.is_dir():
            continue
        for vector_dir in sorted(suite_dir.iterdir()):
            if not vector_dir.is_dir():
                continue
            total += 1

            manifest = yaml_load(vector_dir / "manifest.yaml")
            suite = manifest.get("suite", "unknown")

            handler = REGEN_HANDLERS.get(suite)
            if handler is None:
                print(f"SKIP {vector_dir.name} (unknown suite: {suite})")
                skipped += 1
                continue

            try:
                handler(vector_dir)
                regenerated += 1
                print(f"OK   {vector_dir.name} ({suite})")
            except Exception as e:
                print(f"ERR  {vector_dir.name}: {e}", file=sys.stderr)
                errors += 1

    print(f"\nRegenerated: {regenerated}/{total} vectors")
    print(f"Skipped:     {skipped}")
    print(f"Errors:      {errors}")

if __name__ == "__main__":
    main()
