/**
 * community-alerter: the "community access layer" agent. Subscribes to risk.score,
 * response.plan, and brainbox.directive, and notifies NGO/community-coordinator
 * channels in plain language -- zero blockchain literacy required, per the D3R-AC
 * design goal.
 *
 * Uses brainbox's alert_message (Claude-written, or its deterministic fallback) when
 * available; otherwise falls back to a templated message so nothing goes unsent if
 * brainbox hasn't run.
 *
 * Replace `sendAlert` with real channels (SMS gateway, WhatsApp Business API, email,
 * Slack/Teams webhook for NGO ops rooms, etc).
 */
import { getBus } from '../bus/messageBus';
import { RiskScoreEvent, ResponsePlan, BrainboxDirective } from './coordinationAgent';

async function sendAlert(message: string): Promise<void> {
  // Stub: wire in a real notification channel here.
  console.log(`[ALERT] ${message}`);
}

export class AlertAgent {
  private bus = getBus();

  async runOnce(): Promise<number> {
    const riskEvents = await this.bus.readAll<RiskScoreEvent>('risk.score');
    const planEvents = await this.bus.readAll<ResponsePlan>('response.plan');
    const directiveEvents = await this.bus.readAll<BrainboxDirective>('brainbox.directive');
    const directivesByCommunity = new Map(directiveEvents.map((e) => [e.payload.community_id, e.payload]));
    let sent = 0;

    for (const { payload: risk } of riskEvents) {
      if (!risk.triggered) continue;
      const directive = directivesByCommunity.get(risk.community_id);
      const message =
        directive?.alert_message ??
        `${risk.community_id}: disaster-risk score ${risk.risk_score} has crossed the ` +
          `response threshold (${risk.theta}). Pre-positioning support is being arranged.`;
      await sendAlert(message);
      await this.bus.publish('alert.sent', {
        community_id: risk.community_id,
        kind: 'risk-threshold',
        source: directive?.source ?? 'built-in-template',
      });
      sent += 1;
    }

    for (const { payload: plan } of planEvents) {
      await sendAlert(
        `${plan.community_id}: response plan drafted — ${plan.recommended_action}. ` +
          `${plan.milestones.length} milestone-based disbursements queued.`
      );
      await this.bus.publish('alert.sent', { community_id: plan.community_id, kind: 'response-plan' });
      sent += 1;
    }

    return sent;
  }
}

if (require.main === module) {
  new AlertAgent().runOnce().then((count) => {
    console.log(`sent ${count} alert(s)`);
  });
}
