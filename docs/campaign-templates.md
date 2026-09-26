# Campaign templates — proof of concept

These are separate Ootle template crates, not website presets. They implement campaign methods and provide example constructors, but are **proof of concept only: not deployed, audited, or tested end to end**. Amounts are illustrative test tokens, not estimates of real-world costs.

The existing bounty tests remain evidence for that particular implementation and fixture. They do not establish that these new examples have been tested on-chain.

| Crate | Ootle template name | Funding mode |
|---|---|---|
| [`threshold-exchange-template`](../crates/threshold-exchange-template/) | `ExchangeFunding` | Community + sponsor |
| [`threshold-audit-template`](../crates/threshold-audit-template/) | `AuditFunding` | Community + sponsor; different first and final recipient keys |
| [`threshold-infrastructure-template`](../crates/threshold-infrastructure-template/) | `InfrastructureFunding` | Community + sponsor |
| [`threshold-grant-template`](../crates/threshold-grant-template/) | `GrantFunding` | Community + sponsor |
| [`threshold-community-template`](../crates/threshold-community-template/) | `CommunityFunding` | Community only; sponsor pledges rejected |

All five share [`threshold-poc-core`](../crates/threshold-poc-core/). The original `threshold-template` source and its v1 client are unchanged. These new templates use a separate **experimental terms version 100**; it is not an upgrade or a stable protocol release.

## What is implemented

Each template exposes `new`, `example`, `new_example`, `terms`, `pledges`, `status`, `lock_root`, `pledge`, `activate`, `approve`, `release`, and `refund`.

- `example(config)` returns example terms without creating a campaign. Supply a nonce, resource, future funding deadline, positive stage spacing, a recipient key for each stage, and reviewer keys. Matched examples also need a sponsor key. Use `None` for the community-only example.
- `new_example(config)` creates a component from those example terms. It rejects invalid terms and deadlines using the current consensus epoch.
- `new(terms)` accepts a custom budget and schedule, subject to the funding mode and profile rules. The examples below are defaults, not restrictions on every campaign.
- `pledge(sponsor, transfer)` keeps separate private outputs for each contributor and stage. Community-only campaigns accept only `sponsor = false`.
- `activate(selected, aggregate_masks)` checks exact amounts for every stage in one activation. Matched campaigns need masks in stage order: community, sponsor, community, sponsor, and so on. Community-only campaigns need one community mask per stage, with no placeholder sponsor column.
- `approve`, `release`, and `refund` retain the intended reviewer, recipient, deadline, and owner checks. A component has no administrator or upgrade method. These paths still need new engine and transaction qualification.

The audit template requires different first and final recipient keys. That is a key-level separation, not proof that different people control them. None of the templates verifies exchange operations, work quality, service uptime, or a person's identity.

**The existing v1 client and portable recovery packages do not support these new terms or template IDs.** Client integration, recovery package versioning, wallet checks, and a clean-client recovery demonstration are unfinished. Having a `refund` method in this source is not evidence of working independent recovery for these PoCs.

## Exchange listing and integration

A community and sponsor fund work agreed with an exchange before contributions begin.

- **Budget:** 1,000 tTARI; community 600, sponsor 400.
- **Payments:** 100 after full funding, 400 after integration testing, 500 after the agreed launch requirements are met.
- **Review:** two of three named reviewers for the later payments.
- **Recipients:** the agreed integration team or exchange, named separately for each stage.

The first payment could cover an agreed fee or initial technical work. Reviewers need evidence from the exchange for the later stages. Direct recipients must support the required Ootle payment flow. If someone pays the exchange elsewhere, that onward payment depends on them. A listing is not guaranteed, and released fees cannot be recovered through Threshold.

**Status:** proof-of-concept campaign terms using the current matched funding model; no dedicated deployment or end-to-end test.

## Development and security audit

Projects share the cost of improving a library and having it audited.

