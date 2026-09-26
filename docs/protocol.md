# Threshold v1 — segregated confidential tranches

Status: testnet deployed and exercised, unaudited. Engine and network qualification are tracked separately in [verification.md](verification.md). Values are integer microtari (1 TARI = 1,000,000 units).

## Architecture decision

Reject a naive pooled confidential vault: hiding a balance does not provide a proportional-refund proof. Reject independent spend scripts checking `OutputTo`: it proves only a lower bound for one output in one statement, permits other outputs unless excluded, and cannot coordinate mutable milestone state. `KeyAndScript` has an alternative spend path and cannot hold escrow safely.

Choose contributor-specific, pre-split stealth UTXOs governed by ONE immutable, ownerless component. Each pledge contains one confidential UTXO per milestone. The component executes and validates registration transfers, recording commitments and a fresh refund public key, never amounts or openings. Every escrow UTXO is pure `Script` with a single native `AccessRule::ScopedToComponent` leaf naming this exact component. This gates resource spending in the current frame; `caller_component` is a different rule and is not substituted.

Activation changes the component's state atomically for the entire bundle. Funds are already locked; activation need not move them or destroy their refund attribution. It selects whole pledges and verifies exact aggregate values independently for each (community/sponsor, milestone) cell using the native Ristretto intrinsics:

`sum(C_i) = aggregate_mask * G + frozen_budget * H`.

`H` is Tari's fixed Pedersen value generator, never caller-chosen. The engine's real range/balance proofs at registration establish each commitment's value is nonnegative and funded. Aggregate masks reveal only the already-public cell totals, not individual openings. This is a standard Pedersen opening check, not arbitrary computation over encrypted balances and not a new zero-knowledge proof system. Its composition still requires review. Small groups, collusion and public totals can reveal individual amounts; no anonymity or independent-person counting is claimed.

## Frozen terms and states

The constructor freezes version, resource, campaign nonce, funding epoch, ordered milestones, per-role budgets, recipients, reviewer keys/quorum, and increasing delivery deadlines. No owner, upgrade, arbitrary execution, recipient replacement, emergency sweep or terms editing method exists. V1 supports at most 8 milestones, 32 registered pledges, 5 reviewers per milestone. The full 32-pledge cap was exercised once on Esmeralda; higher counts need a new template and qualification.

`Funding --activate before funding_deadline--> Active --last release--> Complete`

`Funding at/after funding_deadline -> refundable`; `Active at/after next unreleased milestone deadline -> stalled/refundable`. Expiry is derived from consensus epoch, not a backend clock. No early refund while activation remains possible. At exact deadline refunds win and activation/release fail. No further releases after a missed stage deadline. Unselected pledges become refundable immediately on activation. Each pledge can refund once. Completed/paid cells can never refund.

The sponsor is identified by a frozen signer key and registered in a separate role. The same key cannot register as community. This excludes the sponsor's registered escrow from community totals, but cannot establish beneficial ownership across alternate keys. Sponsor funding is locked on the same terms as community funding. No sponsor escape key exists. Multiple wallets are not independent people.

Oversubscription selects a subset of whole pledges with EXACT frozen column sums. No partial selection, silent excess donation or arbitrary change. A coordinator can propose matching subsets, but it cannot activate a subset with incorrect sums. If no exact subset exists, contributors recover after the deadline and can fund a new campaign. No cancellation administrator exists.

The demonstration freezes milestones of 2,000 TARI library start (upfront), 4,000 TARI library delivery, and 4,000 TARI audit. Community supplies 60% of every stage, sponsor 40%. Thus library receives 6,000 and audit 4,000; no activation API accepts omitted legs. Funding does not guarantee joint delivery. Upfront money is at risk.

A new campaign can choose a smaller upfront tranche. For a 100 TARI budget with 60 community and 40 sponsor, a 10% upfront stage freezes community 6 plus sponsor 4, and a later reviewer-gated stage freezes community 54 plus sponsor 36. **None of the 10 TARI upfront amount can be released until the entire 100 TARI bundle activates with both roles fully pledged.** The local Ootle engine test rejects this upfront release before activation, then accepts it after exact activation and preserves separate refunds of the unpaid 90 TARI after the later deadline. The stage split is immutable after campaign creation. The existing live main fixture uses 20%; this 10/90 example is tested locally, not deployed as a new testnet campaign.

Each contributor approves a private vector of tranche values. Default clients apportion integer values with deterministic remainder allocation, but the ledger enforces the frozen aggregate matrix, not proportional equality between all contributors. This is an explicit alternative to pooled proportional refunds. Refund = sum of that contributor's untouched tranche commitments; there is no refund rounding and no unsecured IOU. Sponsor receives only its own untouched tranches. Fees are funded separately, never deducted from escrow.

## Spending and approvals

