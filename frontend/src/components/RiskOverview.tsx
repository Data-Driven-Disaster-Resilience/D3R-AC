import { COMMUNITIES, riskScore } from "../lib/riskModel";

export default function RiskOverview() {
  const scores = COMMUNITIES.map((c) => ({
    name: c.name,
    score: riskScore(c),
  })).sort((a, b) => b.score - a.score);

  return (
    <div className="card" style={{ padding: 24, marginBottom: 24 }}>
      <h2 style={{ marginTop: 0 }}>Risk Overview</h2>

      {scores.map((item) => (
        <div key={item.name} style={{ marginBottom: 16 }}>
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              marginBottom: 6,
            }}
          >
            <span>{item.name}</span>
            <strong>{item.score.toFixed(3)}</strong>
          </div>

          <div
            style={{
              height: 10,
              background: "var(--border-soft)",
              borderRadius: 999,
              overflow: "hidden",
            }}
          >
            <div
              style={{
                width: `${item.score * 100}%`,
                height: "100%",
                background:
                  item.score >= 0.7
                    ? "var(--coral)"
                    : item.score >= 0.4
                    ? "var(--amber)"
                    : "var(--teal)",
                borderRadius: 999,
              }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}
