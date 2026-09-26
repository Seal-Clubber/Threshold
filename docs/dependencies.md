# Dependency and originality notes

Threshold's own source is [MIT-licensed](../LICENSE). Its protocol and client use released Tari/Ootle crates from crates.io, pinned by `Cargo.lock`. Direct Tari crates and their license declarations, checked in their downloaded `Cargo.toml` files, are:

| Crate | Pinned version | Declared license |
|---|---:|---|
| `tari_template_lib` | 0.32.1 | BSD-3-Clause |
| `tari_engine_types` | 0.41.2 | BSD-3-Clause |
| `tari_ootle_transaction` | 0.41.2 | BSD-3-Clause |
| `tari_bor` | 0.16.1 | BSD-3-Clause |
| `ootle_byte_type` | 0.13.0 | BSD-3-Clause |
| `tari_ootle_wallet_crypto` | 0.43.0 | BSD-3-Clause |
| `ootle-rs` | 0.23.0 | BSD-3-Clause |
| `tari_crypto` | 0.23.2 | BSD-3-Clause |
| `tari_template_test_tooling` (test-only) | 0.41.2 | BSD-3-Clause |

This table covers the direct Tari crates, not every transitive dependency. The upstream v0.41.2 release archive is kept locally for protocol inspection under Git-ignored `.upstream/`; its source is not copied into Threshold. No code from contest submissions was imported. A full transitive license inventory should accompany a production distribution.
