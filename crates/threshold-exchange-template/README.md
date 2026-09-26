# ExchangeFunding

**Proof of concept only. Not deployed, audited, or tested end to end.**

An Ootle WASM template with an example constructor and customizable campaign terms. See the [template guide](../../docs/campaign-templates.md) for its payment schedule, API, funding rules, and build commands.

Compilation and basic term checks passed. Ledger execution, private transfers, recovery, and testnet behavior have not been qualified for this template. The existing v1 client and recovery packages do not support its experimental version 100 terms.

Shared contract logic is in [threshold-poc-core](../threshold-poc-core/). The published [v1 template](../threshold-template/) remains separate.
