#!/usr/bin/env bash
# Exercise SPL-token marketplace escrow against a running local Solana validator.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -n "${SOLANA_BIN_DIR:-}" ]; then
  PATH="$SOLANA_BIN_DIR:$PATH"
fi

ENV_FILE="${SOLANA_LOCALNET_ENV:-$ROOT_DIR/target/localnet/marketplace.env}"
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

command -v solana >/dev/null
command -v solana-keygen >/dev/null
command -v spl-token >/dev/null

if [ -z "${MARKETPLACE_CLI_BIN:-}" ]; then
  cargo build -p edgerun-marketplace-cli --features std
  CLI_BIN="$(find "$ROOT_DIR/target" -path '*/debug/edgerun-marketplace' -type f -perm -111 | head -n 1)"
fi

if [ ! -x "$CLI_BIN" ]; then
  echo "failed to locate built edgerun-marketplace binary" >&2
  exit 1
fi

if [ ! -f "$PROVIDER_KEYPAIR" ]; then
  mkdir -p "$(dirname "$PROVIDER_KEYPAIR")"
  solana-keygen new --no-bip39-passphrase --silent --outfile "$PROVIDER_KEYPAIR" >/dev/null
fi

OWNER="$(solana-keygen pubkey "$SOLANA_KEYPAIR")"
PROVIDER_AUTHORITY="$(solana-keygen pubkey "$PROVIDER_KEYPAIR")"
solana --url "$SOLANA_RPC_URL" airdrop 2 "$PROVIDER_AUTHORITY" >/dev/null || true

RUN_ID="$(date +%s)"
DEPLOYMENT_SEED="token-deploy-$RUN_ID"
PROVIDER_SEED="token-provider-$RUN_ID"

extract_field() {
  awk -v label="$1" '$0 ~ label {print $NF; exit}'
}

echo "Preparing token escrow deployment seed $DEPLOYMENT_SEED"
DEPLOYMENT_OUT="$("$CLI_BIN" deployment address "$DEPLOYMENT_SEED")"
echo "$DEPLOYMENT_OUT"
DEPLOYMENT="$(printf '%s\n' "$DEPLOYMENT_OUT" | extract_field "Deployment account:")"

ESCROW_OUT="$("$CLI_BIN" deployment escrow-authority "$DEPLOYMENT")"
echo "$ESCROW_OUT"
ESCROW_AUTHORITY="$(printf '%s\n' "$ESCROW_OUT" | extract_field "Escrow authority:")"

MINT_OUT="$(spl-token --url "$SOLANA_RPC_URL" --fee-payer "$SOLANA_KEYPAIR" create-token --decimals 6)"
echo "$MINT_OUT"
MINT="$(awk '/Address:/ {print $2; exit}' <<<"$MINT_OUT")"

OWNER_TOKEN_OUT="$(spl-token --url "$SOLANA_RPC_URL" --fee-payer "$SOLANA_KEYPAIR" create-account "$MINT" --owner "$OWNER")"
echo "$OWNER_TOKEN_OUT"
OWNER_TOKEN="$(printf '%s\n' "$OWNER_TOKEN_OUT" | extract_field "Creating account")"

ESCROW_TOKEN_OUT="$(spl-token --url "$SOLANA_RPC_URL" --fee-payer "$SOLANA_KEYPAIR" create-account "$MINT" --owner "$ESCROW_AUTHORITY")"
echo "$ESCROW_TOKEN_OUT"
ESCROW_TOKEN="$(printf '%s\n' "$ESCROW_TOKEN_OUT" | extract_field "Creating account")"

spl-token --url "$SOLANA_RPC_URL" --fee-payer "$SOLANA_KEYPAIR" mint "$MINT" 100 "$OWNER_TOKEN"

"$CLI_BIN" deployment create "$DEPLOYMENT_SEED" \
  --name "$DEPLOYMENT_SEED" \
  --cpu 1 \
  --memory 1073741824 \
  --storage 1073741824 \
  --deposit 1000000 \
  --payment-mint "$MINT" \
  --owner-token-account "$OWNER_TOKEN" \
  --escrow-token-account "$ESCROW_TOKEN"

PROVIDER_OUT="$(SOLANA_KEYPAIR="$PROVIDER_KEYPAIR" "$CLI_BIN" provider register "$PROVIDER_SEED" \
  --cpu-cores 1 \
  --memory 1073741824 \
  --storage 1073741824 \
  --network 10)"
echo "$PROVIDER_OUT"
PROVIDER="$(printf '%s\n' "$PROVIDER_OUT" | extract_field "Provider account:")"

PROVIDER_TOKEN_OUT="$(spl-token --url "$SOLANA_RPC_URL" --fee-payer "$SOLANA_KEYPAIR" create-account "$MINT" --owner "$PROVIDER_AUTHORITY")"
echo "$PROVIDER_TOKEN_OUT"
PROVIDER_TOKEN="$(printf '%s\n' "$PROVIDER_TOKEN_OUT" | extract_field "Creating account")"

"$CLI_BIN" deployment assign "$DEPLOYMENT" --provider "$PROVIDER"
"$CLI_BIN" deployment start "$DEPLOYMENT"
sleep "${MARKETPLACE_TOKEN_SETTLE_DELAY_SECONDS:-1}"
"$CLI_BIN" deployment stop "$DEPLOYMENT" \
  --provider-payout-recipient "$PROVIDER_TOKEN" \
  --buyer-refund-token-account "$OWNER_TOKEN" \
  --escrow-token-account "$ESCROW_TOKEN" \
  --escrow-authority "$ESCROW_AUTHORITY"

DEPLOYMENT_INFO="$("$CLI_BIN" deployment get "$DEPLOYMENT")"
echo "$DEPLOYMENT_INFO"
grep -Fq "Status: Stopped" <<<"$DEPLOYMENT_INFO"
grep -Fq "Unsettled escrow: 0 token base units" <<<"$DEPLOYMENT_INFO"

ESCROW_INFO="$(spl-token --url "$SOLANA_RPC_URL" account-info --address "$ESCROW_TOKEN")"
PROVIDER_INFO="$(spl-token --url "$SOLANA_RPC_URL" account-info --address "$PROVIDER_TOKEN")"
echo "$ESCROW_INFO"
echo "$PROVIDER_INFO"
grep -Fq "Balance: 0" <<<"$ESCROW_INFO"

echo "Marketplace token escrow local e2e passed."
