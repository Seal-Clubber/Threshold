// Pure arithmetic; no contract or wallet calls. Amounts use integer microtari.
function sandboxApportion(total, weights) {
  const sum = weights.reduce((a, b) => a + b, 0);
  const raw = weights.map(w => total * w / sum);
  const values = raw.map(Math.floor);
  const order = raw.map((v, i) => ({ i, remainder: v - values[i] })).sort((a, b) => b.remainder - a.remainder || a.i - b.i);
  const left = total - values.reduce((a, b) => a + b, 0);
  for (let i = 0; i < left; i++) values[order[i].i]++;
  return values;
}
function calculateSandbox(c) {
  const budget = c.budget * 1000000;
  const community = c.mode === 'community' ? budget : Math.round(budget * c.share / 100);
  const sponsor = budget - community;
  const upfrontCommunity = Math.round(community * c.upfront / 100);
  const upfrontSponsor = Math.round(sponsor * c.upfront / 100);
  const activeStages = c.upfront === 100 ? [] : c.milestones;
  const weights = activeStages.map(s => s.weight);
  const communityParts = sandboxApportion(community - upfrontCommunity, weights);
  const sponsorParts = sandboxApportion(sponsor - upfrontSponsor, weights);
  const schedule = [];
  if (c.upfront > 0) schedule.push({ name: 'Start payment', recipient: c.startRecipient, community: upfrontCommunity, sponsor: upfrontSponsor, reviewers: 0, quorum: 0, upfront: true });
  activeStages.forEach((s, i) => schedule.push({ ...s, community: communityParts[i], sponsor: sponsorParts[i], upfront: false }));
  schedule.forEach((s, i) => { s.amount = s.community + s.sponsor; s.deadline = c.fundingEpochs + (i + 1) * c.stageEpochs; });
  const errors = [];
  const future = [];
  const sponsors = c.mode === 'community' ? 0 : c.mode === 'matched' ? 1 : c.sponsors;
  if (c.pledges < sponsors + 1) errors.push('Allow at least one community pledge plus one pledge per sponsor.');
  if (activeStages.some(s => s.quorum > s.reviewers)) errors.push('A milestone asks for more approvals than it has reviewers.');
  if (schedule.some(s => s.community <= 0 || (sponsor > 0 && s.sponsor <= 0))) errors.push('Every funded role needs a positive amount in every stage.');
  if (c.mode === 'collective') future.push('Multiple sponsor identities need a new funding model, client support and testing.');
  if (c.pledges > 32) future.push(`${c.pledges} pledge records exceed the current 32-record cap.`);
  if (schedule.length > 8) future.push(`${schedule.length} payment stages exceed the current 8-stage cap.`);
  if (activeStages.some(s => s.reviewers > 5)) future.push('A review group exceeds the current 5-key cap.');
  const support = future.length ? 'future' : c.mode === 'community' ? 'poc' : 'v1';
  const funded = c.communityProgress === 100 && (sponsor === 0 || c.sponsorProgress === 100);
  const committedCommunity = Math.floor(community * c.communityProgress / 100);
  const committedSponsor = Math.floor(sponsor * c.sponsorProgress / 100);
  const paidCount = funded && !errors.length ? Math.min(c.paid, schedule.length) : 0;
  const paid = schedule.slice(0, paidCount).reduce((sum, s) => sum + s.amount, 0);
  const unpaid = committedCommunity + committedSponsor - paid;
  return { community, sponsor, sponsors, schedule, errors, future, support, funded, paidCount, paid, unpaid, eligible: c.expired && !errors.length ? unpaid : 0, committedCommunity, committedSponsor };
}

