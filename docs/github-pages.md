# GitHub Pages demo

The repo includes a static build and a Pages workflow. The published site is one page: the interactive bounty map and its integrated campaign sandbox. No wallet, private recovery material, transaction submission, showcase, observer, or evidence archive is published.

## Preview before uploading

From the repository root, run `node scripts/build-pages.mjs`, `node scripts/check-pages.mjs`, then `node scripts/preview-pages.mjs`. Open `http://127.0.0.1:4766/threshold/`. The preview includes a project subpath like GitHub Pages; the generated `_site/` directory is ignored by Git. The build uses only Node's standard library.

The build copies only `wire.html`, `wire.css`, `wire.js`, and `sandbox.js` into `_site/`, with relative asset paths. It does **not** copy `.local/`, journals, unlock keys, or `evidence/`.

## Manual browser upload

Run `node scripts/prepare-manual-upload.mjs`. It creates an ignored `github-upload/` folder containing five numbered batches of at most 100 files each. The script copies only Git-visible public files, preserves their relative paths, checks the copies, and keeps `.local/`, `_site/`, build artifacts, and recovery secrets out. Read `github-upload/UPLOAD-INSTRUCTIONS.txt` and drag the **contents** of each batch into the repository root in order. GitHub's browser uploader currently limits each upload to 100 files; do not upload the parent `github-upload/` folder or a ZIP file.

## Publish after uploading

1. Upload all five prepared batches to a public GitHub repository, committing each batch to its default branch. Alternatively, push this repository with Git.
2. In **Settings → Pages → Build and deployment**, choose **GitHub Actions** as the source.
3. The `Publish Threshold demo` workflow deploys on a push to the default branch. It can also be run from **Actions**. GitHub shows the site URL in the Pages settings and workflow deployment.

The build uses relative links and works at both `username.github.io/repository/` and a custom-domain root. No repository name needs to be hard-coded.

GitHub Pages serves static files only. The map and sandbox are illustrative simulations and never send transactions or query the chain. Sandbox inputs calculate an independent hypothetical campaign and never change the fixed bounty map. The existing Esmeralda deployment has immutable terms; changing a planner input does not change that contract. The sandbox flags terms beyond the published template's 32-pledge, 5-reviewer, and 8-stage caps. Source evidence remains available in the repository and is not served by the local demo or published Pages artifact.
