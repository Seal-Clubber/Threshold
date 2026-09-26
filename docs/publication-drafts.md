# Contest publication drafts

The repository and demo links are filled in. Replace the Tari payment address and social-post placeholders before posting. Post the social announcement first, then paste its public URL into the forum reply. Do not publish a testnet recovery key, encrypted package, or unlock file.

## September forum reply

**Threshold — confidential matched funding with recoverable milestones on Ootle**

Threshold is an experimental protocol for jointly funding shared software, audits, and developer bounties. Contributors lock native Ootle confidential outputs into separate milestone cells. An ownerless component checks the exact community and sponsor amounts for every stage and activates the entire bundle together. A sponsor cannot stand in for missing community funding. An agreed upfront payment can follow full activation; later payments require the frozen reviewer quorum. After a missed funding or delivery deadline, each owner can recover their own eligible unpaid cells directly with a portable encrypted package and standalone client.

The interactive bounty map lets you switch individual pledges, activation, payment, delivery, review, and refund paths on and off. Its separate sandbox previews other campaign terms. The site is an illustrative simulation; it does not sign, submit, or query chain transactions. The map's 10% upfront example passed a local Ootle engine test; the already deployed Esmeralda campaign has immutable 20% upfront terms.

Beyond bounties, the same matched milestone structure could fund agreed exchange integration work, security audits, shared infrastructure, and community grants. These are use-case illustrations, not separately tested deployments. Exchange acceptance remains external. Community-only funding without a sponsor is a proposed extension: the published v1 requires a positive sponsor share in every stage.

What ran on Esmeralda testnet: four native private pledge records, exact community/sponsor activation, two milestone payouts, and a recipient re-spend; a separate sponsor-missing campaign that rejected activation and later allowed the contributor's direct refund and private re-spend; and a partial-delivery campaign where both the community owner and sponsor independently refunded their unpaid cells after the deadline. Separate fixtures activated and paid out 8, 16, and 32 private pledge records. Those scaling fixtures reused signing keys and do **not** demonstrate 32 independent people. Transactions, receipts, dry runs, and fees are recorded in `docs/demo.md` and `evidence/` within the repository.

This is MIT-licensed, testnet-only, and unaudited. Individual output values use Ootle's native confidentiality, but signer keys, participation, public budgets, and timing remain visible; a coordinator can learn openings shared for matching. Reviewers judge work quality, and the recipient must check payout decryptability before signing because the component cannot prove the ciphertext decrypts. The current published template caps a campaign at 32 pledge records, 5 reviewer keys, and 8 stages.

- Code and technical documentation: https://github.com/Seal-Clubber/Threshold
- Interactive demo: https://seal-clubber.github.io/Threshold/
- Live testnet component: https://ootle-indexer-a.tari.com/substates/component_82cad9632a4f5d2d6f07768d478a26a1e7032774faae6ab1f987a65fb7c97b58
- Tari payment address: [YOUR TARI PAYMENT ADDRESS]
- Public social announcement: [YOUR SOCIAL POST URL]

## Social announcement to post first

Entering Tari's September Ootle contest with Threshold: confidential milestone pledges, an exact sponsor match, atomic bundle activation, reviewer-gated payments, and direct owner refunds after deadlines. The bounty map is interactive; the Esmeralda results have source-linked receipts. Testnet-only and unaudited. https://seal-clubber.github.io/Threshold/ https://github.com/Seal-Clubber/Threshold #Tari #Ootle
