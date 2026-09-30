# Tari bounty program on Ootle: rule mapping and migration gaps

This is a proposal based on Tari's public [bounty board](https://github.com/tari-project/bounties), [detailed guidelines](https://github.com/tari-project/bounties/blob/main/GUIDELINES.md), and [Ootle tokenomics](https://ootle.tari.com/concepts/tokenomics/), read on 2026-09-30. It is not an official Tari program change. The experimental [bounty template](../crates/threshold-bounty-template/) implements an escrow for one issue. It does not edit Tari's repositories or move any existing program funds.

The [proposed issue format](bounty-issue-template.md) gives the council manager a place to maintain the request, record the approved scope and component, and publish award or no-work evidence. The [offline archive](https://seal-clubber.github.io/Threshold/bounties-archive.html) preserves reviewer keys, quorum, and saved state for five components from the former Esmeralda chain. The current-chain [read-only board](../app/bounties.html) awaits fresh transactions.

| Existing program rule | Proposed handling |
|---|---|
| Issues carry `bounty` and `bounty-S/M/L/XL` labels, description, binary acceptance criteria, and context. | Keep GitHub as the source for issue text and review. Record repository, issue number, tier, and a digest of the current scope on Ootle. A council manager can update the scope digest and review quorum before funding; both freeze afterward. |
| S/M/L/XL fixed prices are 15,000 / 60,000 / 150,000 / 450,000 XTM; no negotiation. | The template requires the corresponding exact amount of Ootle native TARI. Testnet funding uses tTARI. This preserves the numbers but changes the payout asset: Ootle's XTM-to-TARI bridge is one-way, with no peg-out to XTM. Tari must explicitly approve that change, a funding route, and any real migration. |
| Commenting signals intent but is not a lock; contributors fork, work, and submit a PR with a closing issue reference. | Keep this GitHub workflow. The template does not reserve work for a commenter. |
| A maintainer picks the best reviewed solution; public contributions may influence the choice and a significant multi-contributor effort may split the bounty. | A designated council manager proposes the accepted merged work and selects one to eight distinct recipient keys. The configured council reviewer quorum approves the frozen proposal. The award amounts must sum exactly to the fixed tier price. The contract cannot determine PR quality, merge order, or who contributed. |
| Workgroup lead approves S/M/L; steward body approves XL. | **Proposed governance change requested for this design:** the council manager is one of one to five configured reviewer keys, with a quorum such as 1-of-1, 1-of-3, or 2-of-3. S/M/L awards become final at council quorum. XL additionally requires a configured quorum of at least two distinct steward keys. Tari would need to accept this change. |
| Payout processing may take up to a week; contributor supplies a Tari address. | A claim becomes available when the on-chain award is final, with no time limit. Each recipient signs their claim and receives the exact public amount. Any off-chain identity and address collection remains a program responsibility. |
| An issue may receive no acceptable work or be closed. | Council reviewers record the same “no work” reason digest before a final award, at any epoch. It takes the configured review quorum to close. The original treasury funding key then signs a separate transaction to reclaim the full untouched escrow. No automatic deadline exists. |

The board README says the first PR that passes review and is merged wins. The detailed guideline gives the maintainer final discretion and explicitly permits splitting a bounty. This proposal follows the detailed guideline for the award decision. Tari should settle that wording before a production migration.

## Open-ended funding and its trade-off

There is no funding, review, claim, or cancellation deadline in this template. This matches an issue that may remain open for an unknown period. It also means that if fewer than the required council reviewers remain available before an award or “no work” decision, the escrow has no recovery path. A production version could use a council-approved replacement or recovery quorum, but adding one changes who can close a bounty and should be decided by Tari governance.

Once an award is final, “no work” cannot return the full amount. Awardees can claim in any order, and a paid claim cannot be undone. The funder has no unilateral refund path.

## What a full program replacement still requires

1. **Tari authorization and canonical identity.** Tari must designate the council manager, reviewer keys and quorum, steward set, treasury funding key, and official registry/address. The current template permits anyone to publish a lookalike component.
2. **GitHub-to-Ootle operations.** A maintained board or CLI must import issue details, compute scope and decision digests, show review and merge evidence, and prepare funding and payout transactions. The smart contract cannot inspect GitHub itself.
3. **Wallet and network qualification.** The [archived synthetic Esmeralda examples](../crates/threshold-bounty-template/README.md#archived-esmeralda-proof) cover S-tier funding, a no-work refund, reviewer changes, two approvals, and a winner claim on the former testnet. Fresh-chain transactions, a production signer flow, adversarial engine tests, XL steward exercise, and independent output recovery checks remain to be done.
4. **Existing bounty migration.** Reconcile open, in-review, merged, and already paid issues before funding any on-chain component. Do not create a second payable promise for an existing award.
5. **Mainnet policy and security review.** Confirm the L1 XTM to L2 TARI funding route, custody policy, fee payer, emergency succession process, and audit requirements. The current version has no audit or production claim.

The existing [Threshold campaign proof](verification.md) verifies a different v1 contract. The separate [bounty receipts](../evidence/bounty/) and [pre-reset component snapshot](../evidence/bounty/verified-state.json) document the synthetic bounty examples only.
