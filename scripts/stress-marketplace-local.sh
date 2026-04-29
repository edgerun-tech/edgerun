#!/usr/bin/env bash
# Exercise provider/deployment lifecycle repeatedly against local Solana.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -n "${SOLANA_BIN_DIR:-}" ]; then
  PATH="$SOLANA_BIN_DIR:$PATH"
fi
ENV_FILE="${SOLANA_LOCALNET_ENV:-$ROOT_DIR/target/localnet/marketplace.env}"
ITERATIONS="${1:-${MARKETPLACE_STRESS_ITERATIONS:-25}}"
DEPOSIT="${MARKETPLACE_STRESS_DEPOSIT:-10000000}"
CPU="${MARKETPLACE_STRESS_CPU:-1}"
MEMORY="${MARKETPLACE_STRESS_MEMORY:-1073741824}"
STORAGE="${MARKETPLACE_STRESS_STORAGE:-1073741824}"
NETWORK="${MARKETPLACE_STRESS_NETWORK_MBPS:-10}"
REPORT_NETWORK_BYTES="${MARKETPLACE_STRESS_REPORT_NETWORK_BYTES:-2048}"
REPORT_DELAY_SECONDS="${MARKETPLACE_STRESS_REPORT_DELAY_SECONDS:-0}"
PRICE_GUARD_CHECK="${MARKETPLACE_STRESS_PRICE_GUARD_CHECK:-1}"
CLI_BIN="${MARKETPLACE_CLI_BIN:-$ROOT_DIR/target/debug/edgerun-marketplace}"
PROVIDER_KEYPAIR="${MARKETPLACE_PROVIDER_KEYPAIR:-$ROOT_DIR/target/localnet/provider-authority.json}"

if [ -f "$ENV_FILE" ]; then
  # shellcheck disable=SC1090
  source "$ENV_FILE"
fi

: "${SOLANA_RPC_URL:=http://127.0.0.1:8899}"
: "${SOLANA_KEYPAIR:=$HOME/.config/solana/id.json}"
: "${EDGERUN_DEPLOYMENT_PROGRAM_ID:?missing EDGERUN_DEPLOYMENT_PROGRAM_ID; run scripts/start-solana-validator.sh first}"
: "${EDGERUN_PROVIDER_REGISTRY_PROGRAM_ID:?missing EDGERUN_PROVIDER_REGISTRY_PROGRAM_ID; run scripts/start-solana-validator.sh first}"

export SOLANA_RPC_URL SOLANA_KEYPAIR EDGERUN_DEPLOYMENT_PROGRAM_ID EDGERUN_PROVIDER_REGISTRY_PROGRAM_ID

if [ -n "${MARKETPLACE_CLI_BIN:-}" ] && [ ! -x "$CLI_BIN" ]; then
  echo "MARKETPLACE_CLI_BIN is not executable: $CLI_BIN" >&2
  exit 1
fi

if [ -z "${MARKETPLACE_CLI_BIN:-}" ]; then
  cargo build -p edgerun-marketplace-cli --features std
  CLI_BIN="$(find "$ROOT_DIR/target" -path '*/debug/edgerun-marketplace' -type f -perm -111 | head -n 1)"
fi

if [ ! -x "$CLI_BIN" ]; then
  echo "failed to locate built edgerun-marketplace binary" >&2
  exit 1
fi

CLI=("$CLI_BIN")
RUN_ID="$(date +%s)"
PROVIDER_SEED="stress-provider-$RUN_ID"

if [ "$PROVIDER_KEYPAIR" = "$SOLANA_KEYPAIR" ]; then
  echo "MARKETPLACE_PROVIDER_KEYPAIR must differ from SOLANA_KEYPAIR to exercise scheduler assignment" >&2
  exit 1
fi

if [ ! -f "$PROVIDER_KEYPAIR" ]; then
  if ! command -v solana-keygen >/dev/null 2>&1; then
    echo "missing provider keypair and solana-keygen is not on PATH: $PROVIDER_KEYPAIR" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$PROVIDER_KEYPAIR")"
  solana-keygen new --no-bip39-passphrase --silent --outfile "$PROVIDER_KEYPAIR" >/dev/null
fi

if command -v solana >/dev/null 2>&1; then
  provider_authority="$(solana-keygen pubkey "$PROVIDER_KEYPAIR")"
  solana --url "$SOLANA_RPC_URL" airdrop 2 "$provider_authority" >/dev/null || true
fi

extract_field() {
  awk -v label="$1" '$0 ~ label {print $NF; exit}'
}

