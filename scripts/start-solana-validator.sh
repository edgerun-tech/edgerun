#!/usr/bin/env bash
# Start a local Solana validator with Edgerun marketplace programs loaded.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOLANA_BIN_DIR="${SOLANA_BIN_DIR:-$HOME/.local/share/solana/install/active_release/bin}"
if [ -d "$SOLANA_BIN_DIR" ]; then
  export PATH="$SOLANA_BIN_DIR:$PATH"
fi

RPC_PORT="${SOLANA_RPC_PORT:-8899}"
FAUCET_PORT="${SOLANA_FAUCET_PORT:-9900}"
LEDGER_DIR="${SOLANA_LEDGER_DIR:-$ROOT_DIR/target/localnet/ledger}"
ENV_FILE="${SOLANA_LOCALNET_ENV:-$ROOT_DIR/target/localnet/marketplace.env}"
LOG_FILE="${SOLANA_LOCALNET_LOG:-$ROOT_DIR/target/localnet/validator.log}"
PID_FILE="${SOLANA_LOCALNET_PID:-$ROOT_DIR/target/localnet/validator.pid}"
KEYPAIR="${SOLANA_KEYPAIR:-$HOME/.config/solana/id.json}"
COMMAND="${1:-start}"

DEPLOYMENT_MANIFEST="$ROOT_DIR/programs/deployment/Cargo.toml"
PROVIDER_MANIFEST="$ROOT_DIR/programs/provider_registry/Cargo.toml"
DEPLOYMENT_SO="$ROOT_DIR/programs/deployment/target/deploy/deployment_contract.so"
DEPLOYMENT_KEYPAIR="$ROOT_DIR/programs/deployment/target/deploy/deployment_contract-keypair.json"
PROVIDER_SO="$ROOT_DIR/programs/provider_registry/target/deploy/provider_registry.so"
PROVIDER_KEYPAIR="$ROOT_DIR/programs/provider_registry/target/deploy/provider_registry-keypair.json"

mkdir -p "$(dirname "$ENV_FILE")" "$(dirname "$LOG_FILE")" "$(dirname "$PID_FILE")"

case "$COMMAND" in
  stop)
    if [ -f "$PID_FILE" ]; then
      PID="$(cat "$PID_FILE")"
      if kill -0 "$PID" 2>/dev/null; then
        kill "$PID"
        echo "Stopped local validator PID $PID"
      else
        echo "No running validator for stale PID $PID"
      fi
      rm -f "$PID_FILE"
    else
      echo "No local validator PID file found"
    fi
    exit 0
    ;;
  status)
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
      echo "Local validator running with PID $(cat "$PID_FILE")"
      echo "RPC: http://127.0.0.1:$RPC_PORT"
      echo "Log: $LOG_FILE"
      echo "Env: $ENV_FILE"
      exit 0
    fi
    echo "Local validator is not running"
    exit 1
    ;;
  start)
    ;;
  *)
    echo "Usage: $0 [start|stop|status]" >&2
    exit 2
    ;;
esac

command -v solana >/dev/null
command -v solana-keygen >/dev/null
command -v solana-test-validator >/dev/null

if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null && [ -f "$ENV_FILE" ]; then
  echo "Validator already running with PID $(cat "$PID_FILE")"
  echo "Env: $ENV_FILE"
  exit 0
fi

command -v cargo-build-sbf >/dev/null

mkdir -p "$(dirname "$KEYPAIR")"

if [ ! -f "$KEYPAIR" ]; then
  solana-keygen new --no-bip39-passphrase --silent --outfile "$KEYPAIR"
fi

echo "Building marketplace programs..."
cargo-build-sbf --manifest-path "$DEPLOYMENT_MANIFEST"
cargo-build-sbf --manifest-path "$PROVIDER_MANIFEST"

DEPLOYMENT_PROGRAM_ID="$(solana address -k "$DEPLOYMENT_KEYPAIR")"
PROVIDER_PROGRAM_ID="$(solana address -k "$PROVIDER_KEYPAIR")"
VALIDATOR_IDENTITY="$(solana address -k "$KEYPAIR")"

if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
  echo "Validator already running with PID $(cat "$PID_FILE")"
else
  rm -rf "$LEDGER_DIR"
  echo "Starting local validator..."
  setsid solana-test-validator \
    --ledger "$LEDGER_DIR" \
    --rpc-port "$RPC_PORT" \
    --faucet-port "$FAUCET_PORT" \
    --reset \
    --quiet \
    --mint "$VALIDATOR_IDENTITY" \
    --bpf-program "$DEPLOYMENT_PROGRAM_ID" "$DEPLOYMENT_SO" \
    --bpf-program "$PROVIDER_PROGRAM_ID" "$PROVIDER_SO" \
    >"$LOG_FILE" 2>&1 </dev/null &
  echo "$!" >"$PID_FILE"
fi

RPC_URL="http://127.0.0.1:$RPC_PORT"
echo "Waiting for validator RPC at $RPC_URL..."
for _ in $(seq 1 60); do
  if solana --url "$RPC_URL" cluster-version >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
solana --url "$RPC_URL" cluster-version >/dev/null
if ! kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
  echo "validator exited after RPC readiness check; recent log:" >&2
  tail -n 80 "$LOG_FILE" >&2 || true
  exit 1
fi

cat >"$ENV_FILE" <<EOF
export SOLANA_RPC_URL=$RPC_URL
export SOLANA_KEYPAIR=$KEYPAIR
export EDGERUN_DEPLOYMENT_PROGRAM_ID=$DEPLOYMENT_PROGRAM_ID
export EDGERUN_PROVIDER_REGISTRY_PROGRAM_ID=$PROVIDER_PROGRAM_ID
EOF

echo "Local validator ready."
echo "PID: $(cat "$PID_FILE")"
echo "Log: $LOG_FILE"
echo "Env: $ENV_FILE"
echo "RPC: $RPC_URL"
echo "Deployment program: $DEPLOYMENT_PROGRAM_ID"
echo "Provider registry program: $PROVIDER_PROGRAM_ID"
echo
echo "Use:"
echo "  source $ENV_FILE"
echo "Stop:"
echo "  $0 stop"
