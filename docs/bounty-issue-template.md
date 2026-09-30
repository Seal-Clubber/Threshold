# Proposed Tari bounty issue template for Ootle escrow

This is a copyable issue format for a **proposed** Tari-run program. It does not create an official bounty or attest that a component is canonical. Tari must designate the council manager, reviewer keys and quorum, and treasury funding key, then publish the verified component address. See the [rule mapping](bounty-program.md).

Apply the `bounty` label and exactly one `bounty-S`, `bounty-M`, `bounty-L`, or `bounty-XL` label. The fixed tier amounts below are Ootle TARI, not a promise to pay L1 XTM. Ootle has no TARI-to-XTM peg-out.

```md
## Bounty: [specific title]

**Repository:** tari-project/[repository]
**Issue:** #[number]
**Tier:** [S / M / L / XL]
**Ootle escrow amount:** [15,000 / 60,000 / 150,000 / 450,000] TARI
**Council manager:** @[designated member]
**Council reviewer keys and name lookup:** [one to five designated members with Ootle public keys]
**Review quorum:** [1-of-1 / 1-of-3 / 2-of-3 / other valid quorum]
**Funding signer:** [designated Tari treasury key]
**Verified Ootle component:** [component address and official registry link, once published]
**Scope digest:** [32-byte digest of this approved request, with the method used]
**Funding transaction:** [link when funded]
**Status:** Draft / Funded / In review / Awarded / No work / Refunded / Paid

### Description

[One paragraph explaining the work to someone new to the codebase.]

### Acceptance criteria

- [ ] [Specific, verifiable pass/fail condition]
- [ ] [Specific, verifiable pass/fail condition]

### Context

[Relevant code, documentation, RFCs, and discussion links.]

### How it works

1. Comment here to signal intent. A comment does not reserve the bounty.
2. Fork the repository and do the work. AI-assisted development is welcome.
3. Submit a PR with a closing issue reference such as `Fixes #[number]`.
4. Multiple PRs may compete. Review comments are public. The council manager proposes the best reviewed and merged solution based on quality, completeness, and contributor behavior. The manager may propose a split when multiple people contributed significantly. The configured council reviewer quorum approves the award. For XL, the configured steward quorum must also approve it.
5. Each awardee signs a claim from the Ootle escrow. There is no claim deadline.

### Reviewer decision record

**Accepted merged PR(s):** [links]
**Award split and recipient keys:** [amounts summing to the fixed tier price]
**Decision digest and transaction:** [links]
**Council reviewer votes:** [signed transaction links matching the review quorum]
**XL steward approvals:** [links, if applicable]

### No-work closure, if applicable

**Reason and evidence:** [why no acceptable work was awarded]
**Council no-work vote transactions:** [links matching the review quorum and same reason digest]
**Original funder's full-refund transaction:** [link]
```

The council manager may revise the request, its on-chain scope digest, reviewer keys, and quorum before funding. Once funded, those terms and the tier are fixed. There is no deadline for work or review. Before the award is final, the reviewer quorum can record no work at any time; the original treasury funder then signs a separate transaction to reclaim the full escrow. A final award or any payout cannot be reversed into a full refund.

Tari's [detailed guidelines](https://github.com/tari-project/bounties/blob/main/GUIDELINES.md) also size bounties by scope clarity, required codebase knowledge, verification complexity, and risk surface. A council reviewer should apply those criteria before funding and seek extra review for consensus, wallet, bridge, or signing work.