Registration accepts only a confidential transfer with no revealed inputs or outputs and exactly one output per milestone, all locked to this component, each with the constant minimum promise of one microtari. It executes that transfer in the same call that records its outputs. It refuses any registered escrow commitment as an input, so the component cannot be used to recycle deposits or evade payout checks.

A stage releases all selected contributors' corresponding cells in ONE native transfer. The component requires the exact input set once each, no additional input, zero revealed inflow/outflow, exactly one output, and key-path authorization. Only the frozen recipient may call release; their signed transaction chooses the output key/ciphertext, enabling normal private wallet output creation. The recipient must verify decryption before signing. A coordinator cannot redirect or corrupt a payout unilaterally. The recipient can authorize an output to another key; the component does not enforce that the ciphertext decrypts to the frozen recipient. V1 exposes the recipient's transaction signing key; recipient anonymity is not claimed.

For non-upfront stages each designated reviewer signs an on-chain approval call naming stage and evidence hash. Distinct reviewer votes, the component address, immutable stage budget/recipient, sequential stage index and `epoch < stage.deadline` bind authority. Quorum must agree on the same evidence. One vote per reviewer per stage; no approval replay into another stage/component. Reviewers are trusted to judge delivery, not to custody or redirect funds. Disagreement or disappearance leads to timed refunds. CI/GitHub artifacts are evidence only. The live fixture chooses separate reviewer and recipient keys; v1 permits a future campaign to choose overlapping keys, so human or even key-level independence is not a contract guarantee.

Contributors do not each vote on delivery. The campaign freezes a small reviewer set and quorum before pledging; sponsor contribution size gives no automatic or weighted review vote. The main live fixture uses two named reviewers and requires both. A new campaign can choose another quorum within its named set, up to five reviewer keys in the published template. Existing campaigns cannot change that choice.

Refunds are signed by the pledge's refund key and cover EXACTLY its unspent eligible cells, no external inputs, one key-authorized output, no revealed funds. Native balance proof conserves the entire entitlement. No coordinator approval is involved. A user can call the immutable component directly through any functioning network client.

## Invariants / threat model

Adversaries may control coordinator, website, indexer, any contributor, recipient or reviewer subset below quorum. They can submit arbitrary transactions and bypass all clients. Network consensus, native cryptography and immutable component code are assumed correct. Recipient/reviewer collusion can authorize poor work; fees and network availability are outside the protocol.

1. Only funded, registered, pure-script UTXOs count; a commitment is registered once.
2. Role-separated exact sums satisfy every frozen bundle leg simultaneously.
3. Each registered commitment is paid or refunded at most once; spend input sets are exhaustive.
4. No main-intent failure is treated as successful activation just because fees committed.
5. No public escrow outflow, unapproved stage, premature withdrawal or administrative drain.
6. Refunding unselected/failure/stalled pledges does not depend on coordinator availability.

## Responsibility boundary

| Responsibility | Enforcement |
|---|---|
| Range/balance proofs; no double-spend; atomic main transaction | Ootle engine |
| Exact registration/spend sets, role sums, deadlines, votes, state | Threshold WASM component |
| Amounts, masks, subset search, proof assembly | Contributor/coordinator; amounts known to coordinator if shared |
| Evidence quality | Designated reviewer quorum |
| Decryptable payout construction | Receiving wallet, which signs the exact release/refund transaction |
| Recovery backup, fee reserve, independent network access | Contributor |
| Progress indexing | Advisory; only committed main result + component state establishes success |

## Recovery material

Retain network/resource/component/template identity and hashes; frozen spec; pledge ID; each UTXO commitment, amount and blinding mask; condition leaf and inclusion proof (single leaf: empty siblings); refund signing key or wallet key derivation; creation transaction references; recipient address/keys; and separate fee funds. Seed alone does not reconstruct the application's masks, witness or pledge mapping. Export before registration is signed; update references after commit. Encrypt locally with an established password KDF and AEAD. Authenticate format/network/component context. Recovery must inspect fresh component state and UTXO status and fail closed on missing/contradictory responses. Because Ootle cannot infer the UTXO address from an encoded component method argument, every pledge, release, and refund transaction explicitly declares its input UTXOs.

## Unproven boundaries

This design avoids the stated missing covenant primitives by retaining per-contributor claims and using a component's full state. It does not establish permissionless confidential donor counting, fair proportional pooled refunds, proof of delivery, confidentiality from a coordinator given openings, or network-independent recovery. Missed-funding, partial-delivery, and final audit owner refunds committed on Esmeralda from a clean client; the missed-funding and one audit refund were privately re-spent. The pinned local engine and Esmeralda both accepted exact activation and payout at the published template's 32-pledge cap. Community records in the capacity fixtures reused test signing keys, so they do not evidence independent people. See [verification.md](verification.md) for the exact evidence and remaining boundaries.
