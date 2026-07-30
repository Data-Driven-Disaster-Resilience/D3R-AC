/**
 * response-coordinator: subscribes to risk.score AND brainbox.directive, drafts a
 * milestone-based response plan for any community whose R(c,t) crossed theta.
 *
 * Prefers the brainbox directive (Claude's reasoning, or its deterministic fallback if
 * no ANTHROPIC_API_KEY is set) for recommended_action and any release_pct_override. If
 * brainbox published no directive for a community (e.g. it wasn't run yet), falls back
 * to the built-in rule below so the pipeline never silently drops a triggered community.
 */
import { getBus } from '../bus/messageBus';

export interface RiskScoreEvent {
  community_id: string;
  hazard_probability: number;
  exposure_factor: number;
  vulnerability_index: number;
  risk_score: number;
  theta: number;
  triggered: boolean;
}

export interface BrainboxDirective {
  community_id: string;
  priority: number;
  recommended_action: 'monitor' | 'pre-position-and-monitor' | 'immediate-fund-release';
  release_pct_override: number | null;
  rationale: string;
  alert_message: string;
  source: 'claude' | 'fallback-deterministic';
}

export interface ResponsePlan {
  community_id: string;
  risk_score: number;
  milestones: { name: string; release_pct: number }[];
  recommended_action: string;
  directive_source: 'claude' | 'fallback-deterministic' | 'built-in-rule';
}

function builtInAction(riskScore: number): string {
  return riskScore >= 0.85 ? 'immediate-fund-release' : 'pre-position-and-monitor';
}

function draftPlan(risk: RiskScoreEvent, directive: BrainboxDirective | undefined): ResponsePlan {
  const firstMilestonePct = directive?.release_pct_override ?? 30;
  return {
    community_id: risk.community_id,
    risk_score: risk.risk_score,
    // Standard 3-milestone disbursement: matches the "conditional, milestone-based"
    // release model described in the D3R-AC contracts/tron README. brainbox can
    // override the first milestone's percentage via release_pct_override.
    milestones: [
      { name: 'pre-position-funds', release_pct: firstMilestonePct },
      { name: 'confirmed-impact', release_pct: 40 },
      { name: 'recovery-verified', release_pct: Math.max(0, 100 - firstMilestonePct - 40) },
    ],
    recommended_action: directive?.recommended_action ?? builtInAction(risk.risk_score),
    directive_source: directive?.source ?? 'built-in-rule',
  };
}

export class CoordinationAgent {
  private bus = getBus();

  async runOnce(): Promise<ResponsePlan[]> {
    const riskEvents = await this.bus.readAll<RiskScoreEvent>('risk.score');
    const directiveEvents = await this.bus.readAll<BrainboxDirective>('brainbox.directive');
    const directivesByCommunity = new Map(directiveEvents.map((e) => [e.payload.community_id, e.payload]));

    const plans: ResponsePlan[] = [];
    for (const event of riskEvents) {
      if (!event.payload.triggered) continue;
      const directive = directivesByCommunity.get(event.payload.community_id);
      const plan = draftPlan(event.payload, directive);
      await this.bus.publish('response.plan', plan);
      plans.push(plan);
    }
    return plans;
  }
}

if (require.main === module) {
  new CoordinationAgent().runOnce().then((plans) => {
    console.log(`published ${plans.length} response.plan event(s)`);
    for (const p of plans) {
      console.log(`  ${p.community_id}: ${p.recommended_action} (via ${p.directive_source})`);
    }
  });
}
