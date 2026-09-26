# Threshold

**Crowdfund shared work. Pay in agreed stages.**

Threshold is an experimental crowdfunding protocol on Tari Ootle. A community and a sponsor fund a shared goal, with payment rules agreed before anyone contributes. The full budget must be committed before payments begin. Later payments can require reviewers to approve the work.

If funding falls short or work stalls, contributors can reclaim their eligible unpaid funds after the deadline, without permission from the campaign organizer.

**[Explore the demo and Sandbox →](https://seal-clubber.github.io/Threshold/)**

The website explains the idea and lets you try different outcomes. It is a simulation: no wallet is needed and no funds move. The contract has also been exercised on testnet; the [testnet demo guide](docs/demo.md) contains the recorded transactions.

> This is unaudited, experimental testnet software. It is not ready to hold real funds.

## How it works

Imagine a community wants a developer to build a new software feature for **100 tTARI**:

1. **Agree on the terms.** The community supplies 60, a sponsor supplies 40, and two named reviewers will check the finished work. Payment amounts and deadlines are fixed before funding.
2. **Collect the pledges.** Each contributor locks their own funds. The sponsor must supply its agreed share too.
3. **Activate the fully funded campaign.** The contract checks the exact community and sponsor amounts for every stage together. If any part is missing, activation fails and no start payment can be made.
4. **Pay 10 to start.** Once activation succeeds, a separate payment gives the developer 10. The other 90 stays locked.
5. **Review and pay the remaining 90.** Both reviewers must approve the same delivery before the deadline. The developer checks and signs the final payment transaction.

If the developer does not deliver, or the required approvals do not arrive, each contributor can reclaim their own unpaid share after the deadline. **Money already paid cannot be taken back.**

The website lets you go back and flip a decision to see a different outcome. Real transactions cannot be rewound.

The 10/90 example passed a local Ootle engine test. The main recorded testnet campaign uses a different, fixed schedule: 20% upfront, 40% for delivery, and 40% for an audit. Its first two payments completed; contributors recovered the unpaid audit funds.

## What can you configure?

| Setting | What you choose before funding |
|---|---|
| Funding goal | The total budget and the community and sponsor shares for each stage. |
| Payment schedule | The amount and recipient of each milestone, including any start payment after full funding. |
| Reviewers | Who checks the work and how many approvals a reviewed milestone needs. Contributors do not all have to vote. |
| Deadlines | When funding closes and when each milestone must be approved and paid. |

The **Sandbox** on the website lets you explore these choices without changing the bounty demo. It previews campaign terms; it does not create a campaign.

<details>
<summary>Current limits and future flexibility</summary>

The current contract allows up to **32 pledge records, 8 stages, and 5 reviewer keys per stage**. It requires a positive community and sponsor contribution at every stage, with one designated sponsor pledge.

These are implementation choices. Higher caps or funding without a sponsor need a new contract version, client changes, and fresh testing against Ootle's transaction limits. They are not switches that can upgrade an existing campaign. Existing campaigns keep their agreed terms.

The [scaling note](docs/scaling.md) explains what has been tested and what larger campaigns would require. Splitting one goal into unrelated campaigns would lose the guarantee that all parts activate together.

</details>

## What could you use it for?

| Use | Example |
|---|---|
| **Developer bounties** | Users and a sponsor pay for a feature they all need. |
| **Exchange listing and integration costs** | Raise money for agreed exchange fees, technical work, and testing. Funding does not guarantee a listing. |
| **Security audits** | Fund development and a separate auditor in the same campaign, with different payment recipients. |
| **Shared infrastructure** | Share the cost of wallet updates, explorer maintenance, or documentation. |
| **Matched grants** | A foundation adds an agreed contribution to community funding for research, education, or open-source work. |
| **Community-only projects** | Members cover the whole budget themselves. **A separate PoC template exists; the published v1 still requires a sponsor.** |

These are possible uses of the funding rules, not six separately tested products. See [use cases](docs/use-cases.md) for payment examples and limitations, including payments to exchanges outside Ootle.

### Proof-of-concept Ootle templates

Separate Rust template crates now cover **exchange funding, development and audits, infrastructure, matched grants, and community-only funding**. They include example campaign terms and share an experimental contract implementation.

**These are PoCs, not deployed or fully tested products.** Validation is limited to compilation and basic term checks. The existing v1 client and recovery packages do not support them yet. The published v1 contract remains unchanged.

See [the template guide](docs/campaign-templates.md) for the crates, constructors, example payments, and unfinished work.

## What is enforced, and what still needs trust?

The contract checks the funding amounts, payment order, required approvals, and deadlines. An organizer cannot bypass those rules or approve someone else's refund.

People still judge whether the work is good. Reviewers can make mistakes or approve poor work. Threshold enforces their approvals; it cannot prove that a feature works, an audit found every bug, or an exchange will list a token.

<details>
<summary>Privacy, payment checks, and recovery</summary>

- **Private contributions do not mean anonymous contributors.** Ootle's confidential outputs hide individual amounts from public contract calls and state. Participation, public budgets, keys, and timing remain visible. Small groups can reveal amounts by deduction, and a coordinator can learn amounts shared with it.
- **Recipients must check their payments.** The recipient must sign the release transaction, but the contract cannot prove that its encrypted output can be decrypted by that recipient. The receiving client must check it before signing. This is a client responsibility, not an on-chain guarantee.
- **Refunds return each contributor's unpaid funds.** Contributions are kept separately for each payment stage. There is no shared pot from which an organizer calculates or distributes refunds.
- **Recovery needs a backup and network access.** Contributors need their recovery package, unlock material, fee funds, and a working route to the network. They do not need the Threshold website or organizer. The current client uses an indexer for chain data; it does not independently verify consensus proofs.
- **A larger donation does not buy more votes.** Reviewer identities and approval requirements are agreed before funding. The protocol does not prove that different wallet keys belong to different people.

Read the [security notes](docs/security.md) and [protocol specification](docs/protocol.md) before adapting the code.

</details>

## What has been demonstrated?

The repository contains local tests and recorded transactions from Ootle's **Esmeralda testnet**, using the pinned **v0.41.2** tooling.

- Confidential pledges, full campaign activation, reviewer approvals, and milestone payments.
- Rejection of incomplete funding, unapproved payments, and attempts to spend locked funds outside the contract.
- Refunds after missed funding and delivery deadlines, using a separate client while the coordinator was offline.
- Decryption and subsequent spending of recovered funds.
- Activation and payment with 32 pledge records. The test reused signing keys, so it does **not** demonstrate 32 independent people or capacity beyond that limit.
- A local engine test showing that a 10% start payment fails before full activation and succeeds afterward.

The [verification ledger](docs/verification.md) distinguishes local tests, trial transactions, and committed testnet transactions. Public receipts are in [`evidence/`](evidence/). A rejected transaction may still charge a fee; a fee payment is not evidence that the campaign action succeeded.

## Run the website locally

With Node.js installed, run this from the repository root:

```sh
node app/server.mjs
```

Open **http://127.0.0.1:4765/**. The site needs no wallet, indexer, or backend connection to demonstrate the flow.

To build and check the static GitHub Pages files:

```sh
node scripts/build-pages.mjs
node scripts/check-pages.mjs
```

The result is in `_site/`. The [GitHub Pages guide](docs/github-pages.md) explains deployment. Transaction evidence stays in the repository rather than being loaded by the website.

## Work on the contract and client

The contract lives in [`crates/threshold-template`](crates/threshold-template/). The transaction and recovery client lives in [`crates/threshold-client`](crates/threshold-client/).

<details>
<summary>Build requirements and test commands</summary>

Use Rust 1.97 or later and the `wasm32-unknown-unknown` target. The local engine tests need Linux because the pinned Wasmer build does not support Windows. Ubuntu WSL works with `build-essential`, `pkg-config`, `libssl-dev`, `cmake`, and `clang` installed.

Run these commands in Ubuntu WSL from the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo test -p threshold-client --test crypto --locked
cargo test -p threshold-client --features engine-tests --test engine --locked -- --nocapture
cargo build --target wasm32-unknown-unknown --release -p threshold-template --locked
```

Do not share one `target/` directory between Windows and Linux builds. If you use both, assign separate `CARGO_TARGET_DIR` values.

The published contract binary matches a fresh **Windows Rust 1.97.1** release build byte for byte. The tested Ubuntu WSL Rust 1.98.1 build produces a different binary. Use the matching toolchain and a separate build directory when checking the published hash. See the [binary comparison](evidence/template-binary-match.json).

</details>

<details>
<summary>Run the testnet demonstrations</summary>

These commands submit real **testnet** transactions using faucet tokens. They are fixed integration examples, not a production wallet or a general campaign creation service.

After installing the build requirements above, [`scripts/demo.sh`](scripts/demo.sh) builds and publishes the contract, obtains faucet tokens, and runs or resumes the main, missed-funding, and partial-delivery examples.

You can also run individual commands from the repository root:

```sh
cargo run -p threshold-client --locked --bin threshold -- help
cargo run -p threshold-client --locked --bin threshold -- demo
cargo run -p threshold-client --locked --bin threshold -- missed
cargo run -p threshold-client --locked --bin threshold -- partial
cargo run -p threshold-client --locked --bin threshold -- scale8
cargo run -p threshold-client --locked --bin threshold -- scale16
cargo run -p threshold-client --locked --bin threshold -- scale32
```

`demo` covers funding and reviewed payments. `missed` covers an unfunded campaign. `partial` covers a campaign with a start payment and unpaid funds to recover. The `scale` commands measure payments with different numbers of pledge records; they reuse test identities.

Commands resume from saved transaction records. **Do not delete a pending transaction journal to force a retry.** Let the client check whether the previous submission completed.

Template and campaign addresses, transaction IDs, and recovery results are listed in the [testnet demo guide](docs/demo.md).

</details>

<details>
<summary>Recover funds with a separate client</summary>

Before a pledge is signed, the demo exports an encrypted `.local/portable-NAME.recovery` package. Keep a backup of that package and the separate `.local/unlock.secret` file in secure storage. **A seed alone is not enough for this implementation. Never publish either recovery material or the unlock file.** The `.local/` directory is ignored by Git.

Copy the client binary, your recovery package, and the unlock file into a clean directory. You do not need the original project folder, organizer, or Threshold server:

```sh
threshold inspect carol.recovery unlock.secret
threshold recover carol.recovery unlock.secret
threshold verify-refund carol.recovery unlock.secret
threshold respend-refund carol.recovery unlock.secret
```

`inspect` checks the current campaign and your refund eligibility. `recover` submits an eligible refund. `verify-refund` checks the resulting funds. `respend-refund` demonstrates that the recovered funds can be spent again.

The client checks fresh network data, verifies the expected contract and terms, and pays transaction fees from the owner's account. It keeps a transaction journal to avoid submitting duplicates after a restart. An optional indexer URL can be supplied as the final argument. An unavailable or misleading indexer remains a risk; this client is not an independent consensus verifier.

The [demo guide](docs/demo.md) records successful separate-client refunds and subsequent spending of those refunds.

</details>

## Further reading

- [Protocol rules](docs/protocol.md) and [security boundaries](docs/security.md)
- [Testnet walkthrough](docs/demo.md) and [verification ledger](docs/verification.md)
- [Use cases](docs/use-cases.md) and [scaling beyond the current limits](docs/scaling.md)
- [Comparison with related projects](docs/comparisons.md)
- [Dependencies and licenses](docs/dependencies.md)
- [Forum and social post drafts](docs/publication-drafts.md)
