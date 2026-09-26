import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const privateDir = resolve(root, '.local', 'fresh-client');
const publicDir = resolve(root, 'evidence');
const cases = [
  { name: 'partial-community', group: 'partial', amount: 3_000_000 },
  { name: 'partial-sponsor', group: 'partial', amount: 2_000_000 },
  { name: 'alice', group: 'audit', amount: 6_000_000 },
  { name: 'bob', group: 'audit', amount: 8_000_000 },
  { name: 'carol', group: 'audit', amount: 10_000_000 },
  { name: 'sponsor', group: 'audit', amount: 16_000_000 },
];

async function json(path) { return JSON.parse(await readFile(path, 'utf8')); }
function requireValue(condition, message) { if (!condition) throw new Error(message); }

const deployments = {
  partial: await json(resolve(publicDir, 'partial-deployment.json')),
  audit: await json(resolve(publicDir, 'deployment.json')),
};
const metrics = { partial: [], audit: [] };
for (const item of cases) {
  const { name, group, amount } = item;
  const journal = await json(resolve(privateDir, `${name}.journal.json`));
  const receipt = await json(resolve(privateDir, `${name}.receipt.json`));
  requireValue(journal.component === deployments[group].component, `${name}: component mismatch`);
  requireValue(journal.amount_microtari === amount, `${name}: amount mismatch`);
  requireValue(/^[a-f0-9]{64}$/.test(journal.transaction_id), `${name}: invalid transaction ID`);
  requireValue(receipt.outcome === 'Commit', `${name}: main intent did not commit`);
  requireValue(receipt.epoch >= (group === 'partial' ? 11445 : 11454), `${name}: premature refund`);
  const filename = group === 'partial' ? `${name}-refund-receipt.json` : `audit-${name}-refund-receipt.json`;
  await writeFile(resolve(publicDir, filename), `${JSON.stringify({ transaction_id: journal.transaction_id, receipt }, null, 2)}\n`);
  metrics[group].push({
    owner: name, transaction_id: journal.transaction_id, amount_microtari: amount,
    epoch: receipt.epoch, outcome: receipt.outcome,
    fee_microtari: receipt.fee_receipt.total_fees_paid,
    native_proof_ms: journal.native_proof_ms,
    encoded_transaction_bytes: journal.encoded_transaction_bytes,
  });
}
for (const group of ['partial', 'audit']) {
  const entries = metrics[group];
  await writeFile(resolve(publicDir, `${group}-recovery-metrics.json`), `${JSON.stringify({
    network: 'esmeralda', component: deployments[group].component,
    total_refunded_microtari: entries.reduce((sum, entry) => sum + entry.amount_microtari, 0),
    refunds: entries,
  }, null, 2)}\n`);
}
const journal = await json(resolve(privateDir, 'carol.respend.journal.json'));
const receipt = await json(resolve(privateDir, 'carol.respend.receipt.json'));
requireValue(receipt.outcome === 'Commit' && /^[a-f0-9]{64}$/.test(journal.transaction_id), 'Carol re-spend did not commit');
await writeFile(resolve(publicDir, 'audit-carol-refund-respend-receipt.json'),
  `${JSON.stringify({ transaction_id: journal.transaction_id, receipt }, null, 2)}\n`);
console.log('Exported six committed owner refunds and Carol\'s private re-spend.');
