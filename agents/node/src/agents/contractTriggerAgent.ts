/**
 * contract-trigger: subscribes to response.plan and prepares requests into the D3R-AC
 * smart contract layer (contracts/tron, contracts/casper) for milestone-based fund release.
 *
 * SAFETY MODEL -- this agent NEVER signs or broadcasts a transaction. On TRON it does a
 * read-only readiness check against the deployed DisbursementController (is the mapped
 * commitment real?) and publishes a 'dry-run' request describing exactly what a human
 * operator / authorized attester would do next. The on-chain flow is
 * createCommitment (admin) -> attestMilestone (attester) -> releaseMilestone, so a
 * response plan alone can never move funds, and deciding whether an automated system may
 * hold the attester role is a governance decision, not something this adapter takes on.
 * Casper is not implemented yet and reports 'blocked'.
 *
 * Configuration (environment):
 *   D3RAC_TRON_FULL_NODE                e.g. https://api.shasta.trongrid.io (testnet only)
 *   D3RAC_TRON_DISBURSEMENT_CONTROLLER  deployed DisbursementController address (base58)
 *   D3RAC_COMMITMENT_MAP                JSON {"<community_id>": <commitmentId>}
 */
import { getBus } from '../bus/messageBus';
import { ResponsePlan } from './coordinationAgent';
import { AGENTS } from '../config/generatedManifest';

export interface FundReleaseRequest {
  community_id: string;
  chain: 'tron' | 'casper';
  milestone: string;
  milestone_index: number;
  release_pct: number;
  commitment_id: number | null;
  tx_hash: null; // always null: this agent never broadcasts
  status: 'dry-run' | 'blocked';
  note: string;
}

export interface TronConfig {
  fullNode?: string;
  controller?: string;
  commitmentMap: Record<string, number>;
}

const MAINNET_HOSTS = ['api.trongrid.io'];
// Zero address: TRON constant calls need an owner_address; this one holds no authority.
const READ_ONLY_CALLER = 'T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb';

export function loadTronConfig(env: NodeJS.ProcessEnv = process.env): TronConfig {
  let commitmentMap: Record<string, number> = {};
  try {
    commitmentMap = JSON.parse(env.D3RAC_COMMITMENT_MAP ?? '{}');
  } catch {
    commitmentMap = {};
  }
  return {
    fullNode: env.D3RAC_TRON_FULL_NODE,
    controller: env.D3RAC_TRON_DISBURSEMENT_CONTROLLER,
    commitmentMap,
  };
}

async function readCommitmentCount(cfg: TronConfig): Promise<number> {
  const res = await fetch(`${cfg.fullNode!.replace(/\/$/, '')}/wallet/triggerconstantcontract`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      owner_address: READ_ONLY_CALLER,
      contract_address: cfg.controller,
      function_selector: 'commitmentCount()',
      parameter: '',
      visible: true,
    }),
  });
  if (!res.ok) throw new Error(`full node returned HTTP ${res.status}`);
  const body = (await res.json()) as { constant_result?: string[] };
  const hex = body.constant_result?.[0];
  if (!hex) throw new Error('commitmentCount() returned no result');
  return Number(BigInt('0x' + hex));
}

export async function prepareTronRelease(
  communityId: string,
  milestone: string,
  milestoneIndex: number,
  pct: number,
  cfg: TronConfig = loadTronConfig(),
): Promise<FundReleaseRequest> {
  const base = {
    community_id: communityId,
    chain: 'tron' as const,
    milestone,
    milestone_index: milestoneIndex,
    release_pct: pct,
    tx_hash: null,
  };
  const blocked = (note: string): FundReleaseRequest => ({ ...base, commitment_id: null, status: 'blocked', note });

  if (!cfg.fullNode || !cfg.controller) {
    return blocked('D3RAC_TRON_FULL_NODE / D3RAC_TRON_DISBURSEMENT_CONTROLLER not set');
  }
  if (MAINNET_HOSTS.some((h) => cfg.fullNode!.includes(h))) {
    return blocked('refusing to run against TRON mainnet: this contract suite is not professionally audited');
  }
  const commitmentId = cfg.commitmentMap[communityId];
  if (!Number.isInteger(commitmentId) || commitmentId < 0) {
    return blocked(`no commitment mapped for community ${communityId} in D3RAC_COMMITMENT_MAP`);
  }
  try {
    const count = await readCommitmentCount(cfg);
    if (commitmentId >= count) {
      return { ...base, commitment_id: commitmentId, status: 'blocked', note: `commitment ${commitmentId} does not exist (commitmentCount=${count})` };
    }
  } catch (err) {
    return { ...base, commitment_id: commitmentId, status: 'blocked', note: `readiness check failed: ${(err as Error).message}` };
  }
  return {
    ...base,
    commitment_id: commitmentId,
    status: 'dry-run',
    note: `commitment ${commitmentId} exists; an authorized attester must call attestMilestone(${commitmentId}, ${milestoneIndex}) before releaseMilestone`,
  };
}

function prepareCasperRelease(communityId: string, milestone: string, milestoneIndex: number, pct: number): FundReleaseRequest {
  return {
    community_id: communityId,
    chain: 'casper',
    milestone,
    milestone_index: milestoneIndex,
    release_pct: pct,
    commitment_id: null,
    tx_hash: null,
    status: 'blocked',
    note: 'Casper release preparation not implemented yet',
  };
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
      const tron = await prepareTronRelease(plan.community_id, firstMilestone.name, 0, firstMilestone.release_pct);
      const casper = prepareCasperRelease(plan.community_id, firstMilestone.name, 0, firstMilestone.release_pct);
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
    console.log(`published ${reqs.length} fund.release.requested event(s) (dry-run/blocked -- this agent never broadcasts)`);
  });
}
