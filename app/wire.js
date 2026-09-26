const $ = id => document.getElementById(id);
const exampleTerms = { budget: 100, communityShare: 60, upfront: 10, quorum: 2 };
const terms = { ...exampleTerms };
const amount = cents => (cents / 100).toLocaleString(undefined, { maximumFractionDigits: 2 });
function apportion(total, weights) {
  const sum = weights.reduce((value, weight) => value + weight, 0);
  const shares = weights.map(weight => total * weight / sum);
  const values = shares.map(Math.floor);
  let remainder = total - values.reduce((value, share) => value + share, 0);
  const order = shares.map((share, index) => ({ index, fraction: share - values[index] })).sort((a, b) => b.fraction - a.fraction || a.index - b.index);
  for (let i = 0; i < remainder; i++) values[order[i].index]++;
  return values;
}
function calculatePlan(config) {
  const budgetCents = config.budget * 100;
  const communityCents = Math.round(budgetCents * config.communityShare / 100);
  const sponsorCents = budgetCents - communityCents;
  const startTarget = Math.round(budgetCents * config.upfront / 100);
  const [communityStart, sponsorStart] = apportion(startTarget, [communityCents, sponsorCents]);
  const communityShares = apportion(communityCents, [1, 1, 1]);
  const starts = apportion(communityStart, communityShares);
  const ownerAmounts = [...communityShares, sponsorCents].map((totalCents, index) => {
    const upfrontCents = index === 3 ? sponsorStart : starts[index];
    return { totalCents, upfrontCents, laterCents: totalCents - upfrontCents };
  });
  const startCents = communityStart + sponsorStart;
  return { budgetCents, communityCents, sponsorCents, startCents, laterCents: budgetCents - startCents, ownerAmounts };
}
const plan = calculatePlan(terms);
const choices = {
  alice: false, bob: false, carol: false, sponsor: false,
  activate: false, upfront: false, developer: false,
  reviewer1: false, reviewer2: false, recipient: false, final: false
};
const owners = [
  { id: 'alice', name: 'Alice', ...plan.ownerAmounts[0] },
  { id: 'bob', name: 'Bob', ...plan.ownerAmounts[1] },
  { id: 'carol', name: 'Carol', ...plan.ownerAmounts[2] },
  { id: 'sponsor', name: 'Sponsor', ...plan.ownerAmounts[3] }
];
const nodes = [
  { title: 'Agree on the terms', text: 'Set the budget, payments, reviewers, and deadlines before funding starts.', rule: 'Roles, recipient, stage amounts, reviewer keys and deadlines are fixed before anyone pledges.' },
  { title: 'Alice · 20 tTARI', text: 'Locks 2 for the start and 18 for the reviewed stage.', actor: 'alice', on: 'Sends her pledge', off: 'Does not send', rule: 'Only Alice can later recover her own unpaid cells.' },
  { title: 'Bob · 20 tTARI', text: 'Locks a separate pair of private cells.', actor: 'bob', on: 'Sends his pledge', off: 'Does not send', rule: 'A public campaign target does not reveal each private output’s opening.' },
  { title: 'Carol · 20 tTARI', text: 'Supplies the final share of community funding.', actor: 'carol', on: 'Sends her pledge', off: 'Does not send', rule: 'The coordinator may learn an opening if Carol shares it for matching.' },
  { title: 'Sponsor · 40 tTARI', text: 'Locks 4 for the start and 36 for the reviewed stage.', actor: 'sponsor', on: 'Sends the match', off: 'Does not send', rule: 'The component checks exact totals for each role and stage; sponsor funds cannot replace a missing community pledge.' },
  { title: 'Start the funded campaign', text: 'All four pledges must be present. One transaction activates the whole campaign, or none of it.', actor: 'activate', on: 'Submit activation', off: 'Not submitted', rule: 'An incomplete bundle rejects; the first payment cannot be released on partial funding.', branch: 'funding', side: 'left' },
  { title: 'Pay 10 to start', text: 'A separate release sends the agreed 10 tTARI only after activation. The remaining 90 stays locked.', actor: 'upfront', on: 'Release 10 upfront', off: 'Not released yet', rule: 'This example is 10/90. The existing testnet campaign has immutable 20% upfront terms.' },
  { title: 'Submit the finished work', text: 'The developer delivers the feature before the deadline. The start payment cannot be taken back.', actor: 'developer', on: 'Submits the work', off: 'Does not deliver', rule: 'Already paid funds cannot be clawed back; only unpaid cells can be refunded.', branch: 'delivery', side: 'left' },
  { title: 'Reviewer one', text: 'Checks whether the feature meets the agreed requirements.', actor: 'reviewer1', on: 'Approves', off: 'Does not approve', rule: 'Reviewer keys are named in the frozen terms; pledge size grants no reviewer votes.' },
  { title: 'Reviewer two', text: 'A second named reviewer decides. This bounty requires both approvals for the later payout.', actor: 'reviewer2', on: 'Approves', off: 'Does not approve', rule: 'The chain verifies signatures and quorum, not whether the code is good.', branch: 'review', side: 'right' },
  { title: 'Check the payment', text: 'The developer’s wallet checks that it can decrypt the payment before signing. This check happens in the wallet, not on-chain.', actor: 'recipient', on: 'Wallet check passes', off: 'Wallet check fails', rule: 'The component requires the frozen recipient’s signature but cannot prove output ciphertext decryptability.', branch: 'recipient', side: 'right' },
  { title: 'Developer receives 100', text: 'A completed bounty pays 10 after full funding and 90 after timely reviewer quorum. This is the successful route through the diagram.', actor: 'final', on: 'Release the final 90', off: 'Final 90 not released', rule: 'The real chain guarantees only its encoded rules. Delivery quality and client-side decryption checks have the limits shown above.' }
];
const branchPreviews = {
  funding: { tag: 'IF FUNDING FALLS SHORT', title: 'A pledge is missing', text: 'No payment can be made. Contributors can reclaim funds after the funding deadline.' },
  delivery: { tag: 'IF WORK IS NOT DELIVERED', title: 'Work stops', text: 'The start payment has been paid. Unpaid funds can be reclaimed after the deadline.' },
  review: { tag: 'IF REVIEWERS DO NOT APPROVE', title: 'Approval is missing', text: 'The final payment needs both reviewers to approve before the deadline.' },
  recipient: { tag: 'IF THE WALLET CHECK FAILS', title: 'Payment needs fixing', text: 'The wallet should stop the payment. The contract cannot perform this check.' }
};
function updateCopy() {
  nodes[0].text = `Community: ${amount(plan.communityCents)} tTARI. Sponsor: ${amount(plan.sponsorCents)} tTARI. Agree on the feature, two reviewers, and deadlines before anyone pledges.`;
  owners.forEach((owner, index) => {
    nodes[index + 1].title = `${owner.name} · ${amount(owner.totalCents)} tTARI`;
    nodes[index + 1].text = `${amount(owner.upfrontCents)} goes toward the start payment; ${amount(owner.laterCents)} waits for approval.`;
  });
  nodes[6].title = `Pay ${amount(plan.startCents)} to start`;
  nodes[6].text = `After full funding and activation, release ${amount(plan.startCents)} tTARI. The remaining ${amount(plan.laterCents)} stays locked.`;
  nodes[6].on = `Release ${amount(plan.startCents)} upfront`;
  nodes[6].rule = `This hypothetical uses ${terms.upfront}% upfront. The existing testnet campaign has immutable 20% upfront terms.`;
  nodes[9].text = 'Makes an independent decision on the same work.';
  nodes[11].title = `Developer receives ${amount(plan.budgetCents)}`;
  nodes[11].on = `Release the final ${amount(plan.laterCents)}`;
  nodes[11].off = `Final ${amount(plan.laterCents)} not released`;
  branchPreviews.delivery.text = `${amount(plan.startCents)} has already been paid. After the milestone deadline, owners can recover their separate unpaid shares.`;
  branchPreviews.review.text = `The remaining ${amount(plan.laterCents)} cannot be paid without ${terms.quorum} named approval${terms.quorum === 1 ? '' : 's'} before the deadline.`;
  nodes.forEach((node, index) => {
    const card = $(`node-${index}`);
    card.setAttribute('aria-label', `Step ${index + 1}: ${node.title}`);
    card.querySelector('h2').textContent = node.title;
    card.querySelector(':scope > p').textContent = node.text;
    card.querySelector('.rule span').textContent = node.rule;
  });
  const review = document.querySelector('.review-stage');
  review.querySelector('.parallel-heading h2').textContent = 'Review the work';
  review.querySelector('.parallel-heading p').textContent = `The final ${amount(plan.laterCents)} tTARI needs ${terms.quorum} of 2 approvals. Reviewers judge the work; the contract checks their signatures.`;
  review.querySelector('.parallel-merge').textContent = `${terms.quorum} APPROVAL${terms.quorum === 1 ? '' : 'S'} REQUIRED ↓`;
}
const refunds = new Set();
const deadlines = { funding: false, delivery: false, review: false, recipient: false };
const timeline = [0, 1, 5, 6, 7, 8, 10, 11];
function stepElement(index) {
  return index === 1 ? document.querySelector('.funding-stage')
    : index === 8 ? document.querySelector('.review-stage') : $(`row-${index}`);
}
function visibleTimeline() {
  return timeline.filter(index => !stepElement(index).hidden);
}
let focusIndex = 0;
let highlightIndex = 0;
let currentBranch = null;
let lastScrollFrame = 0;

