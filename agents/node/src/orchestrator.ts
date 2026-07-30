/**
 * Usage: npm run start -- <agent-id>
 *        npm run start -- run-all
 */
import { NODE_AGENT_IDS } from './config/generatedManifest';
import { CoordinationAgent } from './agents/coordinationAgent';
import { AlertAgent } from './agents/alertAgent';
import { ContractTriggerAgent } from './agents/contractTriggerAgent';

const RUN_ORDER = ['response-coordinator', 'community-alerter', 'contract-trigger'];

const REGISTRY: Record<string, () => Promise<unknown[]>> = {
  'response-coordinator': () => new CoordinationAgent().runOnce(),
  'community-alerter': () => new AlertAgent().runOnce().then((n) => [n]),
  'contract-trigger': () => new ContractTriggerAgent().runOnce(),
};

async function run(agentId: string): Promise<void> {
  if (!NODE_AGENT_IDS.includes(agentId) && agentId !== 'run-all') {
    console.error(`'${agentId}' is not a node agent. Known node agents: ${NODE_AGENT_IDS.join(', ')}`);
    process.exit(1);
  }
  const results = await REGISTRY[agentId]();
  console.log(`[${agentId}] produced ${results.length} result(s)`);
}

async function runAll(): Promise<void> {
  for (const agentId of RUN_ORDER) {
    await run(agentId);
  }
}

async function main(): Promise<void> {
  const arg = process.argv[2];
  if (!arg) {
    console.log(__filename + ' requires an agent id, or "run-all"');
    process.exit(1);
  }
  if (arg === 'run-all') {
    await runAll();
  } else {
    await run(arg);
  }
}

main();
