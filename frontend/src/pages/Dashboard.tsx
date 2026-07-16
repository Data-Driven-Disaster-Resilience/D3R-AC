import { COMMUNITIES, riskScore, riskTier, RISK_THRESHOLD } from "../lib/riskModel";
import type { RiskTier } from "../lib/riskModel";

const TIER_STYLE: Record<RiskTier, { color: string; label: string }> = {
  watch: { color: "var(--teal)", label: "Watch" },
  elevated: { color: "var(--amber)", label: "Elevated" },
  critical: { color: "var(--coral)", label: "Critical" },
};

export default function Dashboard() {
  const rows = COMMUNITIES.map((c) => ({ ...c, score: riskScore(c), tier: riskTier(riskScore(c)) }))
    .sort((a, b) => b.score - a.score);

  return (
    <section className="container" style={{ padding: "48px 24px 80px" }}>
      <p className="eyebrow" style={{ marginBottom: 12 }}>Risk dashboard</p>
      <h1 style={{ fontSize: 34, marginBottom: 8 }}>Communities by resilience-funding priority</h1>
      <p style={{ color: "var(--text-muted)", maxWidth: 620, marginBottom: 32 }}>
        Sorted by R(c,t) = H(t)·E(c)·V(c). Threshold θ = {RISK_THRESHOLD} — scores at or above it
        are eligible for milestone-based fund pre-positioning.
      </p>

      <div className="card" style={{ padding: 0, overflow: "hidden" }}>
        <div className="table-scroll">
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid var(--border-soft)" }}>
              {["Community", "Region", "H(t)", "E(c)", "V(c)", "R(c,t)", "Status", "Milestones"].map((h) => (
                <th key={h} style={{ textAlign: "left", padding: "14px 20px", fontSize: 12, color: "var(--text-faint)", fontWeight: 500, textTransform: "uppercase", letterSpacing: "0.04em" }}>
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => {
              const tier = TIER_STYLE[r.tier];
              return (
                <tr key={r.id} style={{ borderBottom: "1px solid var(--border-soft)" }}>
                  <td style={{ padding: "16px 20px", fontWeight: 500 }}>{r.name}</td>
                  <td style={{ padding: "16px 20px", color: "var(--text-muted)", fontSize: 14 }}>{r.region}</td>
                  <td className="mono" style={{ padding: "16px 20px", fontSize: 14 }}>{r.hazard.toFixed(2)}</td>
                  <td className="mono" style={{ padding: "16px 20px", fontSize: 14 }}>{r.exposure.toFixed(2)}</td>
                  <td className="mono" style={{ padding: "16px 20px", fontSize: 14 }}>{r.vulnerability.toFixed(2)}</td>
                  <td className="mono" style={{ padding: "16px 20px", fontSize: 14, fontWeight: 600 }}>{r.score.toFixed(3)}</td>
                  <td style={{ padding: "16px 20px" }}>
                    <span className="pill" style={{ borderColor: tier.color, color: tier.color }}>
                      <span className="dot" style={{ background: tier.color }} />
                      {tier.label}
                    </span>
                  </td>
                  <td style={{ padding: "16px 20px", fontSize: 13, color: "var(--text-muted)" }}>
                    {r.fundedMilestones} / {r.totalMilestones} funded
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        </div>
      </div>
    </section>
  );
}
