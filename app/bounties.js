const indexer = 'https://ootle-indexer-a.tari.com';
const list = document.querySelector('#bounty-list');
const sync = document.querySelector('#sync-status');
const count = document.querySelector('#count');
const refreshButton = document.querySelector('#refresh');
const statusNames = ['Open', 'Review pending', 'Awarded', 'No work', 'Refunded', 'Paid'];
const tierNames = ['S', 'M', 'L', 'XL'];
let manifest;

function normalized(value) { return String(value || '').toLowerCase().replace(/^(0x|template_)/, ''); }
function hexKey(value) { return typeof value === 'string' ? normalized(value) : normalized(value?.hex); }
function text(parent, tag, value, className) {
  const node = document.createElement(tag);
  node.textContent = String(value);
  if (className) node.className = className;
  parent.append(node);
  return node;
}
function link(parent, label, url, className) {
  const node = text(parent, 'a', label, className);
  node.href = url;
  node.target = '_blank';
  node.rel = 'noopener noreferrer';
  return node;
}
function nameFor(key) {
  return manifest.people.find(person => normalized(person.key) === key)?.name || 'Unlabelled key';
}
function metric(parent, label, value, extra) {
  const box = document.createElement('div');
  box.className = 'metric';
  text(box, 'small', label);
  text(box, 'strong', value, extra);
  parent.append(box);
}
function escrowMetric(state) {
  const remaining = Number(state[16]) / 1e6;
  const fullTier = [15000, 60000, 150000, 450000][state[4]];
  if (state[14] === 4) return ['Full refund', `${fullTier.toLocaleString()} tTARI`];
  if (state[14] === 5) return ['Total paid', `${fullTier.toLocaleString()} tTARI`];
  if (state[14] === 0 && remaining === 0) return ['Escrow', 'Awaiting funding'];
  if (state[14] === 3) return ['Available to refund', `${remaining.toLocaleString()} tTARI`];
  return ['Currently locked', `${remaining.toLocaleString()} tTARI`];
}
function transactionLinks(parent, transactions) {
  const wrap = document.createElement('div');
  wrap.className = 'tx-links';
  const labels = { create: 'Create', setReviewers: 'Set reviewers', fund: 'Fund', propose: 'Propose', approve: 'Approve', noWork: 'No work', refund: 'Refund', claim: 'Claim' };
  for (const label of ['create', 'setReviewers', 'fund', 'propose', 'approve', 'noWork', 'refund', 'claim']) {
    const id = transactions?.[label];
    if (/^[a-f0-9]{64}$/i.test(id)) link(wrap, labels[label], `${indexer}/transactions/${id}`);
  }
  parent.append(wrap);
}
function renderCard(entry, state, verified, error) {
  const card = document.createElement('article');
  card.className = 'card';
  const head = document.createElement('div');
  head.className = 'card-head';
  const heading = document.createElement('div');
  text(heading, 'div', `SYNTHETIC ISSUE #${entry.issue}`, 'card-kicker');
  text(heading, 'h3', state ? `${state[1]} #${state[2]}` : `Component ${entry.component.slice(10, 24)}…`);
  text(heading, 'div', 'Fixed tier · Tari-style Ootle escrow', 'subhead');
  head.append(heading);
  const status = text(head, 'span', state ? statusNames[state[14]] || 'Unknown' : 'Unavailable', `status ${!state ? 'error' : state[14] === 1 || state[14] === 0 ? 'pending' : ''}`);
  status.setAttribute('aria-label', `Contract status: ${status.textContent}`);
  card.append(head);
  if (state) {
    const metrics = document.createElement('div');
    metrics.className = 'metrics';
    metric(metrics, 'Tier', `${tierNames[state[4]] || '?'} · ${[15000,60000,150000,450000][state[4]]?.toLocaleString() || '?'} tTARI`);
    metric(metrics, 'Council rule', `${state[7]} of ${state[6].length}`, 'quorum');
    metric(metrics, ...escrowMetric(state));
    card.append(metrics);
    const treasuryKey = hexKey(state[10]);
    text(card, 'div', 'Funding signer', 'review-title');
    const treasury = document.createElement('div');
    treasury.className = 'reviewer';
    text(treasury, 'strong', nameFor(treasuryKey));
    text(treasury, 'code', treasuryKey, 'key');
    card.append(treasury);
    text(card, 'div', 'Assigned council reviewer keys', 'review-title');
    const reviewers = document.createElement('div');
    reviewers.className = 'reviewers';
    state[6].forEach((raw, i) => {
      const key = hexKey(raw);
      const row = document.createElement('div');
      row.className = 'reviewer';
      const top = document.createElement('div');
      top.className = 'reviewer-top';
      text(top, 'strong', `${nameFor(key)}${key === hexKey(state[5]) ? ' · request manager' : ''}`);
      const noWork = /^[a-f0-9]{64}$/i.test(state[9]?.[i]?.hex);
      text(top, 'span', state[8]?.[i] ? 'Award ✓' : noWork ? 'No work ✓' : 'No vote', `vote ${state[8]?.[i] ? 'yes' : noWork ? 'no-work' : ''}`);
      row.append(top);
      text(row, 'code', key, 'key');
      reviewers.append(row);
    });
    card.append(reviewers);
  } else {
    text(card, 'p', error || 'The component could not be read from the indexer.', 'subhead');
  }
  const bottom = document.createElement('div');
  bottom.className = 'card-bottom';
  text(bottom, 'span', verified ? '✓ Indexer-verified live state' : 'Chain state unavailable', `chain-note ${verified ? '' : 'error'}`);
  const links = document.createElement('div');
  link(links, 'View component ↗', `${indexer}/substates/${entry.component}`, 'component-link');
  transactionLinks(links, entry.transactions);
  bottom.append(links);
  card.append(bottom);
  return card;
}
async function getState(entry) {
  if (!/^component_[a-f0-9]{64}$/i.test(entry.component)) throw new Error('Invalid component address in manifest');
  const response = await fetch(`${indexer}/substates/${entry.component}`, { cache: 'no-store' });
  if (!response.ok) throw new Error(`Indexer returned HTTP ${response.status}`);
  const data = await response.json();
  if (data.verified !== true) throw new Error('Indexer has not verified this component');
  const component = data.substate?.Component;
  if (!component) throw new Error('Substate is not a component');
  if (normalized(component.header?.template_address) !== normalized(manifest.template)) throw new Error('Unexpected contract template');
  const state = component.body?.state;
  if (!Array.isArray(state) || state[0] !== 2 || state[2] !== entry.issue) throw new Error('Component version or issue does not match the manifest');
  if (!Array.isArray(state[6]) || !Array.isArray(state[8]) || !Array.isArray(state[9])) throw new Error('Invalid reviewer state');
  return state;
}
async function refresh() {
  if (!manifest) return;
  refreshButton.disabled = true;
  sync.textContent = 'Checking Esmeralda…';
  const results = await Promise.all(manifest.bounties.map(async entry => {
    try { return { entry, state: await getState(entry), verified: true }; }
    catch (error) { return { entry, error: error.message, verified: false }; }
  }));
  const order = result => {
    if (!result.state) return 7;
    const status = result.state[14];
    if (status === 0) return Number(result.state[16]) > 0 ? 0 : 4;
    return { 1: 1, 2: 2, 3: 3, 5: 5, 4: 6 }[status] ?? 7;
  };
  results.sort((a, b) => order(a) - order(b));
  list.replaceChildren(...results.map(result => renderCard(result.entry, result.state, result.verified, result.error)));
  const good = results.filter(result => result.verified).length;
  const locked = results.reduce((sum, result) => sum + (Number(result.state?.[16]) || 0), 0) / 1e6;
  count.textContent = `${results.length} component${results.length === 1 ? '' : 's'} · ${locked.toLocaleString()} tTARI locked`;
  sync.textContent = `${good}/${results.length} verified · checked ${new Date().toLocaleTimeString()}`;
  refreshButton.disabled = false;
}
async function start() {
  try {
    const response = await fetch('./bounty-index.json', { cache: 'no-store' });
    if (!response.ok) throw new Error(`Manifest returned HTTP ${response.status}`);
    manifest = await response.json();
    if (!Array.isArray(manifest.bounties) || !Array.isArray(manifest.people)) throw new Error('Invalid bounty manifest');
    if (manifest.bounties.length === 0) {
      list.replaceChildren(text(document.createElement('div'), 'p', 'No bounties have been published on the current testnet. The former chain’s five examples and saved receipts are available in the offline proof archive linked above.', 'empty'));
      sync.textContent = 'Awaiting current-chain transactions';
      count.textContent = '0 current components';
      refreshButton.disabled = true;
      return;
    }
    await refresh();
    setInterval(refresh, 30_000);
  } catch (error) {
    list.replaceChildren(text(document.createElement('div'), 'p', error.message, 'empty'));
    sync.textContent = 'Could not load the bounty registry';
  }
}
refreshButton.addEventListener('click', refresh);
start();
