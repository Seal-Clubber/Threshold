const sandboxEl = id => document.getElementById(id);
const sandboxFields = ['budget', 'share', 'upfront', 'stages', 'pledges', 'reviewers', 'quorum', 'funding-epochs', 'stage-epochs'];
const sandboxPresets = {
  bounty: [100, 60, 10, 2, 4, 2, 2, 20, 20],
  tool: [250, 70, 15, 3, 20, 5, 3, 30, 25],
  many: [1000, 60, 10, 3, 101, 7, 5, 40, 30],
};
let sandboxWeights = [1];

function sandboxInt(name, min, max) {
  const value = Number(sandboxEl(`sandbox-${name}`).value);
  return Number.isFinite(value) ? Math.min(max, Math.max(min, Math.trunc(value))) : min;
}
function sandboxAmount(micro) {
  return `${(micro / 1_000_000).toLocaleString(undefined, { maximumFractionDigits: 2 })} tTARI`;
}
function sandboxApportion(total, weights) {
  const weightTotal = weights.reduce((sum, weight) => sum + weight, 0);
  const raw = weights.map(weight => total * weight / weightTotal);
  const values = raw.map(Math.floor);
  let left = total - values.reduce((sum, value) => sum + value, 0);
  const order = raw.map((value, index) => ({ index, remainder: value - values[index] })).sort((a, b) => b.remainder - a.remainder || a.index - b.index);
  for (let i = 0; i < left; i++) values[order[i].index]++;
  return values;
}
function renderSandboxWeights(count) {
  const box = sandboxEl('sandbox-weights');
  if (box.children.length === count) return;
  sandboxWeights = Array.from({ length: count }, (_, index) => sandboxWeights[index] || 1);
  box.replaceChildren();
  sandboxWeights.forEach((weight, index) => {
    const label = document.createElement('label');
    label.className = 'sandbox-field';
    const title = document.createElement('span');
    title.textContent = `Stage ${index + 2} weight`;
    const input = document.createElement('input');
    input.type = 'number'; input.min = '1'; input.max = '100'; input.value = String(weight);
    input.setAttribute('aria-label', `Reviewed stage ${index + 2} relative weight`);
    input.addEventListener('input', () => {
      sandboxWeights[index] = Math.max(1, Math.min(100, Math.trunc(Number(input.value) || 1)));
      renderSandbox();
    });
    label.append(title, input);
    box.append(label);
  });
}
function renderSandbox() {
  const budget = sandboxInt('budget', 1, 100000);
  const share = sandboxInt('share', 1, 99);
  const upfront = sandboxInt('upfront', 1, 60);
  const stages = sandboxInt('stages', 2, 12);
  const pledges = sandboxInt('pledges', 2, 200);
  const reviewers = sandboxInt('reviewers', 1, 20);
  const quorum = sandboxInt('quorum', 1, 20);
  const fundingEpochs = sandboxInt('funding-epochs', 1, 10000);
  const stageEpochs = sandboxInt('stage-epochs', 1, 10000);
  const progress = sandboxInt('progress', 0, 100);
  renderSandboxWeights(stages - 1);

  const budgetMicro = budget * 1_000_000;
  const community = Math.round(budgetMicro * share / 100);
  const sponsor = budgetMicro - community;
  const laterWeight = sandboxWeights.reduce((sum, value) => sum + value, 0);
  const weights = [upfront * laterWeight, ...sandboxWeights.map(value => (100 - upfront) * value)];
  const communityStages = sandboxApportion(community, weights);
  const sponsorStages = sandboxApportion(sponsor, weights);
  const issues = [];
  if (pledges > 32) issues.push(`${pledges} pledge records exceed this template’s 32-record cap`);
  if (reviewers > 5) issues.push(`${reviewers} reviewer keys exceed its 5-key cap`);
  if (stages > 8) issues.push(`${stages} stages exceed its 8-stage cap`);
  if (quorum > reviewers) issues.push('votes needed cannot exceed reviewer keys');
  if (communityStages.some(value => value <= 0) || sponsorStages.some(value => value <= 0)) issues.push('each stage needs a positive amount from both roles');

  sandboxEl('sandbox-share-value').textContent = `${share}%`;
  sandboxEl('sandbox-upfront-value').textContent = `${upfront}%`;
  sandboxEl('sandbox-progress-value').textContent = `${progress}% pledged`;
  sandboxEl('sandbox-community-result').textContent = sandboxAmount(community);
  sandboxEl('sandbox-sponsor-result').textContent = sandboxAmount(sponsor);
  sandboxEl('sandbox-first-result').textContent = sandboxAmount(communityStages[0] + sponsorStages[0]);
  const viability = sandboxEl('sandbox-viability');
  viability.classList.toggle('invalid', issues.length > 0);
  viability.textContent = issues.length ? `Terms need changes: ${issues.join('; ')}. Changing this page cannot lift an on-chain cap.` : 'Within published v1 count limits · illustrative terms, not deployed';
  const gate = sandboxEl('sandbox-gate');
  gate.classList.toggle('open', progress === 100 && issues.length === 0);
  gate.textContent = progress < 100 ? `Locked: ${100 - progress}% of the bundle is still missing. No upfront payment.`
    : issues.length ? 'Funding appears complete, but these terms cannot be used with the published template.'
      : `Full funding still needs exact community and sponsor cells. If they match, ${sandboxAmount(communityStages[0] + sponsorStages[0])} can be released upfront.`;
  const schedule = sandboxEl('sandbox-schedule');
  schedule.replaceChildren(...communityStages.map((communityPart, index) => {
    const sponsorPart = sponsorStages[index];
    const row = document.createElement('div'); row.className = 'sandbox-stage';
    const head = document.createElement('div'); head.className = 'sandbox-stage-head';
    const title = document.createElement('span'); title.textContent = index === 0 ? '01 · Upfront, after full match' : `${String(index + 1).padStart(2, '0')} · Reviewer-gated`;
    const value = document.createElement('strong'); value.textContent = sandboxAmount(communityPart + sponsorPart);
    head.append(title, value);
    const detail = document.createElement('small');
    detail.textContent = `Community ${sandboxAmount(communityPart)} · Sponsor ${sandboxAmount(sponsorPart)} · due by creation epoch + ${fundingEpochs + stageEpochs * (index + 1)}`;
    row.append(head, detail);
    return row;
  }));
}
function setSandboxPreset(name) {
  const values = sandboxPresets[name];
  sandboxFields.forEach((field, index) => { sandboxEl(`sandbox-${field}`).value = values[index]; });
  sandboxWeights = name === 'bounty' ? [1] : [2, 1];
  sandboxEl('sandbox-weights').replaceChildren();
  renderSandbox();
}

sandboxEl('sandbox-open').addEventListener('click', () => sandboxEl('sandbox-dialog').showModal());
sandboxEl('sandbox-close').addEventListener('click', () => sandboxEl('sandbox-dialog').close());
sandboxEl('sandbox-done').addEventListener('click', () => sandboxEl('sandbox-dialog').close());
sandboxEl('sandbox-reset').addEventListener('click', () => setSandboxPreset('bounty'));
sandboxFields.forEach(field => sandboxEl(`sandbox-${field}`).addEventListener('input', renderSandbox));
sandboxEl('sandbox-progress').addEventListener('input', renderSandbox);
document.querySelectorAll('[data-sandbox-preset]').forEach(button => button.addEventListener('click', () => setSandboxPreset(button.dataset.sandboxPreset)));
renderSandbox();
