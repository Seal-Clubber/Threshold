# Former Esmeralda reviewer quorum proof client

This executable submitted five **synthetic** Tari-style bounty examples to the former Esmeralda chain using demonstration identities. It never used Tari council or treasury keys. The first example had **1-of-1** review and closed as no work, returning all 15,000 tTARI in escrow to the demo treasury. The second started at 1-of-1, changed to **2-of-3 before funding**, then recorded a council award proposal, a second reviewer approval, and a contributor claim. The first refund supplied the second escrow. Three more were open and funded at the saved snapshot: **1-of-3**, **1-of-1**, and **2-of-3**, each with 15,000 tTARI locked then. All five used the fixed S-tier amount.

**The testnet has since reset.** The saved receipts and [`verified-state.json`](../../evidence/bounty/verified-state.json) are historical. Do not run this client against the new chain as-is: its receipt replay logic would reuse former-chain transaction records. A fresh run needs its own receipt namespace, a current network endpoint, and qualification against the new Ootle release.

The client obtains faucet tTARI through separate demo donor accounts, consolidates it in the demo treasury, and publishes the compiled bounty template. Each network step dry-runs first. A full main-intent `Commit` receipt is required before the next step. The bounty transactions and public receipts go to [`evidence/bounty/`](../../evidence/bounty/). Faucet and donor-transfer receipts, dry-run output, encrypted demo keys, and escrow openings stay in the parent lab's `.local/` directory. The pending-transaction journal is checked before any retry so a rerun does not blindly resubmit.

To build the historical template and check its saved receipts from the public checkout, using Ubuntu WSL:

```sh
export CARGO_TARGET_DIR=/home/seal/.cache/threshold-bounty-target
cargo build -p threshold-bounty-template --target wasm32-unknown-unknown --release --locked
python3 scripts/check-bounty-live.py --receipts-only
```

The [offline archive](https://seal-clubber.github.io/Threshold/bounties-archive.html) displays the saved former-chain reviewer rules, state, and action receipts without network requests. The current-chain [bounty board](../../app/bounties.html) is empty until fresh transactions are recorded. Neither list is a Tari-approved program registry. A future live board must require the indexer's `verified` flag and match each component's published template; that is still indexer-sourced data, not an independent consensus proof.

## Adding a bounty to this demo board

This historical client has explicit synthetic issues and receipt labels in `src/main.rs`. A future current-chain client should use a separate receipt namespace and write `evidence/bounty/current-index.json` after committed transactions are independently checked. `node scripts/build-pages.mjs` then copies that known-address list into the active site board. The public GitHub Pages board needs a deployment to pick up a changed list. Once listed, a component's status, balance, keys, and votes are read from Ootle on each refresh.

The indexer also exposes a [transaction-event query](https://github.com/tari-project/tari-ootle/blob/v0.41.2/applications/tari_indexer/src/rest_api/handlers/transactions.rs#L256), but finding a component made from this public template does not prove Tari approved it: anyone can instantiate the template. For a program where bounties appear automatically, Tari would need a canonical council-controlled on-chain registry. The board could read its approved addresses and key-to-name labels, then read each bounty component. That registry is a proposed next step, not part of this demo.