// DOM layer: independent from the fixed bounty story.
const sandboxEl = id => document.getElementById(id);
const sandboxFields = ['name', 'mode', 'budget', 'share', 'pledges', 'sponsors', 'upfront', 'stages', 'funding-epochs', 'stage-epochs', 'start-recipient', 'progress', 'sponsor-progress', 'paid', 'expired'];
const sandboxPresets = {
  bounty: { name: 'A feature the community needs', budget: 100, share: 60, upfront: 10, pledges: 4, recipient: 'Developer', stages: [['Deliver the feature', 1, 'Developer', 2, 2]], description: 'A familiar starting point: full funding first, a small start payment, then reviewed delivery.' },
  community: { name: 'A project funded by its community', mode: 'community', budget: 500, upfront: 0, pledges: 25, stages: [['Working prototype', 2, 'Project team', 3, 2], ['Community handover', 3, 'Project team', 3, 2]], description: 'No separate sponsor. The community supplies the whole budget. A sponsor-free PoC exists; its client and ledger qualification are unfinished.' },
  exchange: { name: 'Exchange listing and integration', budget: 1000, share: 60, upfront: 10, pledges: 20, recipient: 'Integration team', stages: [['Integration accepted', 4, 'Integration team', 3, 2], ['Agreed listing work completed', 5, 'Exchange recipient', 3, 2]], description: 'Fund technical work and agreed costs in stages. Funding cannot guarantee a listing; off-chain exchange payments need separate arrangements. Use-case template: PoC.' },
  audit: { name: 'Build it, then audit it', budget: 500, share: 70, upfront: 10, pledges: 16, recipient: 'Development team', stages: [['Deliver implementation', 5, 'Development team', 3, 2], ['Publish the audit', 4, 'Independent auditor', 3, 2]], description: 'Different work, different recipients. Development and audit budgets activate together. Use-case template: PoC; distinct recipients do not prove independence.' },
  maintenance: { name: 'Six rounds of shared maintenance', budget: 1200, share: 75, upfront: 0, pledges: 24, stages: Array.from({length: 6}, (_, i) => [`Maintenance round ${i + 1}`, 1, 'Maintainer', 3, 2]), description: 'A finite set of maintenance milestones, funded in advance. These are reviewed payments, not an automatic subscription. Use-case template: PoC.' },
  research: { name: 'Community research grant', budget: 2000, share: 50, upfront: 20, pledges: 30, recipient: 'Research team', stages: [['Method and early findings', 2, 'Research team', 5, 3], ['Open dataset', 3, 'Research team', 5, 3], ['Final report', 3, 'Research team', 5, 3]], description: 'A foundation matches community funding. Separate checkpoints support a longer research project. The grant template is a PoC.' },
  many: { name: 'A shared infrastructure fund', mode: 'collective', sponsors: 3, budget: 10000, share: 60, upfront: 5, pledges: 203, recipient: 'Build team', stages: [['Core infrastructure', 5, 'Build team', 7, 5], ['Independent review', 2, 'Audit team', 5, 3], ['Documentation and handover', 3, 'Documentation team', 7, 5]], description: 'Explore a larger community and several sponsors backing one goal. This is a future design sketch, not an implemented or capacity-tested template.' },
};
let sandboxMilestones = [];
function sandboxInt(name, min, max) {
  const value = Number(sandboxEl(`sandbox-${name}`).value);
  return Number.isFinite(value) ? Math.min(max, Math.max(min, Math.trunc(value))) : min;
}
function sandboxAmount(micro) {
  return `${(micro / 1000000).toLocaleString(undefined, { maximumFractionDigits: micro % 10000 === 0 ? 2 : 6 })} tTARI`;
}
function readSandbox() {
  const mode = sandboxEl('sandbox-mode').value;
  return {
    name: sandboxEl('sandbox-name').value.trim() || 'Untitled campaign', mode,
    budget: sandboxInt('budget', 1, 1000000), share: sandboxInt('share', 1, 99), upfront: sandboxInt('upfront', 0, 100),
    pledges: sandboxInt('pledges', 1, 10000), sponsors: mode === 'community' ? 0 : mode === 'matched' ? 1 : sandboxInt('sponsors', 2, 100),
    fundingEpochs: sandboxInt('funding-epochs', 1, 10000), stageEpochs: sandboxInt('stage-epochs', 1, 10000),
    startRecipient: sandboxEl('sandbox-start-recipient').value.trim() || 'Start recipient', milestones: sandboxMilestones,
    communityProgress: sandboxInt('progress', 0, 100), sponsorProgress: sandboxInt('sponsor-progress', 0, 100),
    paid: sandboxInt('paid', 0, 25), expired: sandboxEl('sandbox-expired').checked,
  };
}
function sandboxText(tag, text, className) {
  const el = document.createElement(tag); el.textContent = text; if (className) el.className = className; return el;
}
function renderSandboxEditors() {
  const count = sandboxInt('stages', 1, 24);
  if (sandboxMilestones.length === count && sandboxEl('sandbox-editors').children.length === count) return;
  sandboxMilestones = Array.from({length: count}, (_, i) => sandboxMilestones[i] || {name: `Deliverable ${i + 1}`, weight: 1, recipient: 'Project team', reviewers: 3, quorum: 2});
  const cards = sandboxMilestones.map((stage, index) => {
    const card = document.createElement('fieldset'); card.className = 'sandbox-editor';
    card.append(sandboxText('legend', `Milestone ${index + 1}`));
    const fields = [['name', 'Deliverable', 'text'], ['recipient', 'Recipient', 'text'], ['weight', 'Budget weight', 'number'], ['reviewers', 'Reviewer keys', 'number'], ['quorum', 'Approvals needed', 'number']];
    fields.forEach(([key, title, type]) => {
      const label = document.createElement('label'); label.className = `sandbox-field sandbox-edit-${key}`;
      label.append(sandboxText('span', title));
      const input = document.createElement('input'); input.type = type; input.value = stage[key];
      input.setAttribute('aria-label', `Milestone ${index + 1}: ${title}`);
      if (type === 'number') { input.min = '1'; input.max = key === 'weight' ? '100' : '50'; input.step = '1'; } else input.maxLength = 60;
      input.addEventListener('input', () => {
        stage[key] = type === 'number' ? Math.max(1, Math.min(Number(input.max), Math.trunc(Number(input.value) || 1))) : input.value;
        sandboxMarkCustom(); renderSandbox();
      });
      input.addEventListener('change', () => { input.value = stage[key]; });
      label.append(input); card.append(label);
    });
    return card;
  });
  sandboxEl('sandbox-editors').replaceChildren(...cards);
}
function sandboxMarkCustom() {
  document.querySelectorAll('[data-sandbox-preset]').forEach(b => b.setAttribute('aria-pressed', 'false'));
  sandboxEl('sandbox-preset-description').textContent = 'Your own configuration. The support notes update as you change its funding model, scale and review rules.';
  sandboxEl('sandbox-export-status').textContent = '';
}
function renderSandbox() {
  renderSandboxEditors();
  const c = readSandbox(), r = calculateSandbox(c);
  sandboxEl('sandbox-share').disabled = c.mode === 'community';
  sandboxEl('sandbox-sponsors').disabled = c.mode !== 'collective';
  if (c.mode !== 'collective') sandboxEl('sandbox-sponsors').value = r.sponsors;
  sandboxEl('sandbox-stages').disabled = c.upfront === 100;
  sandboxEl('sandbox-editors').hidden = c.upfront === 100;
  sandboxEl('sandbox-start-recipient-field').hidden = c.upfront === 0;
  sandboxEl('sandbox-sponsor-progress-field').hidden = c.mode === 'community';
  sandboxEl('sandbox-share-value').textContent = `${c.mode === 'community' ? 100 : c.share}%`;
  sandboxEl('sandbox-upfront-value').textContent = `${c.upfront}%`;
  sandboxEl('sandbox-progress-value').textContent = `${c.communityProgress}%`;
  sandboxEl('sandbox-sponsor-progress-value').textContent = `${c.sponsorProgress}%`;
  sandboxEl('sandbox-community-result').textContent = sandboxAmount(r.community);
  sandboxEl('sandbox-sponsor-result').textContent = sandboxAmount(r.sponsor);
  sandboxEl('sandbox-first-result').textContent = sandboxAmount(r.schedule[0].amount);
  const reviewed = r.schedule.filter(s => !s.upfront);
  Array.from(sandboxEl('sandbox-editors').children).forEach((card, i) => {
    card.querySelector('legend').textContent = `Milestone ${i + 1}${reviewed[i] ? ` · ${sandboxAmount(reviewed[i].amount)}` : ''}`;
  });
  const labels = {v1: 'Within v1’s current configuration limits', poc: 'Sponsor-free model · proof of concept', future: 'Future design · new implementation needed'};
  const viability = sandboxEl('sandbox-viability');
  viability.className = `sandbox-viability ${r.errors.length ? 'invalid' : r.support}`;
  viability.textContent = r.errors.length ? 'Resolve these settings to continue' : labels[r.support];
  const notes = [...r.errors, ...r.future];
  if (c.mode === 'community') notes.push('Sponsor-free template code exists, but client integration, recovery validation and end-to-end tests remain unfinished.');
  if (r.support === 'v1') notes.push('These settings fit the current counts and funding shape. This is not a deployed campaign or validation of wallet keys, proofs or a use-case template.');
  if (r.support === 'future') notes.push('The preview illustrates intended behaviour only. Contract and client work, transaction-limit checks and fresh tests are required; feasibility is not established.');
  if (c.mode === 'collective') notes.push(`The ${r.sponsors} sponsors are modelled as one aggregate pool. Individual sponsor commitments and matching ratios are not simulated.`);
  if (c.upfront === 100) notes.push('All funds would be released at the start. No reviewed payment or unpaid recovery balance would remain after that payment.');
  sandboxEl('sandbox-support-notes').replaceChildren(...notes.map(n => sandboxText('li', n)));
  const gate = sandboxEl('sandbox-gate');
  gate.classList.toggle('open', r.funded && !r.errors.length);
  gate.textContent = r.errors.length ? 'Fix the inconsistent settings above before interpreting the outcome.' : c.expired
    ? 'The relevant deadline has passed in this scenario. No new activation or current-stage payment; inspect the eligible unpaid balance below.' : !r.funded
    ? `Waiting for ${sandboxAmount(r.community - r.committedCommunity)} from the community and ${sandboxAmount(r.sponsor - r.committedSponsor)} from sponsors. No start payment.`
    : `In this scenario the funding target is met. All stage amounts still need exact verification together. ${c.upfront > 0 ? `${sandboxAmount(r.schedule[0].amount)} is the proposed start payment.` : 'No upfront payment: the first milestone needs review.'}`;
  sandboxEl('sandbox-paid').max = String(r.schedule.length);
  sandboxEl('sandbox-paid').disabled = !r.funded || !!r.errors.length;
  sandboxEl('sandbox-paid').value = String(r.paidCount);
  sandboxEl('sandbox-paid-value').textContent = `${r.paidCount} / ${r.schedule.length}`;
  sandboxEl('sandbox-recovery').textContent = r.errors.length ? 'Recovery preview unavailable until settings are consistent.'
    : r.paidCount === r.schedule.length ? 'All stages paid. Nothing remains to recover.'
    : c.expired ? `Hypothetical eligible unpaid balance: ${sandboxAmount(r.eligible)}. ${sandboxAmount(r.paid)} already paid stays with recipients. Each owner claims their own remaining cells; this is not an automatic refund.`
    : `${sandboxAmount(r.paid)} already paid · ${sandboxAmount(r.unpaid)} still committed. Recovery opens after the relevant ${r.funded ? 'next unpaid stage' : 'funding'} deadline.`;
  sandboxEl('sandbox-schedule').replaceChildren(...r.schedule.map((s, i) => {
    const row = sandboxText('div', '', 'sandbox-stage');
    const head = sandboxText('div', '', 'sandbox-stage-head');
    head.append(sandboxText('span', `${String(i + 1).padStart(2, '0')} · ${s.name || 'Untitled milestone'}`), sandboxText('strong', sandboxAmount(s.amount)));
    row.append(head, sandboxText('small', `${s.recipient || 'Unnamed recipient'} · ${s.upfront ? 'After full funding, no review' : `${s.quorum} of ${s.reviewers} approvals on the same evidence`}`), sandboxText('small', `Community ${sandboxAmount(s.community)} · Sponsors ${sandboxAmount(s.sponsor)} · deadline: creation + ${s.deadline} epochs`));
    return row;
  }));
}
function setSandboxPreset(name) {
  const p = sandboxPresets[name];
  const values = {name: p.name, mode: p.mode || 'matched', budget: p.budget, share: p.share || 60, upfront: p.upfront, pledges: p.pledges, sponsors: p.sponsors || 3, stages: p.stages.length, 'funding-epochs': 20, 'stage-epochs': 20, 'start-recipient': p.recipient || 'Project team', progress: 80, 'sponsor-progress': 100, paid: 0};
  Object.entries(values).forEach(([k, v]) => { sandboxEl(`sandbox-${k}`).value = v; });
  sandboxEl('sandbox-expired').checked = false;
  sandboxMilestones = p.stages.map(([name, weight, recipient, reviewers, quorum]) => ({name, weight, recipient, reviewers, quorum}));
  sandboxEl('sandbox-editors').replaceChildren();
  sandboxEl('sandbox-preset-description').textContent = p.description;
  sandboxEl('sandbox-export-status').textContent = '';
  document.querySelectorAll('[data-sandbox-preset]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.sandboxPreset === name)));
  renderSandbox();
}
function exportSandbox() {
  const c = readSandbox(), r = calculateSandbox(c);
  if (r.errors.length) { sandboxEl('sandbox-export-status').textContent = 'Resolve the settings before saving.'; return; }
  const data = { format: 'threshold-campaign-sketch-v1', purpose: 'Illustrative design only. Not contract terms, a wallet payload, or a deployment file.', implementation: r.support, caveats: [...r.future, ...(c.mode === 'community' ? ['Sponsor-free PoC: client integration and end-to-end testing unfinished.'] : []), 'Amounts and labels do not validate keys, proofs, recovery or transaction feasibility.'], configuration: c, computed: r };
  const url = URL.createObjectURL(new Blob([JSON.stringify(data, null, 2)], {type: 'application/json'}));
  const link = document.createElement('a'); link.href = url; link.download = 'threshold-campaign-sketch.json'; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  sandboxEl('sandbox-export-status').textContent = 'Saved a design sketch, not deployment instructions.';
}
sandboxEl('sandbox-open').addEventListener('click', () => sandboxEl('sandbox-dialog').showModal());
sandboxEl('configuration-open').addEventListener('click', () => sandboxEl('sandbox-dialog').showModal());
sandboxEl('demo-open').addEventListener('click', () => {
  sandboxEl('intro-title').focus({ preventScroll: true });
  sandboxEl('intro-title').scrollIntoView({ behavior: 'smooth', block: 'start' });
});
sandboxEl('sandbox-close').addEventListener('click', () => sandboxEl('sandbox-dialog').close());
sandboxEl('sandbox-done').addEventListener('click', () => sandboxEl('sandbox-dialog').close());
sandboxEl('sandbox-reset').addEventListener('click', () => setSandboxPreset('bounty'));
sandboxEl('sandbox-export').addEventListener('click', exportSandbox);
sandboxFields.forEach(field => {
  const input = sandboxEl(`sandbox-${field}`);
  input.addEventListener('input', () => {
    if (!['progress', 'sponsor-progress', 'paid', 'expired'].includes(field)) sandboxMarkCustom();
    renderSandbox();
  });
  if (input.type === 'number') input.addEventListener('change', () => { input.value = sandboxInt(field, Number(input.min), Number(input.max)); renderSandbox(); });
});
document.querySelectorAll('[data-sandbox-preset]').forEach(button => button.addEventListener('click', () => setSandboxPreset(button.dataset.sandboxPreset)));
setSandboxPreset('bounty');

const usecasesDialog = sandboxEl('usecases-dialog');
sandboxEl('usecases-open').addEventListener('click', () => usecasesDialog.showModal());
sandboxEl('usecases-close').addEventListener('click', () => usecasesDialog.close());
sandboxEl('usecases-sandbox').addEventListener('click', () => {
  usecasesDialog.close();
  sandboxEl('sandbox-dialog').showModal();
});
function showUsecase(name) {
  document.querySelectorAll('[data-usecase]').forEach(option => option.setAttribute('aria-pressed', String(option.dataset.usecase === name)));
  document.querySelectorAll('.usecase').forEach(article => { article.hidden = article.id !== `usecase-${name}`; });
}
document.querySelectorAll('[data-usecase]').forEach(button => button.addEventListener('click', () => showUsecase(button.dataset.usecase)));
document.querySelectorAll('[data-open-usecase]').forEach(button => button.addEventListener('click', () => {
  showUsecase(button.dataset.openUsecase);
  usecasesDialog.showModal();
}));
