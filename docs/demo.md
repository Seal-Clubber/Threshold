# Threshold demonstration

This is a reproducible Esmeralda **testnet** demonstration against Ootle v0.41.2. It uses four synthetic identities and a 100 tTARI budget, scaled 1:100 from the brief's 10,000 TARI example. Public milestone totals are 20, 40 and 40 tTARI. Every individual deposit is a native confidential UTXO. The protocol and limitations are in [protocol.md](protocol.md) and [security.md](security.md); the [verification ledger](verification.md) distinguishes committed transactions from dry runs.

The existing testnet main campaign freezes 20% upfront. A separate local engine fixture uses the requested 10% upfront and 90% later-stage split: a release before full 100 tTARI activation fails, the 10 tTARI payout succeeds after activation, and owners later refund the unpaid 90 tTARI when its deadline passes. This fixture does not change the immutable testnet campaign.

## Live artifacts

| Event | Esmeralda transaction | Local evidence |
|---|---|---|
| Publish immutable template | [`383669cb…`](https://ootle-indexer-a.tari.com/transactions/383669cbb5d8dcb0b9387e366d5e3e78ba1f67928f24f838131650f7b1749320) | `evidence/publish-receipt.json` |
| Create campaign | [`644950e6…`](https://ootle-indexer-a.tari.com/transactions/644950e65587917d24cc5cc5240d3ee0ed3782834b42473eef0c33f071ac5260) | `evidence/campaign-receipt.json` |
| Activate exact community+sponsor bundle | [`e02a099c…`](https://ootle-indexer-a.tari.com/transactions/e02a099ca3bd01d25ee3872ed350d0e2cbcf26cf2f6ea1a9d15e16f317aaf31d) | `evidence/activation-receipt.json` |
| Pay library start | [`389a8c74…`](https://ootle-indexer-a.tari.com/transactions/389a8c742d7847a76a8da646bc6a92f531aa2c46b6c906dfc40f418036e87a9f) | `evidence/release-0-receipt.json` |
| Pay library delivery after two reviewer votes | [`73d12a4f…`](https://ootle-indexer-a.tari.com/transactions/73d12a4f90ede9f6799502498205d0fc2310ecbbdde157138eb7da2941fa305f) | `evidence/release-1-receipt.json` |
| Recipient re-spends decrypted payout | [`0fb350cf…`](https://ootle-indexer-a.tari.com/transactions/0fb350cf30bf9f93a5ae871cf2b2efea414440f602ca88c85888e999d0393210) | `evidence/recipient-respend-receipt.json` |
| Duplicate activation pays fee but rejects main intent | [`5c093655…`](https://ootle-indexer-a.tari.com/transactions/5c093655d631f71cdd0ce3d5ca33397f881e231bdb3da5b93ee86b43fb253774) | `evidence/fee-only-check-receipt.json` |
| Direct audit escrow spend outside component rejects | [`e35ac868…`](https://ootle-indexer-a.tari.com/transactions/e35ac8688266806f699308e14f45f63fe3c004778e9f39421cd4ed38b52af7ff) | `evidence/direct-bypass-check-receipt.json` |
| Missed campaign owner refunds 3 tTARI | [`bfc6ca02…`](https://ootle-indexer-a.tari.com/transactions/bfc6ca02c0df4994c229d06416a39854b57b95f914b8a0b47d194f7172d33dae) | `evidence/missed-refund-receipt.json` |
| Owner re-spends decrypted refund privately | [`3cba55a2…`](https://ootle-indexer-a.tari.com/transactions/3cba55a27714acd325bcb245cb0a396f28d9fe2db8762ab16c4d5a96442417e2) | `evidence/missed-refund-respend-receipt.json` |
| Create short-deadline partial-delivery campaign | [`f8188c69…`](https://ootle-indexer-a.tari.com/transactions/f8188c693c14cca322b1c419346501f89f90d703139d6727dfcb1fc9d7cb1d08) | `evidence/partial-campaign-receipt.json` |
| Lock private community and sponsor cells | [`523046bf…`](https://ootle-indexer-a.tari.com/transactions/523046bf341e6d94fbfa5c57d7573f191f5141dcbc05dcfecba4bc0497dab454), [`40ab7c33…`](https://ootle-indexer-a.tari.com/transactions/40ab7c3348321005318b4193c8c03ff066b29421daaa81d2f5b50a4b5b0ff06b) | `evidence/partial-pledge-*-receipt.json` |
| Activate short-deadline bundle | [`d6530cd6…`](https://ootle-indexer-a.tari.com/transactions/d6530cd69f7c0c9b317dc53245a3907e6d8d47823d465a85740a56f1a3f8a256) | `evidence/partial-activation-receipt.json` |
| Pay its upfront tranche | [`c0a03cc1…`](https://ootle-indexer-a.tari.com/transactions/c0a03cc13dcfb9c821ae278c0a109b426c9fe8ad15349c527c1ffaa67b35b17c) | `evidence/partial-release-0-receipt.json` |
| Refund partial community's unpaid 3 tTARI | [`9f5f4b73…`](https://ootle-indexer-a.tari.com/transactions/9f5f4b730ef13f7dfdeadd5d5f54e88a2e1fea4e15d5771c05351a9c40feb19d) | `evidence/partial-community-refund-receipt.json` |
| Refund partial sponsor's unpaid 2 tTARI | [`add73a07…`](https://ootle-indexer-a.tari.com/transactions/add73a073c7eba7271060b2079156f6ba545d4b02acfa555263747c9d6e88c87) | `evidence/partial-sponsor-refund-receipt.json` |
| Refund main audit cells: Alice 6, Bob 8, Carol 10, sponsor 16 tTARI | [`8907ede2…`](https://ootle-indexer-a.tari.com/transactions/8907ede224fda76eeb097775c414c12154c48f64adbea9ac55f632abbdf008ff), [`b2941667…`](https://ootle-indexer-a.tari.com/transactions/b294166711b8795b0c0f17482b074dfb2b0cd6b05956f10c70deb62b48665696), [`af25fb35…`](https://ootle-indexer-a.tari.com/transactions/af25fb351878239fa4e2a7222eca6b66538c8062d394b50d5a63ab3b39128e53), [`4d9a22d3…`](https://ootle-indexer-a.tari.com/transactions/4d9a22d3bdc2d75737d4a8a69ee949d1057ebf08efc76bb2d10ae8de5ab0241e) | `evidence/audit-*-refund-receipt.json` |
| Carol privately re-spends recovered audit output | [`ad6ffc6b…`](https://ootle-indexer-a.tari.com/transactions/ad6ffc6ba7b2c40b543d9e0f078f515d6d6c04e35ac6bc764f85df5cb63f9423) | `evidence/audit-carol-refund-respend-receipt.json` |
| Eight private pledge records activate together | [`476434b1…`](https://ootle-indexer-a.tari.com/transactions/476434b1d4c8373c353c0ff9fd81a72fef1874659df306ba50f0d83202eb8ee7) | `evidence/scale8-activation-receipt.json` |
| Eight-input private payout commits | [`0d618efc…`](https://ootle-indexer-a.tari.com/transactions/0d618efcdf6f5edd76ac2348bf3cbda478cdf83e8a794685a2afb75e0fcdffd4) | `evidence/scale8-release-receipt.json` |
| Sixteen private pledge records activate together | [`7b4cdcbe…`](https://ootle-indexer-a.tari.com/transactions/7b4cdcbec12ec78a363e85d67ee94ef318799a98dbd89b1d1d457090d4e2e38d) | `evidence/scale16-activation-receipt.json` |
| Sixteen-input private payout commits | [`a8941407…`](https://ootle-indexer-a.tari.com/transactions/a89414075739d0590b3c42d56fe97e0bc202dc4ec52132bf56da1cc126e0d7a8) | `evidence/scale16-release-receipt.json` |
| Thirty-two private pledge records activate together | [`6710113b…`](https://ootle-indexer-a.tari.com/transactions/6710113bd039bd3c54743f9e8581498e286f5a43039e3b1031b7ff6d4c36e066) | `evidence/scale32-activation-receipt.json` |
| Thirty-two-input private payout commits | [`699be5e2…`](https://ootle-indexer-a.tari.com/transactions/699be5e2cd7a3be7d0eaa7e8f2d31334215a8062c851a781c225ae284b856666) | `evidence/scale32-release-receipt.json` |

The published template is `template_7324098fce367741408484cea7779a253303b0a5fed0a4c93ce1876d86778400`; the [live component](https://ootle-indexer-a.tari.com/substates/component_82cad9632a4f5d2d6f07768d478a26a1e7032774faae6ab1f987a65fb7c97b58) is `component_82cad9632a4f5d2d6f07768d478a26a1e7032774faae6ab1f987a65fb7c97b58`. The client obtained `Commit` receipts for all linked transactions. The indexer has returned `summary: null` for some recent transaction pages, so those saved receipts remain **local client evidence** until that view catches up; the [verification log](verification.md) labels them accordingly. The indexer reports its component state as verified, but this project has not implemented a consensus light client.

## Run locally

Install Rust with `wasm32-unknown-unknown`, the Ubuntu packages named in [README](../README.md), and Node.js. Run the pinned checks in Ubuntu WSL:

```sh
cargo test -p threshold-client --test crypto --locked
cargo test -p threshold-client --features engine-tests --test engine --locked
cargo build -p threshold-template --target wasm32-unknown-unknown --release --locked
```

From the repository root, `cargo run -p threshold-client --locked --bin threshold -- demo` resumes the main campaign. `missed` resumes a second campaign whose 3 tTARI community pledge lacks a 2 tTARI sponsor leg; `evidence/missed-incomplete-bundle-dry-run.json` records rejection. Transaction journals in `evidence/` are reconciled before any fresh submission. Do not remove a pending journal to force a retry. For the site, run `node app/server.mjs` and open the printed localhost address. The single page is a self-contained bounty wire diagram: visitors flip off-by-default switches for four pledgers, activation, upfront payment, developer delivery, two reviewers, recipient client preflight, and final release. Side branches show missing funding, stopped work, failed quorum, or failed client decryption; owner-refund buttons are illustrative. Its separate sandbox previews hypothetical campaign terms without changing the map. The site only serves static assets; it does not query the indexer, sign, or submit. Its 10/90 bounty is illustrative and locally engine-tested; the live campaign's upfront share is 20%. Full receipts remain in `evidence/` and this guide.

The incomplete campaign's funding deadline was epoch `11435`. At epoch `11440`, its owner used a separate Ubuntu directory containing only the binary, encrypted recovery package and unlock file to commit a 3 tTARI refund while the coordinator and observer were offline. `verify-refund` checked the package, journal and receipt, indexer-verified component and UTXO state, and decrypted the output. The owner then committed a private re-spend; rerunning `respend-refund` found the saved `Commit` and submitted no duplicate. [Recovery measurements](../evidence/missed-recovery-metrics.json) record the transaction sizes, fees and proof time. The indexer was also queried for the new private output, which it marked verified and unspent. These checks rely on the indexer's verified view, not an independent consensus light client.

The main campaign's unreleased audit tranche became refundable at its frozen epoch `11454`. At epoch 11464, with the observer offline, the standalone Ubuntu client found and refunded exactly 40 tTARI of unpaid audit cells: Alice 6, Bob 8, Carol 10, sponsor 16. All four full `Commit` receipts and recipient output decryptions were verified; Carol then re-spent her 10 tTARI output privately and the new live output was verified and decrypted. [Audit recovery measurements](../evidence/audit-recovery-metrics.json) record fees, proof times and encoded transaction sizes.

To exercise partial delivery, a separate [ownerless campaign](https://ootle-indexer-a.tari.com/substates/component_0398bdd0737ffafed51b7a4befd56b5cb5600020c3ccac370ae625c706ce8109) used two contributors and two stages. It privately locked 6 tTARI community and 4 tTARI sponsor in separate cells, activated both stages atomically, and paid the agreed 5 tTARI upfront tranche. The indexer marked the component verified and showed stage 1 unpaid, with 3 tTARI community and 2 tTARI sponsor still in separate cells. At epoch 11441, the clean Ubuntu client refused an early refund without creating a journal. At epoch 11464, after the frozen deadline 11445, live dry runs rejected late approval and release; then the same isolated client committed both owner refunds. Both private outputs decrypted and their UTXOs and component state were indexer-verified. See [partial recovery measurements](../evidence/partial-recovery-metrics.json) and `evidence/partial-late-*-dry-run.json`.

## Negative evidence and costs

`evidence/` includes rejected dry runs for underfunding, using sponsor funds as community funds, omitting a bundle leg, duplicate activation, missing reviewer approval, direct script bypass, extra payout output and public outflow. Rejection is evaluated on the **main intent**; fee-only effects do not count as funding. The committed `fee-only-check` transaction made this distinction concrete: `OnlyFeeCommit` from the client, `FeeIntentCommit` in its receipt, 2,094 microtari paid, and only the fee vault updated. The active component was not changed. A repeat invocation recognized the saved outcome and sent nothing. Engine tests additionally reject cross-campaign reuse of a locked cell, a redirected payout attempted by a non-recipient, replayed payout, and double refund. A recipient who signs a redirected output themselves remains an explicitly documented v1 limit. Native crypto tests cover output encryption/decryption, re-spend construction, exact opening checks and package tampering.

A direct stealth transfer tried to spend all four still-locked audit cells without calling the Threshold component. Its dry run rejected with `No active call frame`; the submitted transaction returned `OnlyFeeCommit`/`FeeIntentCommit`, charged 9,274 microtari, and updated only the fee vault. The clean client subsequently inspected Carol's 10 tTARI audit cell and found it still unspent. This is network evidence of the component-scoped Script gate, not just an interface restriction.

Committed receipt fees for this fixture, in microtari, were: template publication 5,228,873; campaign creation 2,906; each of four shield transfers 15,957; pledge registration 24,477–24,937; joint activation 9,646; reviewer votes 3,379 and 3,412; milestone releases 12,492 and 12,559; and recipient re-spend 9,336. These are observed `total_fees_paid` values in `evidence/*-receipt.json`, not a fee quote. The separate missed campaign cost 2,532 to create, 15,957 to shield, 11,331 to pledge, 11,378 to refund, and 9,336 to re-spend the refund. The one-cell refund's full encoded transaction was 1,569 bytes and its native proof took 2,283 ms on this host; its private re-spend transaction was 1,482 bytes. A 1-microtari escrow cell may therefore cost more to reclaim than its value unless the owner maintains separate fee funds.

The short-deadline partial-delivery campaign observed 2,668 microtari to create, 15,957 per shield, 17,868 and 17,984 for pledge registration, 6,755 for activation, 11,744 for its upfront payment, and 11,629 for each one-cell owner refund. Each main audit owner refund cost 12,240 microtari; Carol's subsequent private re-spend cost 9,336. Each one-cell refund encoded to 1,569 bytes and took about two seconds for native proof construction on this host.

The indexer exposes full signed transactions. The [`measure_indexed` tool](../crates/threshold-client/src/bin/measure_indexed.rs) decoded selected public transactions with pinned Ootle types, verified each calculated transaction ID, and counted exact `tari_bor` bytes. The [30 recorded measurements](../evidence/transaction-sizes.json) include:

To repeat one measurement, save the public JSON from `https://ootle-indexer-a.tari.com/transactions/e02a099ca3bd01d25ee3872ed350d0e2cbcf26cf2f6ea1a9d15e16f317aaf31d` to a local file, then run `cargo run -p threshold-client --bin measure_indexed --locked -- PATH_TO_JSON` in Ubuntu WSL. The tool rejects a body whose calculated transaction ID differs from the indexed ID.

| Esmeralda transaction | Signed CBOR bytes | Observed fee (microtari) |
|---|---:|---:|
| Three-cell private pledge | 2,059 | 24,477–24,937 in main fixture |
| Four-pledge exact activation | 680 | 9,646 |
| Four-input private milestone payout | 2,122 | 12,492–12,559 |
| Two-pledge exact activation | 610 | 6,755 |
| Two-input private upfront payout | 1,780 | 11,744 |
| One-cell missed-funding owner refund | 1,569 | 11,378 |
| Private re-spend of that refund | 1,482 | 9,336 |
| Eight-pledge exact activation | 548 | 5,432 |
| Eight-input private payout | 2,806 | 12,667 |
| Sixteen-pledge exact activation | 556 | 6,541 |
| Sixteen-input private payout | 4,174 | 14,132 |
| Thirty-two-pledge exact activation | 582 | 8,776 |
| Thirty-two-input private payout | 6,912 | 17,074 |

These are actual Esmeralda transaction sizes, not estimates from native proof statements. They do not establish a simple per-pledge fee curve: storage, WASM work, fees and transaction shape differ by operation. The eight-, sixteen-, and thirty-two-pledge capacity fixtures used respectively one, one, and three community test signing keys, plus a separate sponsor key; they measure transaction capacity, not independent people. The recipient decrypted all three payouts, and the indexer marked their components and unspent outputs verified. See [eight-pledge](../evidence/scale8-metrics.json), [sixteen-pledge](../evidence/scale16-metrics.json), and [thirty-two-pledge](../evidence/scale32-metrics.json) measurements. The published template caps a campaign at 32 pledge records; larger campaigns have not been implemented or tested.

The reproducible native microbenchmark `cargo run -p threshold-client --bin bench_native --locked` in Ubuntu WSL generated and validated five one-output stealth transfer statements at each input count. The median times below are from the **debug** build on this host; output encryption, input preparation, transaction framing, fee intent, consensus and network latency are outside the timed proof section.

| Private inputs | Median proof construction | Encoded transfer statement | Median native validation |
|---:|---:|---:|---:|
| 1 | 1,715 ms | 1,022 bytes | 140 ms |
| 4 | 1,709 ms | 1,304 bytes | 140 ms |
| 16 | 1,720 ms | 2,432 bytes | 140 ms |
| 32 | 1,722 ms | 3,937 bytes | 141 ms |

These figures are statement sizes, **not full serialized transaction sizes**. The full signed 32-input Esmeralda payout was measured separately above. The code caps registrations at 32 and stages at 8; the 32-pledge cap has now been exercised on Esmeralda, but higher pledge counts, repeated large campaigns, and production throughput remain unmeasured. Do not extrapolate from these synthetic fixtures to production cost or privacy. In particular, the coordinator learns individual openings if users provide them for subset selection, and reviewers decide whether off-chain work is acceptable.

The pinned **local Ootle engine** also executed complete eight- and 32-pledge campaigns, with one private cell per pledge, exact role-separated activation, and an eight- or 32-input private release. The eight-pledge debug run took 41.61 s to prepare/fund and 43.57 s overall; the 32-pledge run took 145.66 s to prepare/fund and 148.57 s overall. The component activation calls took 21 ms and 79 ms respectively after deposits were prepared. These are wall-clock measurements on this host and include local test-tooling overhead, not Esmeralda consensus or fees. The full 32-pledge cap is now also exercised on Esmeralda. The local test remains reproducible with `THRESHOLD_SCALE_PLEDGES=32 cargo test -p threshold-client --features engine-tests --test engine measured_multi_pledge_activation_and_release_in_engine --locked -- --nocapture`.

The pinned Ootle [stealth limits](https://github.com/tari-project/tari-ootle/blob/v0.41.2/crates/engine_types/src/limits.rs) permit at most 1,000 inputs and 16 outputs in one stealth statement, 64 transfers, 1,024 stealth inputs and 256 stealth outputs across a transaction. These are consensus ceilings, **not** evidence that Threshold can fund 1,000 pledges economically or within its component-state and fee limits.
