import { COMMUNITIES, riskScore, riskTier } from "../lib/riskModel";

const COLORS = {
  watch: "var(--teal)",
  elevated: "var(--amber)",
  critical: "var(--coral)",
};

export default function CommunityCards() {
  const rows = COMMUNITIES.map((c) => ({
    ...c,
    score: riskScore(c),
    tier: riskTier(riskScore(c)),
  })).sort((a, b) => b.score - a.score);

  return (
    <div
      className="community-cards"
      style={{
        display: "grid",
        gap: 16,
      }}
    >
      {rows.map((r) => (
        <div
          key={r.id}
          className="card"
          style={{ padding: 20 }}
        >
          <h3 style={{ margin: 0 }}>{r.name}</h3>

          <p style={{ color: "var(--text-muted)", margin: "6px 0 18px" }}>
            {r.region}
          </p>

          <div style={{ display: "grid", gap: 8 }}>
            <div>Risk Score: <strong>{r.score.toFixed(3)}</strong></div>
            <div>Hazard: {r.hazard.toFixed(2)}</div>
            <div>Exposure: {r.exposure.toFixed(2)}</div>
            <div>Vulnerability: {r.vulnerability.toFixed(2)}</div>
            <div>
              Status:
              <span
                style={{
                  color: COLORS[r.tier],
                  marginLeft: 8,
                  fontWeight: 700,
                }}
              >
                {r.tier.toUpperCase()}
              </span>
            </div>
            <div>
              Milestones: {r.fundedMilestones}/{r.totalMilestones}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