require_contains() {
  local haystack="$1"
  local needle="$2"
  local context="$3"
  if ! grep -Fq "$needle" <<<"$haystack"; then
    echo "missing expected output for $context: $needle" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

echo "Registering provider seed $PROVIDER_SEED"
echo "Stress config: iterations=$ITERATIONS deposit=$DEPOSIT cpu=$CPU memory=$MEMORY storage=$STORAGE network_mbps=$NETWORK report_network_bytes=$REPORT_NETWORK_BYTES report_delay_seconds=$REPORT_DELAY_SECONDS"
PROVIDER_OUT="$(SOLANA_KEYPAIR="$PROVIDER_KEYPAIR" "${CLI[@]}" provider register "$PROVIDER_SEED" \
  --cpu-cores "$CPU" \
  --memory "$MEMORY" \
  --storage "$STORAGE" \
  --network "$NETWORK")"
echo "$PROVIDER_OUT"
PROVIDER="$(printf '%s\n' "$PROVIDER_OUT" | extract_field "Provider account:")"
if [ -z "$PROVIDER" ]; then
  echo "failed to parse provider account" >&2
  exit 1
fi

started=0
reported=0
stopped=0
failed=0
price_guard_checked=0
price_guard_rejected=0
price_schedule_accepted=0

for i in $(seq 1 "$ITERATIONS"); do
  seed="stress-deploy-$RUN_ID-$i"
  echo
  echo "[$i/$ITERATIONS] create $seed"
  if ! CREATE_OUT="$("${CLI[@]}" deployment create "$seed" \
    --name "$seed" \
    --cpu "$CPU" \
    --memory "$MEMORY" \
    --storage "$STORAGE" \
    --deposit "$DEPOSIT")"; then
    echo "$CREATE_OUT"
    failed=$((failed + 1))
    continue
  fi
  echo "$CREATE_OUT"
  DEPLOYMENT="$(printf '%s\n' "$CREATE_OUT" | extract_field "Deployment account:")"
  if [ -z "$DEPLOYMENT" ]; then
    echo "failed to parse deployment account" >&2
    failed=$((failed + 1))
    continue
  fi

  "${CLI[@]}" deployment assign "$DEPLOYMENT" --provider "$PROVIDER"

  if [ "$PRICE_GUARD_CHECK" -ne 0 ] && [ "$price_guard_checked" -eq 0 ]; then
    price_guard_checked=1
    too_soon_effective_at="$(($(date +%s) + 60))"
    valid_effective_at="$(($(date +%s) + 90000))"

    echo "Checking price grace guard with too-soon effective_at=$too_soon_effective_at"
    if "${CLI[@]}" deployment schedule-pricing "$DEPLOYMENT" \
      --core-hour 20000 \
      --ram-gib-hour 5000 \
      --storage-gib-hour 1000 \
      --network-mbit-hour 2000 \
      --effective-at "$too_soon_effective_at"; then
      echo "price grace guard unexpectedly accepted a too-soon pricing update" >&2
      failed=$((failed + 1))
      continue
    fi
    price_guard_rejected=1

    echo "Scheduling valid future pricing effective_at=$valid_effective_at"
    "${CLI[@]}" deployment schedule-pricing "$DEPLOYMENT" \
      --core-hour 20000 \
      --ram-gib-hour 5000 \
      --storage-gib-hour 1000 \
      --network-mbit-hour 2000 \
      --effective-at "$valid_effective_at"
    DEPLOYMENT_OUT="$("${CLI[@]}" deployment get "$DEPLOYMENT")"
    require_contains "$DEPLOYMENT_OUT" \
      "Pending pricing: core 20000, RAM GiB 5000, storage GiB 1000, network Mbit 2000 lamports/hour at $valid_effective_at" \
      "scheduled pricing readback"
    require_contains "$DEPLOYMENT_OUT" "Auto-stop on price increase: true" "auto-stop readback"
    price_schedule_accepted=1
  fi

  "${CLI[@]}" deployment start "$DEPLOYMENT"
  started=$((started + 1))

  if [ "$REPORT_DELAY_SECONDS" -gt 0 ]; then
    sleep "$REPORT_DELAY_SECONDS"
  fi

  SOLANA_KEYPAIR="$PROVIDER_KEYPAIR" "${CLI[@]}" deployment report "$DEPLOYMENT" \
    --provider "$PROVIDER" \
    --cpu "$CPU" \
    --memory "$MEMORY" \
    --storage "$STORAGE" \
    --network "$REPORT_NETWORK_BYTES"
  reported=$((reported + 1))

  "${CLI[@]}" deployment stop "$DEPLOYMENT" --provider-payout-recipient "$PROVIDER"
  stopped=$((stopped + 1))

  DEPLOYMENT_OUT="$("${CLI[@]}" deployment get "$DEPLOYMENT")"
  require_contains "$DEPLOYMENT_OUT" "Status: Stopped" "stopped deployment readback"
  require_contains "$DEPLOYMENT_OUT" "Unsettled escrow: 0 lamports" "escrow settlement readback"
done

echo
echo "=== Marketplace Stress Summary ==="
echo "Provider: $PROVIDER"
echo "Requested: $ITERATIONS"
echo "Started: $started"
echo "Reported: $reported"
echo "Stopped: $stopped"
echo "Failed create/parse: $failed"
echo "Price guard checked: $price_guard_checked"
echo "Too-soon price update rejected: $price_guard_rejected"
echo "Future price update accepted: $price_schedule_accepted"

EARNINGS_OUT="$("${CLI[@]}" provider earnings "$PROVIDER")"
echo "$EARNINGS_OUT"
require_contains "$EARNINGS_OUT" "Deployments: $ITERATIONS" "provider earnings deployment count"
require_contains "$EARNINGS_OUT" "Stopped deployments: $ITERATIONS" "provider earnings stopped count"

test "$failed" -eq 0
test "$started" -eq "$ITERATIONS"
test "$reported" -eq "$ITERATIONS"
test "$stopped" -eq "$ITERATIONS"
if [ "$PRICE_GUARD_CHECK" -ne 0 ]; then
  test "$price_guard_rejected" -eq 1
  test "$price_schedule_accepted" -eq 1
fi
