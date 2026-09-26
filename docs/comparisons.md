# Related funding protocols

Threshold's contribution is a particular combination: native Ootle confidential outputs, exact role-separated matching across a jointly activated bundle, ordered milestone spending, and contributor-specific timed recovery. It does not invent crowdfunding, matching, stealth payments, or milestone governance.

| System | Relevant capability | Material difference from Threshold v1 |
|---|---|---|
| [Allo Protocol](https://docs.allo.gitcoin.co/) | Modular pools and allocation/distribution strategies for on-chain capital allocation. | Threshold fixes one campaign's exact private UTXO cells and owner recoveries in an immutable Ootle component. It is less general; it does not claim an Allo-compatible strategy. |
| [Juicebox](https://docs.juicebox.money/) | Project funding cycles, rules and payouts. | Threshold's reference case requires both community and sponsor columns for every bundled milestone in one activation, with individual private values and separate untouched-cell refunds. |
| [clr.fund](https://clr.fund/) | Quadratic funding with MACI-based contribution privacy and anti-collusion goals. | Threshold does **not** calculate quadratic matching, prove unique humans, or protect contributions from a coordinator who receives openings. Its sponsor match is an exact 60/40 monetary condition. |

These are architectural comparisons, not compatibility or security rankings. Threshold's public terms, timing, signer keys, campaign participation and small-group arithmetic can reveal private values. Its exact-value matching is enforceable only because the component checks native Pedersen commitments and the selected UTXO set; a website claim would not suffice. The first template is unaudited and testnet-only.