- **Budget:** 500 tTARI; community 350, sponsor 150.
- **Payments:** 50 to begin development, 250 for accepted code, 200 for the audit report and agreed follow-up.
- **Review:** two of three named reviewers for the later payments.
- **Recipients:** the developer for the first two stages; the auditor for the last.

All three stages must be funded before the first payment. Each stage pays in order. An accepted audit report does not guarantee bug-free software, and full funding does not guarantee delivery.

**Status:** proof-of-concept campaign terms; no dedicated deployment or end-to-end test for this schedule.

## Shared infrastructure and maintenance

Users and an ecosystem sponsor pay for a fixed period of maintenance.

- **Budget:** 240 tTARI; community 180, sponsor 60.
- **Payments:** 12 to start, then 72, 72, and 84 for three agreed work packages.
- **Review:** two of three named reviewers for the later payments.
- **Recipient:** the maintenance team, fixed before funding.

Write down what each work package includes, such as a wallet update, an explorer improvement, or documentation. Reviewers check delivery; the contract does not measure service quality. Renewal requires a new campaign.

**Status:** proof-of-concept campaign terms; no dedicated deployment or end-to-end test.

## Matched community grant

A foundation matches community funding for research, education, translations, or an open-source project.

- **Budget:** 200 tTARI; community 100, sponsor 100.
- **Payments:** 20 to start, 90 for an intermediate deliverable, 90 for completion.
- **Review:** three of five named reviewers for the later payments.
- **Recipient:** the grant recipient, fixed before funding.

Define concrete deliverables rather than broad promises. This is a fixed sponsor match, not a system that calculates funding from the number of unique donors. Donation size gives no extra review votes.

**Status:** proof-of-concept campaign terms; no dedicated deployment or end-to-end test.

## Community-only project

Members fund a shared resource without a sponsor.

- **Illustrative budget:** 100 tTARI, entirely from community members.
- **Illustrative payments:** 10 after full funding and 90 after delivery.
- **Review:** two of three named reviewers for the later payment.

**Status: separate proof-of-concept template, not supported by published v1.** The new `CommunityFunding` source has an explicit community-only mode, no sponsor key, zero sponsor budgets, and one exact-budget column per stage. Its activation and refund paths have not been exercised in an engine or on testnet. Do not submit these terms to the published v1 contract, which rejects a zero sponsor budget.

## Build and lightweight checks

Use Ubuntu WSL or Linux with the same Rust and WASM requirements as the original template. From the repository root:

```sh
cargo test --locked -p threshold-poc-core
cargo build --locked --target wasm32-unknown-unknown --release \
  -p threshold-exchange-template \
  -p threshold-audit-template \
  -p threshold-infrastructure-template \
  -p threshold-grant-template \
  -p threshold-community-template
```

The WASM files are named `threshold_exchange_template.wasm`, `threshold_audit_template.wasm`, `threshold_infrastructure_template.wasm`, `threshold_grant_template.wasm`, and `threshold_community_template.wasm` under the target directory's `wasm32-unknown-unknown/release/` folder. This session uses a separate Linux target directory, `target/poc-linux`, to avoid mixing Windows and Linux build files.

All five WASM builds and four lightweight tests passed in Ubuntu WSL. The tests cover example totals and encoding, funding-mode restrictions, sponsor-role admission, and selected recipient/reviewer/deadline validation. They do **not** construct private transfers or run an Ootle ledger. The full v1 engine suite, capacity tests, adversarial transaction tests, and network deployments were deliberately not run for these PoCs.

## Before turning an example into a real campaign

Replace placeholder roles with the participants' actual transaction-signing public keys. Agree on amounts, stage recipients, reviewer approvals, evidence requirements, and deadlines before anyone contributes. Ootle deadlines use network epochs, not calendar dates.

Funds already released cannot be refunded. Each contributor needs their own recovery backup, unlock material, transaction fees, and network access for independent recovery. The receiver must check payment decryptability before signing.

See [use cases](use-cases.md) for the product explanations, [the protocol](protocol.md) for the contract rules, and [verification](verification.md) for what has actually been tested.
