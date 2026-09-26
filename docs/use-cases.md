# What can Threshold fund?

**Suggested positioning: Crowdfund shared work. Release funds by milestone.**

Threshold combines confidential contribution cells, an enforceable sponsor match, atomic bundle activation, ordered milestone payments, and direct recovery of eligible unpaid funds. Market that combination as conditional crowdfunding for shared work. The current reference implementation is experimental, testnet-only, and unaudited; these examples are applications of its rules, not claims that every workflow below has been deployed.

Separate [proof-of-concept Ootle template crates](campaign-templates.md) now implement the five additional use cases, including a sponsor-free variant. They have compile and basic term checks only, no engine or testnet qualification, and no compatible recovery client yet. The table below describes the published v1 rather than upgrading those PoCs to tested products.

| Use case | Concrete example | Current fit |
|---|---|---|
| Developer bounties | Several applications and an ecosystem sponsor pay for a shared SDK feature, with a start payment only after full funding and later payment after review. | Fits v1's matched, ordered milestones. The map illustrates this; its 10/90 split has a local engine fixture. |
| Exchange listing or integration costs | Community and sponsor fund agreed technical integration, testing, and venue-confirmed milestones. | Fits the funding structure when recipients participate in the required Ootle payment flow. Exchange acceptance and off-chain performance remain external. |
| Community-only crowdfunding | Members fund a community website, event, or public resource without any sponsor role. | Requires a new template and client qualification. The published v1 rejects a zero sponsor budget. |
| Development and security audit bundles | Fund a shared library and a separate auditor together, with different recipients on sequential stages. | Fits v1's atomic funding and stage-specific recipients. Funding together does not guarantee delivery together or bug-free software. |
| Shared infrastructure and maintenance | Users and a sponsor pay for a fixed period of explorer work, wallet integration, documentation, or maintenance. | Fits finite milestones. V1 is not an indefinite subscription: at most eight ordered stages, then a new campaign for renewal. |
| Matched community grants | A foundation matches a defined community budget for education, translations, research, or an open-source project. | Fits exact role budgets and reviewer-approved delivery. No quadratic matching, donor identity proof, or automatic sponsor-weighted voting. |

## Exchange costs: fund agreed work, not a promised market outcome

Obtain the exchange's agreement on scope and payment conditions before taking pledges. An illustrative campaign might fund integration implementation, testing, and an exchange-confirmed launch as separate stages. Frozen terms name the recipients, exact community and sponsor budgets, reviewers, evidence policy, and deadlines. A reviewer quorum approves the agreed evidence; the chain cannot verify an external exchange's decision or operations by itself.

Each recipient must be willing and able to receive the configured Ootle resource and authorize the required release transaction. If an integration team receives the funds and pays an exchange by another method, that downstream transfer is a trusted off-chain action. Threshold does not enforce currency conversion, payment on another chain, or compliance with the exchange's agreement.

If the exchange requires a fee upfront, the campaign can make that an explicitly agreed first tranche after full funding. Once released, that money is at risk and cannot be clawed back through Threshold if the venue later declines or delays a listing. Only eligible unpaid cells remain recoverable after deadlines. The campaign does not guarantee exchange acceptance, listing duration, liquidity, or token price. No exchange workflow is claimed as separately testnet-qualified here.

## Community-only funding: an explicit v1 limitation

The published [term validation](../crates/threshold-template/src/lib.rs) requires `stage.community > 0 && stage.sponsor > 0` for every stage. Terms freeze one `sponsor_key`; pledge registration binds that key to the sponsor role and allows one sponsor pledge. Activation checks both role columns independently. The sandbox therefore does not offer a zero-sponsor configuration.

Members could voluntarily agree that one of them supplies the sponsor leg, but that remains a campaign with a required designated matching role. It is not sponsorless crowdfunding, and it should not be advertised as such. Pooling other members' money under that person's key would also give them custody and refund control over that pooled pledge; this is not a substitute for independent owner recovery.

The separate `threshold-community-template` PoC now contains an explicit funding mode and a community-only exact-budget activation path, retaining per-owner cells and a refund method. Client validation, transaction construction, recovery-package support, and ledger qualification remain unfinished. Qualification must cover underfunding, role misuse, zero-valued or omitted columns, replay, deadline races, partial payments, and individual refunds in both modes. It requires a newly published immutable template; changing the website cannot upgrade the existing deployment. The sponsor requirement is a Threshold v1 design choice, not evidence that Ootle inherently requires sponsors.

## Additional ways to describe the product

- **Shared infrastructure funding:** organizations co-fund work they all depend on.
- **Milestone crowdfunding:** contributors commit to specific payment and recovery rules before pledging.
- **Conditional matching grants:** a sponsor's matching promise becomes locked funding under the same deadlines.
- **Joint procurement for open-source work:** multiple buyers fund defined deliverables, optionally with separate recipients per stage.

Creator commissions, community education, and event preparation could use the same matched milestone structure with appropriate recipients and reviewers. These remain proposed use cases. Prefer concrete deliverables and observable evidence over promises of outcomes the reviewers cannot establish.

Avoid claims of guaranteed delivery, anonymous donors, unlimited contributors, or fully trustless crowdfunding. Reviewers judge work quality; recipients must check private output decryptability before signing; public terms, keys, participation, and timing remain visible. A coordinator may learn openings shared for matching. Recovery requires the owner's package and unlock material, fee funds, and a working network data route. The published caps are 32 pledge records, five reviewer keys per stage, and eight stages.

See [the protocol](protocol.md), [security boundaries](security.md), and [verification ledger](verification.md) for enforceable rules and the specific tested fixtures.