function switchMarkup(node) {
  return `<div class="choice-row"><span class="choice-copy" id="label-${node.actor}"></span><label class="switch"><input type="checkbox" role="switch" data-choice="${node.actor}" aria-labelledby="label-${node.actor}"><span class="switch-track" aria-hidden="true"></span></label></div>`;
}
function nodeMarkup(index) {
  const node = nodes[index];
  return `<section class="node ${index === 11 ? 'success-node' : ''}" id="node-${index}" aria-label="Step ${index + 1}: ${node.title}"><div class="node-head"><span class="node-index">${String(index + 1).padStart(2, '0')} / 12</span><span class="state-pill" id="pill-${index}"></span></div><h2>${node.title}</h2><p>${node.text}</p>${node.actor ? switchMarkup(node) : ''}<details class="rule"><summary>Rules and limits</summary><span>${node.rule}</span></details></section>`;
}
function branchMarkup(kind, side) {
  const branch = branchPreviews[kind];
  return `<aside class="branch-card ${side}" id="branch-${kind}" aria-live="polite"><span class="branch-tag">${branch.tag}</span><h3>${branch.title}</h3><p>${branch.text}</p><div class="branch-extra" id="branch-extra-${kind}"></div></aside>`;
}
function buildDiagram() {
  const markup = [];
  for (let index = 0; index < nodes.length; index++) {
    if (index === 1) {
      markup.push(`<section class="parallel-stage funding-stage" aria-label="Four independent pledges"><div class="parallel-heading"><span>FUNDING</span><h2>Collect the pledges</h2><p>Three community members and one sponsor fund this bounty. Turn on all four pledges to meet the goal.</p></div><div class="funding-groups"><section class="funding-group community-group" aria-labelledby="community-group-title"><h3 id="community-group-title">Community <span>60 tTARI</span></h3><div class="parallel-grid community-grid">${[1, 2, 3].map(i => `<div class="parallel-cell" id="row-${i}">${nodeMarkup(i)}</div>`).join('')}</div></section><section class="funding-group sponsor-group" aria-labelledby="sponsor-group-title"><h3 id="sponsor-group-title">Sponsor <span>40 tTARI</span></h3><div class="parallel-grid sponsor-grid"><div class="parallel-cell" id="row-4">${nodeMarkup(4)}</div></div></section></div><div class="parallel-merge">COMMUNITY + SPONSOR REQUIRED ↓</div></section>`);
      index = 4;
      continue;
    }
    if (index === 8) {
      markup.push(`<section class="parallel-stage review-stage" aria-label="Two independent reviewer decisions"><div class="parallel-heading"><span>REVIEW GATE</span><h2>Two reviewers. One quorum.</h2><p>Both named reviewers must approve the same delivery before the final 90 can be paid.</p></div><div class="parallel-grid two">${[8, 9].map(i => `<div class="parallel-cell" id="row-${i}">${nodeMarkup(i)}</div>`).join('')}</div><div class="parallel-branch-wrap">${branchMarkup('review', 'right')}</div><div class="parallel-merge">BOTH APPROVALS REQUIRED ↓</div></section>`);
      index = 9;
      continue;
    }
    const node = nodes[index];
    markup.push(`<div class="flow-row" id="row-${index}">${nodeMarkup(index)}${node.branch ? branchMarkup(node.branch, node.side) : ''}</div>`);
  }
  $('flow').innerHTML = markup.join('');
  $('flow').addEventListener('change', event => {
    const input = event.target;
    if (!(input instanceof HTMLInputElement)) return;
    if (input.dataset.deadline) {
      deadlines[input.dataset.deadline] = input.checked;
      refunds.clear();
      render();
      return;
    }
    if (!input.dataset.choice) return;
    choices[input.dataset.choice] = input.checked;
    refunds.clear();
    for (const kind of Object.keys(deadlines)) deadlines[kind] = false;
    highlightIndex = nodes.findIndex(node => node.actor === input.dataset.choice);
    focusIndex = highlightIndex >= 1 && highlightIndex <= 4 ? 1 : highlightIndex === 9 ? 8 : highlightIndex;
    render();
  });
  $('flow').addEventListener('click', event => {
    const button = event.target.closest('button[data-refund],button[data-jump]');
    if (!button) return;
    if (button.dataset.refund) { refunds.add(button.dataset.refund); render(); }
    if (button.dataset.jump) { focusIndex = Number(button.dataset.jump); scrollToRow(focusIndex); }
  });
}
function setNode(index, state, label) {
  const node = $(`node-${index}`);
  node.classList.toggle('reached', state === 'reached');
  node.classList.toggle('failed', state === 'failed');
  node.classList.toggle('pending', state === 'pending');
  $('pill-' + index).textContent = label;
}
function branchRefunds(kind) {
  const pledgers = owners.filter(owner => choices[owner.id]);
  const amountKey = kind === 'funding' ? 'totalCents' : 'laterCents';
  const label = kind === 'funding' ? 'Funding deadline passed' : 'Milestone deadline passed';
  const deadlineSwitch = `<div class="choice-row deadline-choice"><span class="choice-copy">${label}<small>Flip on to open owner refunds</small></span><label class="switch"><input type="checkbox" role="switch" data-deadline="${kind}" aria-label="${label}" ${deadlines[kind] ? 'checked' : ''}><span class="switch-track" aria-hidden="true"></span></label></div>`;
  if (!deadlines[kind]) return deadlineSwitch + '<p class="branch-foot">Owner refunds remain locked until this deadline passes.</p>';
  if (!pledgers.length) return deadlineSwitch + '<p class="branch-foot">Nobody pledged, so there is nothing to refund.</p>';
  return deadlineSwitch + `<p class="branch-foot">Each owner can now claim their own unpaid cells, in any order:</p><div class="refund-list">${pledgers.map(owner => `<button type="button" data-refund="${owner.id}" class="${refunds.has(owner.id) ? 'refunded' : ''}" ${refunds.has(owner.id) ? 'disabled' : ''}>${refunds.has(owner.id) ? '✓' : '↗'} ${owner.name} ${refunds.has(owner.id) ? 'recovered' : 'claims'} ${amount(owner[amountKey])} tTARI</button>`).join('')}</div><p class="branch-foot">${refunds.size} of ${pledgers.length} owners have reclaimed funds in this simulation. No coordinator signature is needed.</p>`;
}
function renderBranches(data) {
  for (const kind of Object.keys(branchPreviews)) {
    const active = currentBranch === kind;
    const card = $('branch-' + kind);
    card.classList.toggle('active', active);
    card.hidden = !active;
    const wrapper = card.closest('.parallel-branch-wrap');
    if (wrapper) wrapper.hidden = !active;
    const extra = $('branch-extra-' + kind);
    if (!active) {
      card.querySelector('h3').textContent = branchPreviews[kind].title;
      card.querySelector('p').textContent = branchPreviews[kind].text;
      extra.replaceChildren();
      continue;
    }
    if (kind === 'funding') {
      const missing = owners.filter(owner => !choices[owner.id]).map(owner => owner.name).join(', ');
      card.querySelector('h3').textContent = choices.activate ? 'Activation rejected' : 'Match incomplete';
      card.querySelector('p').textContent = `${amount(plan.budgetCents - data.totalCents)} tTARI still needed from ${missing}. ${choices.activate ? 'Activation is rejected.' : 'The campaign cannot start yet.'} No start payment is made.`;
      extra.innerHTML = branchRefunds(kind) + `<button type="button" data-jump="${nodes.findIndex(node => node.actor === owners.find(owner => !choices[owner.id]).id)}" class="return-button">← Change a missing pledge</button>`;
    } else if (kind === 'delivery') {
      card.querySelector('h3').textContent = 'Work never arrives';
      card.querySelector('p').textContent = `The developer keeps the ${amount(plan.startCents)} already paid. The unpaid ${amount(plan.laterCents)} can return to its owners only after the frozen milestone deadline.`;
      extra.innerHTML = branchRefunds(kind) + '<button type="button" data-jump="7" class="return-button">← Change developer outcome</button>';
    } else if (kind === 'review') {
      card.querySelector('h3').textContent = 'Approval is missing';
      card.querySelector('p').textContent = `${data.approvals} of ${terms.quorum} approvals received. The final ${amount(plan.laterCents)} stays locked. Contributors can reclaim their unpaid funds after the deadline.`;
      extra.innerHTML = branchRefunds(kind) + `<button type="button" data-jump="${choices.reviewer1 ? 9 : 8}" class="return-button">← Revisit a reviewer</button>`;
    } else {
      card.querySelector('h3').textContent = 'The wallet stops the payment';
      card.querySelector('p').textContent = 'The developer cannot decrypt the payment. The wallet should stop and fix it before signing. The contract cannot enforce this check. Unpaid funds become refundable after the deadline.';
      extra.innerHTML = branchRefunds(kind) + '<button type="button" data-jump="10" class="return-button">← Recheck recipient output</button>';
    }
  }
}
function render() {
  updateCopy();
  const community = owners.slice(0, 3).reduce((sum, owner) => sum + (choices[owner.id] ? owner.totalCents : 0), 0);
  const sponsor = choices.sponsor ? plan.sponsorCents : 0;
  const totalCents = community + sponsor;
  const funded = community === plan.communityCents && sponsor === plan.sponsorCents;
  const activated = funded && choices.activate;
  const startPaid = activated && choices.upfront;
  const delivered = startPaid && choices.developer;
  const approvals = Number(choices.reviewer1) + Number(choices.reviewer2);
  const approved = delivered && approvals >= terms.quorum;
  const recipientPassed = approved && choices.recipient;
  const finalPaid = recipientPassed && choices.final;
  currentBranch = !funded ? 'funding' : !startPaid ? null : !choices.developer ? 'delivery' : approvals < terms.quorum ? 'review' : !choices.recipient ? 'recipient' : null;
  $('community-total').textContent = `${amount(community)} / ${amount(plan.communityCents)}`;
  $('sponsor-total').textContent = `${amount(sponsor)} / ${amount(plan.sponsorCents)}`;
  $('paid-total').textContent = `${amount(finalPaid ? plan.budgetCents : startPaid ? plan.startCents : 0)} / ${amount(plan.budgetCents)}`;
  $('status-label').textContent = currentBranch ? { funding: 'FUNDING INCOMPLETE', delivery: 'WAITING FOR WORK', review: 'WAITING FOR APPROVAL', recipient: 'CHECK THE PAYMENT' }[currentBranch] : !activated ? 'READY TO START' : !startPaid ? 'READY FOR START PAYMENT' : !finalPaid ? 'READY FOR FINAL PAYMENT' : 'BOUNTY COMPLETE';
  $('status-label').classList.toggle('blocked', !finalPaid);
  setNode(0, 'reached', 'TERMS FROZEN');
  owners.forEach((owner, index) => {
    const on = choices[owner.id];
    setNode(index + 1, on ? 'reached' : 'pending', on ? 'PLEDGED' : 'NOT PLEDGED');
    $(`label-${owner.id}`).innerHTML = `${owner.name}: ${on ? 'pledged' : 'not pledged'}<small>${on ? `${amount(owner.totalCents)} tTARI locked for this bounty` : 'Turn on to pledge'}</small>`;
  });
  [5, 6, 7, 8, 9, 10, 11].forEach(index => {
    const node = nodes[index];
    const on = choices[node.actor];
    const note = index === 5 ? funded ? 'Full match is available' : 'All four pledges are required'
      : index === 6 ? activated ? 'Activation has passed' : 'Requires a committed activation'
      : index === 11 ? recipientPassed ? 'Payment checks passed' : 'Complete the earlier steps first'
      : index === 7 ? startPaid ? 'The start payment is complete' : 'Not reached before the start payment'
      : index === 10 ? approved ? 'Reviewers have approved' : 'Waiting for reviewer approval'
      : delivered ? (index === 8 || index === 9) && approved && !on ? 'Optional: quorum already reached' : 'Work has been submitted' : 'Not reached before delivery';
    $(`label-${node.actor}`).innerHTML = `${on ? node.on : node.off}<small>${note}</small>`;
  });
  for (const id of Object.keys(choices)) $(`flow`).querySelector(`[data-choice="${id}"]`).checked = choices[id];
  setNode(5, activated ? 'reached' : choices.activate && !funded ? 'failed' : 'pending', activated ? 'ATOMICALLY ACTIVE' : choices.activate && !funded ? 'BUNDLE REJECTED' : funded ? 'READY TO ACTIVATE' : 'MATCH INCOMPLETE');
  setNode(6, startPaid ? 'reached' : choices.upfront && !activated ? 'failed' : 'pending', startPaid ? `${amount(plan.startCents)} PAID` : choices.upfront && !activated ? 'RELEASE REJECTED' : 'NOT RELEASED');
  setNode(7, !startPaid ? 'pending' : delivered ? 'reached' : 'failed', !startPaid ? 'NOT REACHED' : delivered ? 'WORK SUBMITTED' : 'NO DELIVERY');
  setNode(8, !delivered || !choices.reviewer1 && approved ? 'pending' : choices.reviewer1 ? 'reached' : 'failed', !delivered ? 'NOT REACHED' : choices.reviewer1 ? 'APPROVED' : approved ? 'OPTIONAL' : 'NO VOTE');
  setNode(9, !delivered || !choices.reviewer2 && approved ? 'pending' : choices.reviewer2 ? 'reached' : 'failed', !delivered ? 'NOT REACHED' : choices.reviewer2 ? 'APPROVED' : approved ? 'OPTIONAL' : 'NO VOTE');
  setNode(10, !approved ? 'pending' : choices.recipient ? 'reached' : 'failed', !approved ? 'NOT REACHED' : choices.recipient ? 'CLIENT CHECK PASSED' : 'CLIENT CHECK FAILED');
  const finalLabel = finalPaid ? `${amount(plan.budgetCents)} PAID` : choices.final && !approved ? 'RELEASE REJECTED' : choices.final && !choices.recipient ? 'CLIENT STOPPED' : recipientPassed ? 'READY TO RELEASE' : 'NOT PAYABLE';
  setNode(11, finalPaid ? 'reached' : choices.final && !recipientPassed ? 'failed' : 'pending', finalLabel);
  $('node-11').querySelector('p').textContent = finalPaid
    ? `Bounty complete. The developer received ${amount(plan.startCents)} to start and ${amount(plan.laterCents)} after approval.`
    : currentBranch === 'funding' ? 'Waiting for the full budget. Turn on the missing pledges above to continue.'
    : !activated ? 'The full match is present, but activation is a separate transaction. Submit the exact bundle to continue.'
    : !startPaid ? `The bounty is active, but the ${amount(plan.startCents)} start payment is a separate release. Submit it to continue.`
    : currentBranch === 'delivery' ? `The developer keeps the ${amount(plan.startCents)} start payment. The unpaid ${amount(plan.laterCents)} can be reclaimed after the deadline.`
    : currentBranch === 'review' ? `The final ${amount(plan.laterCents)} needs ${terms.quorum} approvals. Without them, contributors can reclaim it after the deadline.`
    : currentBranch === 'recipient' ? 'The client stopped the final submission because it could not verify decryption. Correct the output before submitting.'
    : `Ready to pay. The developer signs the final ${amount(plan.laterCents)} tTARI release transaction.`;
  renderBranches({ totalCents, approvals });
  $('node-5').hidden = !funded;
  $('row-5').classList.toggle('branch-only', !funded);
  $('row-6').hidden = !activated;
  $('row-7').hidden = !startPaid;
  document.querySelector('.review-stage').hidden = !delivered;
  $('row-10').hidden = !approved;
  $('row-11').hidden = !recipientPassed;
  const availableSteps = visibleTimeline();
  if (!availableSteps.includes(focusIndex)) {
    focusIndex = availableSteps.at(-1);
    highlightIndex = focusIndex;
  }
  document.querySelectorAll('.node').forEach(node => node.classList.remove('headline'));
  $(`node-${highlightIndex}`).classList.add('headline');
  document.querySelector('.funding-stage').classList.toggle('focused', focusIndex === 1);
  document.querySelector('.review-stage').classList.toggle('focused', focusIndex === 8);
  $('back').disabled = focusIndex === 0;
  $('next').disabled = focusIndex === availableSteps.at(-1);
}
function scrollToRow(index, showBranch = false) {
  const availableSteps = visibleTimeline();
  const requestedStep = index >= 1 && index <= 4 ? 1 : index === 9 ? 8 : index;
  highlightIndex = availableSteps.includes(requestedStep) ? index : availableSteps.at(-1);
  focusIndex = highlightIndex >= 1 && highlightIndex <= 4 ? 1 : highlightIndex === 9 ? 8 : highlightIndex;
  render();
  const target = showBranch && currentBranch ? $('branch-' + currentBranch)
    : focusIndex === 1 ? document.querySelector('.funding-stage .parallel-heading')
    : focusIndex === 8 ? document.querySelector('.review-stage .parallel-heading')
    : stepElement(focusIndex);
  target.scrollIntoView({ behavior: 'smooth', block: 'center' });
}
function setTheme(theme) {
  document.documentElement.dataset.theme = theme;
  $('theme-label').textContent = theme === 'dark' ? 'Light' : 'Dark';
  $('theme-toggle').setAttribute('aria-label', theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode');
  $('theme-toggle').setAttribute('aria-pressed', String(theme === 'dark'));
  document.querySelector('meta[name="theme-color"]').content = theme === 'dark' ? '#081522' : '#f3f8fc';
  try { localStorage.setItem('threshold-demo-theme', theme); } catch {}
}
buildDiagram();
try { setTheme(localStorage.getItem('threshold-demo-theme') === 'light' ? 'light' : 'dark'); }
catch { setTheme('dark'); }
render();
$('theme-toggle').addEventListener('click', () => setTheme(document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark'));
$('back').addEventListener('click', () => {
  const steps = visibleTimeline();
  scrollToRow(steps[Math.max(0, steps.indexOf(focusIndex) - 1)]);
});
$('next').addEventListener('click', () => {
  const steps = visibleTimeline();
  scrollToRow(steps[Math.min(steps.length - 1, steps.indexOf(focusIndex) + 1)]);
});
$('flow-scroll').addEventListener('scroll', () => {
  cancelAnimationFrame(lastScrollFrame);
  lastScrollFrame = requestAnimationFrame(() => {
    const viewport = $('flow-scroll').getBoundingClientRect();
    const targetY = viewport.top + viewport.height * .42;
    let nearest = 0, distance = Infinity;
    visibleTimeline().forEach(index => {
      const element = stepElement(index);
      const rect = element.getBoundingClientRect();
      const delta = targetY >= rect.top && targetY <= rect.bottom ? 0 : Math.min(Math.abs(rect.top - targetY), Math.abs(rect.bottom - targetY));
      if (delta < distance) { distance = delta; nearest = index; }
    });
    if (nearest !== focusIndex) { focusIndex = nearest; highlightIndex = nearest; render(); }
  });
}, { passive: true });
