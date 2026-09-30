# GitHub Pages demo

Published demo: [seal-clubber.github.io/Threshold](https://seal-clubber.github.io/Threshold/).

The repo includes a static build and a Pages workflow. The interactive bounty map and its integrated campaign sandbox remain simulations. The current-chain bounty board has no entries after the testnet reset. A separate offline archive renders five former-chain examples from a saved indexer snapshot and bundles their public action receipts. No wallet, private recovery material, transaction submission, showcase, or observer is published.

## Preview before uploading

From the repository root, run `node scripts/build-pages.mjs`, `node scripts/check-pages.mjs`, then `node scripts/preview-pages.mjs`. Open `http://127.0.0.1:4766/threshold/`. The preview includes a project subpath like GitHub Pages; the generated `_site/` directory is ignored by Git. The build uses only Node's standard library.

The build copies the map and board assets into `_site/`, with relative paths. It generates the active `bounty-index.json` from `evidence/bounty/current-index.json` when a fresh-chain manifest exists; until then the active board is empty. The offline `bounties-archive.html` is generated from the saved former-chain manifest and snapshot. It bundles 19 public bounty action receipts, the manifest, and the snapshot under `_site/bounty-proof/`. New current-chain components need a new manifest and site deployment before they appear. The build does **not** copy `.local/`, journals, unlock keys, donor setup records, or the older campaign evidence archive.

## Manual browser upload

Run `node scripts/prepare-manual-upload.mjs`. It creates an ignored `github-upload/` folder containing numbered batches of at most 100 files each. The script copies only Git-visible public files, preserves their relative paths, checks the copies, and keeps `.local/`, `_site/`, build artifacts, and recovery secrets out. Read `github-upload/UPLOAD-INSTRUCTIONS.txt` and drag the **contents** of each batch into the repository root in order. GitHub's browser uploader currently limits each upload to 100 files; do not upload the parent `github-upload/` folder or a ZIP file.

## Publish after uploading

1. Upload all prepared batches to a public GitHub repository, committing each batch to its default branch. Alternatively, push this repository with Git.
2. In **Settings → Pages → Build and deployment**, choose **GitHub Actions** as the source.
3. The `Publish Threshold demo` workflow deploys on a push to the default branch. It can also be run from **Actions**. GitHub shows the site URL in the Pages settings and workflow deployment.

The build uses relative links and works at both `username.github.io/repository/` and a custom-domain root. No repository name needs to be hard-coded.

The larger Sandbox includes seven presets, editable milestone recipients and review thresholds, separate funding sliders, a recovery preview, and a local JSON sketch download. It distinguishes current v1 configuration limits, sponsor-free PoC support, and future designs such as multiple sponsors or larger campaigns. These future previews are not evidence of implemented support or capacity. The downloaded sketch is not a deployment payload. Run `node scripts/check-sandbox.mjs` to check the preview arithmetic and support classification; these are browser-model checks, not contract tests.

GitHub Pages serves static files only. The map and sandbox are illustrative simulations and never send transactions or query the chain. The archive page makes no network requests and labels every state as historical. Once a fresh-chain manifest exists, the current board sends read requests to the Esmeralda indexer and requires its `verified` flag and the expected template address before rendering contract state. It does not independently verify consensus or discover every bounty; a Tari-approved registry would be needed for an official board. Sandbox inputs calculate an independent hypothetical campaign and never change the fixed bounty map or either board. The sandbox flags terms beyond the published v1 template's 32-pledge, 5-reviewer, and 8-stage caps.
