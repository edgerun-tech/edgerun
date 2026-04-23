#!/bin/bash
# Setup local Solana test validator for Edgerun marketplace testing

set -e

export PATH="/tmp/solana-release/bin:$PATH"

# Generate a test keypair if it doesn't exist
if [ ! -f ~/.config/solana/id.json ]; then
    solana-keygen new --no-passphrase -o ~/.config/solana/id.json
fi

# Create a temporary ledger directory
LEDGER_DIR=$(mktemp -d)
echo "Ledger directory: $LEDGER_DIR"

# Start the test validator in the background
echo "Starting Solana test validator..."
solana-test-validator \
    --ledger "$LEDGER_DIR" \
    --rpc-port 8899 \
    --faucet-port 9900 \
    --reset \
    --mint 5UXF5kGC2UoHPGJCYnfq radQ2FH short \
    &
VALIDATOR_PID=$!

echo "Validator PID: $VALIDATOR_PID"
echo "Waiting for validator to be ready..."
sleep 5

# Check if validator is running
solana -u http://localhost:8899 cluster-version

echo "Test validator started!"
echo "RPC URL: http://localhost:8899"
echo ""
echo "Your funded keypair:"
solana -u http://localhost:8899 address
echo ""
echo "To use with edgerun-marketplace-cli:"
echo "  export SOLANA_RPC_URL=http://localhost:8899"
echo "  export SOLANA_KEYPAIR=~/.config/solana/id.json"
echo ""
echo "To stop the validator: kill $VALIDATOR_PID"

# Keep the validator running
wait $VALIDATOR_PID
