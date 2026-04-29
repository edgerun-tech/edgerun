#!/usr/bin/env bash
# Smoke-test marketplace CLI signing and RPC flow against scripts/mock-solana-rpc.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MOCK_ADDR="${MOCK_SOLANA_RPC_ADDR:-127.0.0.1:18899}"
MOCK_URL="http://$MOCK_ADDR"
WORK_DIR="${MARKETPLACE_MOCK_WORK_DIR:-$ROOT_DIR/target/marketplace-mock}"
KEYPAIR="$WORK_DIR/mock-keypair.bin"
PROVIDER_KEYPAIR="$WORK_DIR/mock-provider-keypair.bin"
MOCK_LOG="$WORK_DIR/mock-solana-rpc.log"
MOCK_PID=""
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"

mkdir -p "$WORK_DIR"

cleanup() {
  if [ -n "$MOCK_PID" ] && kill -0 "$MOCK_PID" 2>/dev/null; then
    kill "$MOCK_PID" 2>/dev/null || true
    wait "$MOCK_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

python3 - "$KEYPAIR" <<'PY'
import sys
with open(sys.argv[1], "wb") as f:
    f.write(bytes(range(32)))
PY

python3 - "$PROVIDER_KEYPAIR" <<'PY'
import sys
with open(sys.argv[1], "wb") as f:
    f.write(bytes(range(32, 64)))
PY

cargo build --manifest-path "$ROOT_DIR/scripts/mock-solana-rpc/Cargo.toml"
MOCK_BIN="$(
  {
    find "$TARGET_DIR" -path '*/debug/mock-solana-rpc' -type f -perm -111 2>/dev/null
    if [ -d "$ROOT_DIR/scripts/mock-solana-rpc/target" ]; then
      find "$ROOT_DIR/scripts/mock-solana-rpc/target" -path '*/debug/mock-solana-rpc' -type f -perm -111 2>/dev/null
    fi
    true
  } \
  | head -n 1
)"
if [ ! -x "$MOCK_BIN" ]; then
  echo "failed to locate built mock-solana-rpc binary" >&2
  exit 1
fi
MOCK_SOLANA_RPC_ADDR="$MOCK_ADDR" \
  "$MOCK_BIN" \
  >"$MOCK_LOG" 2>&1 &
MOCK_PID="$!"

for _ in $(seq 1 50); do
  if bash -c ":</dev/tcp/${MOCK_ADDR/:/\/}" 2>/dev/null; then
    break
  fi
  sleep 0.1
done
bash -c ":</dev/tcp/${MOCK_ADDR/:/\/}" 2>/dev/null

cargo build -p edgerun-marketplace-cli --features std
CLI_BIN="$(find "$TARGET_DIR" -path '*/debug/edgerun-marketplace' -type f -perm -111 | head -n 1)"
if [ ! -x "$CLI_BIN" ]; then
  echo "failed to locate built edgerun-marketplace binary" >&2
  exit 1
fi

export SOLANA_RPC_URL="$MOCK_URL"
export SOLANA_KEYPAIR="$KEYPAIR"
export EDGERUN_ALLOW_DEV_PROGRAM_IDS=1

RUN_ID="mock-$(date +%s)"
PROVIDER_SEED="provider-$RUN_ID"
DEPLOYMENT_SEED="deployment-$RUN_ID"

echo "Using mock Solana RPC at $MOCK_URL"

PROVIDER_OUT="$(SOLANA_KEYPAIR="$PROVIDER_KEYPAIR" "$CLI_BIN" provider register "$PROVIDER_SEED" \
  --cpu-cores 2 \
  --memory 1073741824 \
  --storage 2147483648 \
  --network 10)"
echo "$PROVIDER_OUT"
PROVIDER="$(awk '/Provider account:/ {print $NF; exit}' <<<"$PROVIDER_OUT")"
test -n "$PROVIDER"
grep -Fq "Transaction confirmed:" <<<"$PROVIDER_OUT"

CREATE_OUT="$("$CLI_BIN" deployment create "$DEPLOYMENT_SEED" \
  --name "$DEPLOYMENT_SEED" \
  --cpu 1 \
  --memory 536870912 \
  --storage 1073741824 \
  --deposit 1000000)"
echo "$CREATE_OUT"
DEPLOYMENT="$(awk '/Deployment account:/ {print $NF; exit}' <<<"$CREATE_OUT")"
test -n "$DEPLOYMENT"
grep -Fq "Transaction confirmed:" <<<"$CREATE_OUT"

"$CLI_BIN" deployment assign "$DEPLOYMENT" --provider "$PROVIDER"
"$CLI_BIN" deployment start "$DEPLOYMENT"
SOLANA_KEYPAIR="$PROVIDER_KEYPAIR" "$CLI_BIN" deployment report "$DEPLOYMENT" \
  --provider "$PROVIDER" \
  --cpu 1 \
  --memory 536870912 \
  --storage 1073741824 \
  --network 2048 \
  --containers 1
"$CLI_BIN" deployment stop "$DEPLOYMENT" --provider-payout-recipient "$PROVIDER"
"$CLI_BIN" deployment burn-rate --cpu 1 --memory 536870912 --storage 1073741824 --network 10

echo "Marketplace mock e2e smoke test passed."
