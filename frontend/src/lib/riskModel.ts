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

// Illustrative dataset standing in for the data-pipeline layer described
// in the README, which is marked [TBD] there.
export const COMMUNITIES: Community[] = [
  { id: "c1", name: "Kebbi River Basin", region: "Kebbi, NG", hazard: 0.72, exposure: 0.61, vulnerability: 0.58, fundedMilestones: 1, totalMilestones: 4 },
  { id: "c2", name: "Lokoja Confluence", region: "Kogi, NG", hazard: 0.55, exposure: 0.7, vulnerability: 0.49, fundedMilestones: 2, totalMilestones: 4 },
  { id: "c3", name: "Maiduguri Corridor", region: "Borno, NG", hazard: 0.81, exposure: 0.66, vulnerability: 0.74, fundedMilestones: 0, totalMilestones: 5 },
  { id: "c4", name: "Port Harcourt Delta", region: "Rivers, NG", hazard: 0.4, exposure: 0.52, vulnerability: 0.35, fundedMilestones: 3, totalMilestones: 3 },
  { id: "c5", name: "Sokoto Frontier", region: "Sokoto, NG", hazard: 0.63, exposure: 0.44, vulnerability: 0.51, fundedMilestones: 1, totalMilestones: 4 },
  { id: "c6", name: "Cross River Uplands", region: "Cross River, NG", hazard: 0.29, exposure: 0.38, vulnerability: 0.31, fundedMilestones: 2, totalMilestones: 2 },
];
