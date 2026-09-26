import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const source = await readFile(new URL('../app/sandbox.js', import.meta.url), 'utf8');
const context = vm.createContext({});
vm.runInContext(source.split('// DOM layer:')[0], context);
const calculate = context.calculateSandbox;
const base = {
  budget: 100, share: 60, upfront: 10, mode: 'matched', pledges: 4, sponsors: 3,
  fundingEpochs: 20, stageEpochs: 20, startRecipient: 'Developer',
  milestones: [{ name: 'Delivery', recipient: 'Developer', weight: 1, reviewers: 2, quorum: 2 }],
  communityProgress: 100, sponsorProgress: 100, paid: 1, expired: false,
};
let r = calculate(base);
assert.equal(r.support, 'v1');
assert.equal(r.paid, 10_000_000);
assert.equal(r.unpaid, 90_000_000);
assert.equal(r.eligible, 0);
assert.equal(calculate({ ...base, expired: true }).eligible, 90_000_000);
r = calculate({ ...base, sponsorProgress: 99, expired: true });
assert.equal(r.funded, false);
assert.equal(r.paidCount, 0);
assert.equal(r.eligible, 99_600_000);
r = calculate({ ...base, mode: 'community', sponsorProgress: 0 });
assert.equal(r.support, 'poc');
assert.equal(r.sponsor, 0);
assert.equal(r.funded, true);
assert.equal(calculate({ ...base, mode: 'collective' }).support, 'future');
assert.equal(calculate({ ...base, pledges: 33 }).support, 'future');
assert.equal(calculate({ ...base, upfront: 0 }).schedule[0].upfront, false);
r = calculate({ ...base, upfront: 100, expired: true });
assert.equal(r.schedule.length, 1);
assert.equal(r.paid, 100_000_000);
assert.equal(r.eligible, 0);
r = calculate({ ...base, milestones: [{ ...base.milestones[0], quorum: 3 }], expired: true });
assert.ok(r.errors.length);
assert.equal(r.paid, 0);
assert.equal(r.eligible, 0);
assert.ok(calculate({ ...base, mode: 'collective', pledges: 3 }).errors.length);
// Conservation with uneven weights, all funding modes and both upfront boundaries.
for (const mode of ['matched', 'community', 'collective']) {
  for (const upfront of [0, 1, 17, 99, 100]) {
    for (const budget of [1, 101, 999999]) {
      r = calculate({ ...base, mode, upfront, budget, paid: 0,
        milestones: [1, 7, 13].map(weight => ({ ...base.milestones[0], weight })) });
      assert.equal(r.schedule.reduce((sum, s) => sum + s.community, 0), r.community);
      assert.equal(r.schedule.reduce((sum, s) => sum + s.sponsor, 0), r.sponsor);
      assert.equal(r.schedule.reduce((sum, s) => sum + s.amount, 0), budget * 1_000_000);
      assert.ok(r.schedule.every(s => Number.isSafeInteger(s.amount) && s.amount > 0));
      assert.ok(r.schedule.every((s, i) => i === 0 || s.deadline > r.schedule[i - 1].deadline));
    }
  }
}
console.log('Sandbox checks passed: conservation, funding gates, recovery, review validity and support labels. Browser simulation only; no contract qualification.');
