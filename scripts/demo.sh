#!/usr/bin/env bash
set -euo pipefail

# Run from the repository root in Ubuntu WSL. Uses Esmeralda faucet test tokens.
# Existing transaction journals are retained and reconciled by the client.
cd "$(dirname "$0")/.."
cargo build -p threshold-template --target wasm32-unknown-unknown --release --locked
cargo run -p threshold-client --bin threshold --locked -- bootstrap
cargo run -p threshold-client --bin threshold --locked -- publish
cargo run -p threshold-client --bin threshold --locked -- demo
cargo run -p threshold-client --bin threshold --locked -- missed
cargo run -p threshold-client --bin threshold --locked -- partial

printf 'Public receipts and deployments are in evidence/. Start the interactive bounty demo with: node app/server.mjs\n'
printf 'For independent recovery, copy one encrypted .local/portable-*.recovery package, .local/unlock.secret, and the Ubuntu threshold binary to a clean directory.\n'
