# Threshold

Threshold is an experimental confidential, matched milestone funding protocol for Tari Ootle. Each contribution is split into private milestone cells. An immutable component locks those cells, checks exact community and sponsor budgets for every milestone in one activation, authorizes ordered payments, and lets each owner recover eligible unspent cells directly.

The [protocol specification](docs/protocol.md) explains the guarantees and their limits. The design does not implement quadratic matching, anonymous identity, proportional refunds from a pooled vault, or automatic proof of satisfactory work. It is unaudited testnet software. The current campaign uses synthetic test token amounts and demonstrates the native privacy mechanism, not donor anonymity.

The [scaling note](docs/scaling.md) records the 32-pledge testnet result and the new-template work needed for 100 or more funders. It also explains why splitting a campaign into unrelated smaller campaigns would weaken atomic funding.

A 10% upfront / 90% reviewer-gated schedule passed the local Ootle engine test: the upfront release fails before full activation and succeeds afterward. The already deployed main testnet campaign has immutable 20% upfront terms.

The [related-protocol comparison](docs/comparisons.md) explains the scope against Allo, Juicebox and clr.fund. [Dependency and originality notes](docs/dependencies.md) record the direct Tari licenses. [Unpublished contest and social drafts](docs/publication-drafts.md) are local preparation only.

## Status

Ootle v0.41.2 is pinned and the public Esmeralda indexer reported v0.41.2 at the initial probe. The Threshold WASM template was published on Esmeralda. The [verification log](docs/verification.md) separates local cryptography tests, Linux engine tests, and committed testnet transactions. Local `evidence/` contains public transaction IDs and receipts. `.local/` contains encrypted recovery material and a separate unlock key; it is ignored by Git.

Run `node app/server.mjs` and open `http://127.0.0.1:4765/` for the interactive developer-bounty wire diagram. All switches start off. Visitors set four independent pledges, atomic activation, the upfront release, developer delivery, two reviewer votes, a recipient client check, and the final release. Funding and review decisions sit side by side; any missing requirement highlights a funding, delivery, review, or client-check branch. Visitors can scroll back and change the hypothetical outcome; after turning on a branch's deadline switch, separate owner-refund buttons demonstrate independent recovery. The integrated sandbox independently previews other budgets, funding splits, milestones, pledge and reviewer counts, quorum, and deadlines; it never changes the bounty map. The page never sends transactions or queries the indexer. The fixed 100 tTARI bounty's 10% upfront payment is illustrative and locally engine-tested; the existing live campaign has immutable 20% upfront terms. The dark interface has a light-mode switch. Committed transaction receipts and limitations are documented in [the demo guide](docs/demo.md) and [verification log](docs/verification.md).

For a public static version, run `node scripts/build-pages.mjs` or use the included GitHub Pages workflow after uploading the repository. [Pages setup](docs/github-pages.md) publishes only the bounty map and sandbox. The evidence remains in the repository, outside the site.

The main demonstration component is `component_82cad9632a4f5d2d6f07768d478a26a1e7032774faae6ab1f987a65fb7c97b58`; its two paid stages and final owner refunds have completed. The published template is `template_7324098fce367741408484cea7779a253303b0a5fed0a4c93ce1876d86778400`. Consult the live component state and transaction receipts before treating any operation as final.

## Build and test

Rust 1.97+ and the `wasm32-unknown-unknown` target are required. Local engine tests require Linux because the pinned Wasmer Cranelift build rejects Windows. Ubuntu WSL works with `build-essential`, `pkg-config`, `libssl-dev`, `cmake`, and `clang`. Run all Cargo commands in Ubuntu WSL, or assign distinct `CARGO_TARGET_DIR` values to Windows and Linux builds; mixing both toolchains in one `target/` can produce incompatible compiler artifacts.
The published template is byte-identical to a fresh Windows Rust 1.97.1 release build. Ubuntu WSL's installed Rust 1.98.1 builds functionally tested code but produces a different WASM binary; see the [artifact comparison](evidence/template-binary-match.json). Use Rust 1.97.1 on Windows with a separate target directory when verifying the exact published hash.

```sh
rustup target add wasm32-unknown-unknown
cargo test -p threshold-client --test crypto --locked
cargo test -p threshold-client --features engine-tests --test engine --locked -- --nocapture
cargo build --target wasm32-unknown-unknown --release -p threshold-template --locked
```

