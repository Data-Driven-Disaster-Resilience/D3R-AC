import { expect } from "chai";
import { ethers, deploy } from "./helpers.mjs";

/// Randomized invariant (stateful property) tests for DisbursementController.
///
/// A seeded PRNG drives long random sequences of create / attest / release / cancel / fund
/// calls -- including deliberately invalid ones -- against an independent in-memory model of
/// the contract's rules. After EVERY step, on-chain state must equal the model and the
/// accounting invariants below must hold. Seeds are fixed, so any failure is reproducible
/// (the failing seed and step are in the message).
///
/// Invariants (docs/audit-engagement-package.md section 3.5: no fuzz/invariant testing existed):
///   I1  token.balanceOf(controller) == total funded - total released          (no funds appear/vanish)
///   I2  token.balanceOf(recipient)  == total released                          (payouts land, exactly once)
///   I3  commitment.releasedAmount == sum of released milestone amounts <= totalAmount
///   I4  every milestone's attested/released flags match the model; released implies attested
///   I5  a cancelled commitment never releases anything further
///   I6  a call the model says must revert does revert, and leaves state unchanged

function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const SEEDS = [1, 7, 42, 2026, 90210];
const STEPS = 120;

describe("DisbursementController -- randomized invariants", function () {
  this.timeout(180_000);

  for (const seed of SEEDS) {
    it(`holds all invariants over ${STEPS} random steps (seed ${seed})`, async function () {
      const rand = mulberry32(seed);
      const int = (lo, hi) => lo + Math.floor(rand() * (hi - lo + 1));

      const [admin, recipient] = await ethers.getSigners();
      const token = await deploy("D3RACToken", admin, 10_000_000, admin.address);
      const registry = await deploy("IdentityRegistry", admin, admin.address);
      const controller = await deploy("DisbursementController", admin, await registry.getAddress(), admin.address);
      await registry.verifyRecipient(recipient.address, "Invariant Test Community");
      const tokenAddr = await token.getAddress();
      const controllerAddr = await controller.getAddress();

      // ---- independent model ----
      let funded = 0n;
      let released = 0n;
      const model = []; // { active, milestones: [{ amount, attested, released }] }

      const attempt = async (fn) => {
        try {
          const tx = await fn();
          await tx.wait();
          return true;
        } catch {
          return false;
        }
      };
      const where = (step, op) => `seed ${seed}, step ${step}, op ${op}`;

      async function checkInvariants(step, op) {
        const ctx = where(step, op);
        expect(await token.balanceOf(controllerAddr), `I1 ${ctx}`).to.equal(funded - released);
        expect(await token.balanceOf(recipient.address), `I2 ${ctx}`).to.equal(released);
        expect(await controller.commitmentCount(), `count ${ctx}`).to.equal(BigInt(model.length));
        for (let id = 0; id < model.length; id++) {
          const m = model[id];
          const c = await controller.getCommitment(id);
          const modelReleased = m.milestones.filter((x) => x.released).reduce((s, x) => s + x.amount, 0n);
          const total = m.milestones.reduce((s, x) => s + x.amount, 0n);
          expect(c.releasedAmount, `I3 released ${ctx} c${id}`).to.equal(modelReleased);
          expect(c.totalAmount, `I3 total ${ctx} c${id}`).to.equal(total);
          expect(c.releasedAmount <= c.totalAmount, `I3 bound ${ctx} c${id}`).to.equal(true);
          expect(c.active, `I5 active ${ctx} c${id}`).to.equal(m.active);
          for (let i = 0; i < m.milestones.length; i++) {
            const ms = await controller.getMilestone(id, i);
            expect(ms.attested, `I4 attested ${ctx} c${id}.${i}`).to.equal(m.milestones[i].attested);
            expect(ms.released, `I4 released ${ctx} c${id}.${i}`).to.equal(m.milestones[i].released);
            if (ms.released) expect(ms.attested, `I4 released=>attested ${ctx} c${id}.${i}`).to.equal(true);
          }
        }
      }

      for (let step = 0; step < STEPS; step++) {
        const roll = rand();

        if (roll < 0.12 || model.length === 0) {
          // create a commitment (always valid: verified recipient, non-empty, positive amounts)
          const n = int(1, 4);
          const amounts = Array.from({ length: n }, () => BigInt(int(1, 500)));
          const descriptions = amounts.map((_, i) => `milestone ${i}`);
          const ok = await attempt(() =>
            controller.createCommitment(recipient.address, tokenAddr, "Invariant Test Community", descriptions, amounts)
          );
          expect(ok, `create should succeed (${where(step, "create")})`).to.equal(true);
          model.push({ active: true, milestones: amounts.map((a) => ({ amount: a, attested: false, released: false })) });
          await checkInvariants(step, "create");
          continue;
        }

        if (roll < 0.27) {
          // fund the controller with a random amount
          const amt = BigInt(int(1, 800));
          const ok = await attempt(() => token.transfer(controllerAddr, amt));
          expect(ok, `fund should succeed (${where(step, "fund")})`).to.equal(true);
          funded += amt;
          await checkInvariants(step, "fund");
          continue;
        }

        const id = rand() < 0.05 ? model.length + int(0, 3) : int(0, model.length - 1); // sometimes out of range
        const known = id < model.length;
        const idx = known && rand() >= 0.05 ? int(0, model[id].milestones.length - 1) : int(0, 9);

        if (roll < 0.52) {
          const mm = known ? model[id].milestones[idx] : undefined;
          const shouldWork = known && model[id].active && mm !== undefined && !mm.attested;
          const ok = await attempt(() => controller.attestMilestone(id, idx));
          expect(ok, `I6 attest outcome (${where(step, `attest c${id}.${idx}`)})`).to.equal(shouldWork);
          if (shouldWork) mm.attested = true;
          await checkInvariants(step, `attest c${id}.${idx}`);
        } else if (roll < 0.9) {
          const mm = known ? model[id].milestones[idx] : undefined;
          const poolBalance = funded - released;
          const shouldWork =
            known && model[id].active && mm !== undefined && mm.attested && !mm.released && poolBalance >= mm.amount;
          const ok = await attempt(() => controller.releaseMilestone(id, idx));
          expect(ok, `I6 release outcome (${where(step, `release c${id}.${idx}`)})`).to.equal(shouldWork);
          if (shouldWork) {
            mm.released = true;
            released += mm.amount;
          }
          await checkInvariants(step, `release c${id}.${idx}`);
        } else {
          const shouldWork = known && model[id].active;
          const ok = await attempt(() => controller.cancelCommitment(id));
          expect(ok, `I6 cancel outcome (${where(step, `cancel c${id}`)})`).to.equal(shouldWork);
          if (shouldWork) model[id].active = false;
          await checkInvariants(step, `cancel c${id}`);
        }
      }
    });
  }
});
