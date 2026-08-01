// R(c, t) = H(t) · E(c) · V(c)
// H: hazard probability at time t | E: exposure factor | V: vulnerability index
// Threshold θ triggers fund pre-positioning.

export interface Community {
  id: string;
  name: string;
  region: string;
  hazard: number; // H(t), 0–1
  exposure: number; // E(c), 0–1
  vulnerability: number; // V(c), 0–1
  fundedMilestones: number;
  totalMilestones: number;
}

export const RISK_THRESHOLD = 0.35;

export function riskScore(c: Pick<Community, "hazard" | "exposure" | "vulnerability">): number {
  return c.hazard * c.exposure * c.vulnerability;
}

export type RiskTier = "watch" | "elevated" | "critical";

export function riskTier(score: number): RiskTier {
  if (score >= RISK_THRESHOLD * 1.8) return "critical";
  if (score >= RISK_THRESHOLD) return "elevated";
  return "watch";
}

// Illustrative fallback dataset, used by src/lib/dataFeed.ts whenever the
// data-pipeline's live feed (frontend/public/data/communities.json,
// written by data-pipeline/scripts/run_cycle.py) isn't reachable — see
// data-pipeline/README.md. `id` values match the off-chain community
// slugs in data-pipeline/config/communities.yaml so a live row and its
// local milestone data (fundedMilestones/totalMilestones, which the
// hazard pipeline doesn't compute — that's DisbursementController's job)
// merge together correctly by id.
export const COMMUNITIES: Community[] = [
  { id: "kebbi-river-basin", name: "Kebbi River Basin", region: "Kebbi, NG", hazard: 0.72, exposure: 0.61, vulnerability: 0.58, fundedMilestones: 1, totalMilestones: 4 },
  { id: "lokoja-confluence", name: "Lokoja Confluence", region: "Kogi, NG", hazard: 0.55, exposure: 0.7, vulnerability: 0.49, fundedMilestones: 2, totalMilestones: 4 },
  { id: "maiduguri-corridor", name: "Maiduguri Corridor", region: "Borno, NG", hazard: 0.81, exposure: 0.66, vulnerability: 0.74, fundedMilestones: 0, totalMilestones: 5 },
  { id: "port-harcourt-delta", name: "Port Harcourt Delta", region: "Rivers, NG", hazard: 0.4, exposure: 0.52, vulnerability: 0.35, fundedMilestones: 3, totalMilestones: 3 },
  { id: "sokoto-frontier", name: "Sokoto Frontier", region: "Sokoto, NG", hazard: 0.63, exposure: 0.44, vulnerability: 0.51, fundedMilestones: 1, totalMilestones: 4 },
  { id: "cross-river-uplands", name: "Cross River Uplands", region: "Cross River, NG", hazard: 0.29, exposure: 0.38, vulnerability: 0.31, fundedMilestones: 2, totalMilestones: 2 },
];