Run the client from the repository root. It targets Esmeralda and spends faucet test tokens. Existing transaction journals are reconciled before a new submission; never discard a pending journal to force a retry.

The complete setup sequence is in [`scripts/demo.sh`](scripts/demo.sh). Run it from Ubuntu WSL after installing the build dependencies below; it builds the WASM template, obtains faucet tokens, publishes the template, and resumes all three demonstration campaigns.

```sh
cargo run -p threshold-client --locked --bin threshold -- help
cargo run -p threshold-client --locked --bin threshold -- demo
cargo run -p threshold-client --locked --bin threshold -- missed
cargo run -p threshold-client --locked --bin threshold -- partial
cargo run -p threshold-client --locked --bin threshold -- scale8
cargo run -p threshold-client --locked --bin threshold -- scale16
cargo run -p threshold-client --locked --bin threshold -- scale32
```

`demo` creates four test identities, privately shields three community pledges and one sponsor pledge, rejects invalid activation attempts, activates the full bundle, obtains reviewer approvals, and pays two milestones. `missed` creates an incomplete campaign so the donor can recover after the frozen funding deadline. `partial` creates a separate two-owner campaign, pays its upfront stage, and leaves a community cell and a sponsor cell for direct owner refunds after epoch 11445. `scale8`, `scale16`, and `scale32` measure private payouts with eight, sixteen, and thirty-two inputs on testnet; community pledge records reuse test signing keys, so they do not model that many independent people. These commands resume from recorded transactions. They are integration demonstrations, not a production wallet or a service for arbitrary campaigns.

## Independent recovery

Before signing a pledge, the demo exports `.local/portable-NAME.recovery`, encrypted with Tari's Argon2id and authenticated encryption. Keep a copy of that package **and** `.local/unlock.secret` on separate secure storage. The package contains the refund key, output openings, script context, terms, pledge ID, and transaction references once known. A seed alone cannot reconstruct these fields. The unlock file grants control of a test account, so do not publish it.

Copy the binary, one package, and the unlock file to a clean directory; no Threshold server, database, or original `.local/` directory is needed:

```sh
threshold inspect carol.recovery unlock.secret
threshold recover carol.recovery unlock.secret
threshold verify-refund carol.recovery unlock.secret
threshold respend-refund carol.recovery unlock.secret
```

The command queries fresh indexer state, checks the template, ownerless rule, frozen terms, pledge, each UTXO and eligibility epoch, creates a one-output private refund, pays fees from the owner's account, and submits directly to Ootle. It writes a local transaction journal before submission and refuses an ambiguous retry. On a pending-journal restart it checks a refreshed verified component for the owner's refund before saving a fetched committed receipt. An optional indexer URL is accepted as the final argument. The indexer's `verified` flag is checked, but this client is not an independent consensus light client; a malicious or unavailable indexer remains a network trust/availability risk.

The incomplete campaign's 3 tTARI pledge was refunded at epoch 11440 from this isolated Ubuntu client while the observer was offline. Its owner decrypted the refund and re-spent it into a new private output. At epoch 11464, the same isolated client refunded both unpaid cells of the partial-delivery campaign and all four unpaid audit cells of the main campaign. Carol also re-spent her audit refund privately. The [demo guide](docs/demo.md) gives committed transaction IDs and measurements.
After a re-spend, `verify-refund` checks the saved committed receipts, the consumed input commitment and the new live private output. `recover` and `respend-refund` also recognize saved full `Commit` receipts, so repeating them does not submit a second transaction while the indexer receipt endpoint lags.

## Reading the evidence

The [demo guide](docs/demo.md) links transaction IDs, receipts, rejected dry runs, and measured limits. A dry run with `AcceptFeeRejectRest` proves that the **main intent failed**; fee execution alone is never recorded as successful funding. When a recent transaction receipt has not yet appeared at the indexer endpoint, the client labels its saved full receipt as local evidence and separately checks the indexer-verified resulting component and UTXO state. This is not independent consensus proof verification.

The client SDK is `crates/threshold-client/src/lib.rs`; the WASM protocol is `crates/threshold-template/src/lib.rs`. No coordinator endpoint has authority over escrow. See [security and privacy](docs/security.md) before adapting this code to real assets.
