# Bounties powered by Threshold

This separate experimental Ootle template funds one GitHub bounty issue per component. It adapts the public [Tari bounty guidelines](https://github.com/tari-project/bounties/blob/main/GUIDELINES.md) into a council-reviewed escrow proposal. It is distinct from the earlier Threshold milestone-crowdfunding entry. **Tari has not adopted or audited this proposal.** The five examples were recorded on the former Esmeralda testnet before its reset; they use synthetic issues, demo reviewer identities, and test tTARI, not Tari treasury funds.

## Start here

| What to inspect | Link |
|---|---|
| Source | [Ootle template](src/lib.rs) and [transaction demo client](../threshold-bounty-demo/) |
| Offline archive | [Static Pages archive](https://seal-clubber.github.io/Threshold/bounties-archive.html) once this update is deployed; it bundles the saved state and action receipts without chain requests |
| Former-chain proof | [1-of-1 refund](../../evidence/bounty/one-refund-receipt.json) and [2-of-3 approval](../../evidence/bounty/two-approve-receipt.json) followed by a [claim](../../evidence/bounty/two-claim-receipt.json); the full record is [below](#archived-esmeralda-proof) |
| Check saved evidence | [Public receipt manifest](../../evidence/bounty/demo-index.json) and [`scripts/check-bounty-live.py --receipts-only`](../../scripts/check-bounty-live.py) |
| Rules and limitations | [Program mapping](../../docs/bounty-program.md) and [proposed issue format](../../docs/bounty-issue-template.md) |

The former-chain examples show a 1-of-1 no-work decision followed by a full treasury refund; a reviewer set changed from 1-of-1 to 2-of-3 **before funding**, then a two-reviewer award and contributor claim; and three bounties that were open and funded at 1-of-3, 1-of-1, and 2-of-3 when the snapshot was taken. Each used the S-tier amount of 15,000 test tTARI. The offline archive renders the saved records without an indexer. The separate current-chain board will list new bounties only after their addresses are added to its current manifest and the site is deployed.

To check the saved receipts from the repository root, run `python3 scripts/check-bounty-live.py --receipts-only`. It checks committed outcomes and manifest coverage without network access. The saved indexer snapshot is historical, not proof of the current chain. For local contract tests, use the [commands below](#qualification).

## Proposed flow

1. A designated council manager creates a bounty request for a `tari-project/` repository issue. The constructor records the issue, tier, issue-body digest, treasury funding key, one to five council reviewer keys, their quorum, and optional XL steward keys. The manager must be one of the reviewers and may update the digest, reviewer keys, and quorum before funding.
2. The funding key converts an exact public TARI bucket into one component-locked stealth output. The tier price and scope digest freeze on funding. There is no deadline.
3. Contributors still comment on GitHub as a courtesy, fork the repository, and submit a PR referencing the issue. The council manager checks the public work and proposes an award with a decision digest and PR digests. The configured reviewer quorum approves it. The contract cannot read GitHub or judge code quality.
4. An S/M/L award becomes claimable at council quorum. An XL award additionally needs the configured steward quorum. An award may split the full fixed tier price among up to eight distinct recipient keys.
5. Each awarded recipient signs a claim for their exact amount. A partial claim leaves one component-locked output for the remaining awardees. A final claim consumes it. The returned bucket is public TARI and should be deposited in the recipient's account in the same transaction.
6. Before a final award, council reviewers can record the same nonzero “no work” reason digest at any epoch. On review quorum, the original treasury funding key can claim the entire reserved amount.

The council quorum can stop work and authorize a full refund **only before a final award or any payout**. Paid funds cannot be reclaimed. If too few reviewers remain available, an unawarded funded bounty can remain locked indefinitely; this version has no timeout or alternate authority. The funder cannot cancel unilaterally.

## Contract boundary

- `Tier::{S,M,L,XL}` is fixed at 15,000 / 60,000 / 150,000 / 450,000 TARI in the resource's six-decimal base unit. Only Ootle's native `TARI_TOKEN` is accepted. On Esmeralda this is test tTARI; it is not a mainnet XTM payout.
- Exact funding is checked against the output's Pedersen commitment and supplied opening, then the native transfer proof is checked by Ootle. Claims check the single current input, exact revealed payout, and component lock on any remainder. The engine checks transfer conservation.
- A recipient must sign their own claim. The contract returns a bucket to the transaction; the signed transaction chooses where to deposit it.
- The GitHub issue, acceptance criteria, merged status, contributor identity, maintainer judgment, and reason for “no work” are represented by council-supplied digests. The contract authenticates reviewer keys and enforces their configured quorum, but does not verify the underlying off-chain facts.
- Any publisher can instantiate this template with its own reviewer key. A production program needs a council-approved canonical registry or address and an authenticated publication process so readers know which components are official. The template alone does not prevent duplicate components for one GitHub issue.

## Qualification

Policy tests cover tier amounts, split totals, issue identity, and 1-of-1, 1-of-3, and 2-of-3 reviewer configurations. A local Ootle engine test exercises exact funding, a 1-of-1 no-work decision and full refund at a far-future epoch, a pre-funding change to 2-of-3, frozen funded terms, two reviewer award transactions, a two-recipient split, unauthorized claim rejection, and both payout claims. These are local engine checks, not Esmeralda transactions or a security audit.

Run from the repository root in Ubuntu WSL:

```sh
cargo test -p threshold-bounty-template --test policy --locked
cargo test -p threshold-bounty-template --test engine --locked -- --nocapture
cargo build -p threshold-bounty-template --target wasm32-unknown-unknown --release --locked
```

Keep Linux and Windows target directories separate as described in the root README.

## Archived Esmeralda proof

The offline archive preserves the last saved state of these five synthetic components before the testnet reset. Names such as “Demo council member 1” are labels for test keys, not Tari appointments. The former template address was `29468dbacd59e120c99867c7e594a6a1e790d8f6c8d36c625ccd28abece07b01`. The [saved manifest](../../evidence/bounty/demo-index.json) and [snapshot](../../evidence/bounty/verified-state.json) preserve the component addresses; the decisive receipts below are kept in this repository.

| Review rule | Saved transaction receipts | Result at the snapshot |
|---|---|---|
| **1 of 1, issue #900001** | [No-work vote](../../evidence/bounty/one-no-work-receipt.json) · [Full refund](../../evidence/bounty/one-refund-receipt.json) | Funded with 15,000 tTARI; one reviewer closed as no work and the demo treasury reclaimed the full escrow. |
| **2 of 3, issue #900002** | [Set reviewers before funding](../../evidence/bounty/two-set-reviewers-receipt.json) · [Manager proposal](../../evidence/bounty/two-propose-receipt.json) · [Second approval](../../evidence/bounty/two-approve-receipt.json) · [Claim](../../evidence/bounty/two-claim-receipt.json) | Started at 1 of 1, changed to 2 of 3 before funding, then paid 15,000 tTARI after two council signatures. |
| **1 of 3, issue #900003** | [Set reviewers](../../evidence/bounty/three-set-reviewers-receipt.json) · [Fund](../../evidence/bounty/three-fund-receipt.json) | Open with 15,000 tTARI locked when recorded. |
| **1 of 1, issue #900004** | [Fund](../../evidence/bounty/four-fund-receipt.json) | Open with 15,000 tTARI locked when recorded. |
| **2 of 3, issue #900005** | [Set reviewers](../../evidence/bounty/five-set-reviewers-receipt.json) · [Fund](../../evidence/bounty/five-fund-receipt.json) | Open with 15,000 tTARI locked when recorded. |

The [public manifest](../../evidence/bounty/demo-index.json), [full-commit action receipts](../../evidence/bounty/), and [saved indexer snapshot](../../evidence/bounty/verified-state.json) accompany the transactions. Run `python3 scripts/check-bounty-live.py --receipts-only` from the repository root to check all retained receipts and manifest transaction IDs. The snapshot was captured before the reset and counted faucet setup receipts that are now archived locally outside the public checkout. It is not an independent consensus proof. The [demo client](../threshold-bounty-demo/) documents how the transactions were submitted.
