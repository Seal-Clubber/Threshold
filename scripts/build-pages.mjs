import { mkdir, readFile, writeFile, copyFile, readdir, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = join(root, '_site');
const app = join(root, 'app');
const bountyEvidenceDir = join(root, 'evidence', 'bounty');
const currentBountyEvidence = join(bountyEvidenceDir, 'current-index.json');

await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
const html = (await readFile(join(app, 'wire.html'), 'utf8'))
  .replace('href="/wire.css"', 'href="./wire.css"')
  .replace('src="/wire.js"', 'src="./wire.js"')
  .replace('src="/sandbox.js"', 'src="./sandbox.js"');
await writeFile(join(output, 'index.html'), html);
await copyFile(join(app, 'wire.css'), join(output, 'wire.css'));
await copyFile(join(app, 'wire.js'), join(output, 'wire.js'));
await copyFile(join(app, 'sandbox.js'), join(output, 'sandbox.js'));
await copyFile(join(app, 'bounties.html'), join(output, 'bounties.html'));
await copyFile(join(app, 'bounties.css'), join(output, 'bounties.css'));
await copyFile(join(app, 'bounties.js'), join(output, 'bounties.js'));
let current = { network: 'esmeralda', template: '', people: [], bounties: [] };
try {
  current = JSON.parse(await readFile(currentBountyEvidence, 'utf8'));
} catch (error) {
  if (error.code !== 'ENOENT') throw error;
}
if (current.network !== 'esmeralda' || !Array.isArray(current.people) || !Array.isArray(current.bounties)) throw new Error('Invalid current bounty index');
const bountyIndex = {
  network: current.network,
  template: current.template,
  people: current.people.map(({ name, key }) => ({ name, key })),
  bounties: current.bounties.map(({ issue, component, reviewers, quorum, transactions }) => ({ issue, component, reviewers, quorum, transactions })),
};
await writeFile(join(output, 'bounty-index.json'), JSON.stringify(bountyIndex));

// Preserve the pre-reset demonstration as a self-contained historical page.
const archive = JSON.parse(await readFile(join(bountyEvidenceDir, 'demo-index.json'), 'utf8'));
const snapshot = JSON.parse(await readFile(join(bountyEvidenceDir, 'verified-state.json'), 'utf8'));
if (archive.network !== 'esmeralda' || snapshot.network !== 'esmeralda' || archive.bounties.length !== 5 || snapshot.components.length !== 5) throw new Error('Invalid archived bounty evidence');
const escapeHtml = value => String(value).replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
const archiveDir = join(output, 'bounty-proof');
await mkdir(archiveDir);
await copyFile(join(bountyEvidenceDir, 'demo-index.json'), join(archiveDir, 'demo-index.json'));
await copyFile(join(bountyEvidenceDir, 'verified-state.json'), join(archiveDir, 'verified-state.json'));
const receipts = new Map();
for (const file of await readdir(bountyEvidenceDir)) {
  if (!/^[a-z0-9-]+-receipt\.json$/.test(file)) continue;
  const record = JSON.parse(await readFile(join(bountyEvidenceDir, file), 'utf8'));
  if (record.receipt?.outcome !== 'Commit' || !/^[a-f0-9]{64}$/.test(record.transaction_id)) throw new Error(`Invalid archived receipt: ${file}`);
  receipts.set(record.transaction_id, file);
  await copyFile(join(bountyEvidenceDir, file), join(archiveDir, file));
}
const people = new Map(archive.people.map(person => [person.key.toLowerCase(), person.name]));
const statuses = { 0: 'Open', 4: 'Refunded', 5: 'Paid' };
const labels = { create: 'Create', setReviewers: 'Set reviewers', fund: 'Fund', propose: 'Propose', approve: 'Approve', noWork: 'No work', refund: 'Refund', claim: 'Claim' };
const transactionOrder = ['create', 'setReviewers', 'fund', 'propose', 'approve', 'noWork', 'refund', 'claim'];
const treasury = archive.people.find(person => person.name === 'Demo Tari treasury');
if (!treasury) throw new Error('Archived treasury label missing');
const metric = (label, value, extra = '') => `<div class="metric"><small>${escapeHtml(label)}</small><strong class="${extra}">${escapeHtml(value)}</strong></div>`;
const cards = archive.bounties.map(entry => {
  const state = snapshot.components.find(component => component.issue === entry.issue && component.component === entry.component);
  if (!state || state.review_quorum !== entry.quorum || !statuses[state.status_code]) throw new Error(`Archive mismatch: ${entry.issue}`);
  const balance = state.status_code === 0 ? ['Currently locked', '15,000 tTARI'] : state.status_code === 4 ? ['Full refund', '15,000 tTARI'] : ['Total paid', '15,000 tTARI'];
  const reviewers = state.reviewer_keys.map((key, index) => {
    const vote = state.award_votes[index] ? ['Award ✓', 'yes'] : state.no_work_votes[index] ? ['No work ✓', 'no-work'] : ['No vote', ''];
    return `<div class="reviewer"><div class="reviewer-top"><strong>${escapeHtml(people.get(key.toLowerCase()) || 'Demo reviewer')}${index === 0 ? ' · request manager' : ''}</strong><span class="vote ${vote[1]}">${vote[0]}</span></div><code class="key">${escapeHtml(key)}</code></div>`;
  }).join('');
  const transactions = transactionOrder.filter(action => entry.transactions[action]).map(action => {
    const id = entry.transactions[action];
    const file = receipts.get(id);
    if (!file) throw new Error(`Missing archived receipt: ${id}`);
    return `<a href="./bounty-proof/${file}" title="${escapeHtml(id)}">${escapeHtml(labels[action])}</a>`;
  }).join('');
  return { status: state.status_code, html: `<article class="card"><div class="card-head"><div><div class="card-kicker">SYNTHETIC ISSUE #${entry.issue}</div><h3>Tari bounty #${entry.issue}</h3><div class="subhead">Fixed tier · Tari-style Ootle escrow</div></div><span class="status ${state.status_code === 0 ? 'pending' : ''}">${statuses[state.status_code]}</span></div><div class="metrics">${metric('Tier', 'S · 15,000 tTARI')}${metric('Council rule', `${state.review_quorum} of ${state.reviewer_keys.length}`, 'quorum')}${metric(...balance)}</div><div class="review-title">Funding signer</div><div class="reviewer"><strong>${escapeHtml(treasury.name)}</strong><code class="key">${escapeHtml(treasury.key)}</code></div><div class="review-title">Assigned council reviewer keys</div><div class="reviewers">${reviewers}</div><div class="card-bottom"><span class="chain-note">✓ Saved indexer-verified state</span><div><a class="component-link" href="./bounty-proof/verified-state.json" title="${escapeHtml(entry.component)}">View saved state ↗</a><div class="tx-links">${transactions}</div></div></div></article>` };
}).sort((a, b) => ({ 0: 0, 5: 1, 4: 2 }[a.status] - { 0: 0, 5: 1, 4: 2 }[b.status])).map(card => card.html).join('\n');
const archiveHtml = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="theme-color" content="#071522"><title>Bounties powered by Threshold</title><meta name="description" content="Offline archive of five pre-reset Esmeralda bounty escrows and their committed receipts."><link rel="stylesheet" href="./bounties.css"></head>
<body><div class="shell"><header class="topbar"><a class="brand" href="./">threshold<span>.</span></a><span class="network-pill">ESMERALDA TESTNET</span><a class="back-link" href="./">← Interactive demo</a></header><main><section class="hero"><p class="eyebrow">A PROPOSED TARI BOUNTY WORKFLOW</p><h1>Bounties powered by Threshold.</h1><p class="lead">Each bounty fixes its council reviewer keys and approval threshold when funded. The saved cards below show the escrow balance, council decisions, and committed transactions.</p><p class="archive-time"><strong>Historical snapshot.</strong> Recorded ${escapeHtml(snapshot.checked_at_utc)} before the Esmeralda reset. These are synthetic issues and demo identities, not current funds or Tari-appointed council members.</p><p><a href="./bounty-proof/demo-index.json">Saved manifest</a> · <a href="./bounty-proof/verified-state.json">Saved snapshot</a></p></section><section class="board"><div class="section-head"><div><p class="eyebrow">SAVED CONTRACT STATE</p><h2>Bounty escrows</h2></div><span class="count">5 components · 45,000 tTARI locked</span></div><div class="bounty-list">${cards}</div></section><section class="explainer"><div><p class="eyebrow">THE DECISION RULE</p><h2>1 of 1 or 2 of 3. Set before funding.</h2></div><p>A council member creates and can revise the request. The reviewer list and quorum are adjustable until funding; funding freezes them. A matching quorum approves an award or a no-work closure. XL awards also need their configured steward quorum. There is no deadline. A no-work closure lets the original treasury signer reclaim the full untouched escrow before a final award.</p></section><section class="discovery"><div><p class="eyebrow">ADDING A BOUNTY</p><h2>How does it appear here?</h2></div><p>This saved page shows the five original demo components and never queries a chain. A future current-chain board will read newly published bounty components from its approved address list.</p><p>A council-controlled on-chain registry could eventually hold approved component addresses and reviewer name labels. That would let the board discover new bounties without a site deployment.</p></section><footer>Each card reproduces the saved indexer-verified state and links to bundled full-commit receipts. This is historical evidence, not an independent consensus proof or a current-chain balance. The page does not query the network or submit transactions.</footer></main></div></body></html>`;
await writeFile(join(output, 'bounties-archive.html'), archiveHtml);
await writeFile(join(output, '.nojekyll'), '');
console.log(`Built the interactive map, current bounty board, and offline proof archive at ${output}`);
