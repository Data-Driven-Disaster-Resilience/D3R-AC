/**
 * contract-trigger: subscribes to response.plan and calls into the D3R-AC smart
 * contract layer (contracts/tron, contracts/casper) to request milestone-based fund
 * release. This is intentionally a thin adapter -- it does not reimplement contract
 * logic, it only calls it, so it stays in sync with whatever ABI/addresses the
 * contracts/ folder deploys.
 *
 * TODO once contracts are deployed:
 *   1. npm install tronweb (TRON) and casper-js-sdk (Casper) in node/package.json
 *   2. Load the deployed contract address + ABI from contracts/tron and contracts/casper
 *   3. Replace `requestTronRelease` / `requestCasperRelease` bodies with real calls
 *   4. Store returned tx hashes on the fund.release.requested event for auditability
 */
import { getBus } from '../bus/messageBus';
import { ResponsePlan } from './coordinationAgent';
import { AGENTS } from '../config/generatedManifest';

interface FundReleaseRequest {
  community_id: string;
  chain: 'tron' | 'casper';
  milestone: string;
  release_pct: number;
  tx_hash: string | null;
  status: 'stubbed' | 'submitted' | 'failed';
}

async function requestTronRelease(communityId: string, milestone: string, pct: number): Promise<FundReleaseRequest> {
  // Stub -- see TODO above. contracts/tron holds the token/identity/disbursement contracts.
  return { community_id: communityId, chain: 'tron', milestone, release_pct: pct, tx_hash: null, status: 'stubbed' };
}

async function requestCasperRelease(communityId: string, milestone: string, pct: number): Promise<FundReleaseRequest> {
  // Stub -- Casper adapter is pending contract deployment per the main D3R-AC README.
  return { community_id: communityId, chain: 'casper', milestone, release_pct: pct, tx_hash: null, status: 'stubbed' };
}

export class ContractTriggerAgent {
  private bus = getBus();
  private tronDir = AGENTS['contract-trigger']?.config?.tron_contracts_dir ?? 'contracts/tron';
  private casperDir = AGENTS['contract-trigger']?.config?.casper_contracts_dir ?? 'contracts/casper';

  async runOnce(): Promise<FundReleaseRequest[]> {
    const planEvents = await this.bus.readAll<ResponsePlan>('response.plan');
    const requests: FundReleaseRequest[] = [];

    for (const { payload: plan } of planEvents) {
      const firstMilestone = plan.milestones[0];
      if (!firstMilestone) continue;
      const tron = await requestTronRelease(plan.community_id, firstMilestone.name, firstMilestone.release_pct);
      const casper = await requestCasperRelease(plan.community_id, firstMilestone.name, firstMilestone.release_pct);
      await this.bus.publish('fund.release.requested', tron);
      await this.bus.publish('fund.release.requested', casper);
      requests.push(tron, casper);
    }

    return requests;
  }
}

if (require.main === module) {
  console.log(`contract-trigger reading contracts from: ${new ContractTriggerAgent()['tronDir']}`);
  new ContractTriggerAgent().runOnce().then((reqs) => {
    console.log(`published ${reqs.length} fund.release.requested event(s) (stubbed -- no chain calls made)`);
  });
}
