import { COMMUNITIES } from "../lib/riskModel";

export default function FundingProgress() {
  return (
    <div className="card" style={{ padding: 24, marginBottom: 24 }}>
      <h2 style={{ marginTop: 0 }}>Funding Progress</h2>

      {COMMUNITIES.map((c) => {
        const percent = (c.fundedMilestones / c.totalMilestones) * 100;

        return (
          <div key={c.id} style={{ marginBottom: 18 }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                marginBottom: 6,
              }}
            >
              <span>{c.name}</span>
              <strong>{Math.round(percent)}%</strong>
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
                  width: `${percent}%`,
                  height: "100%",
                  background: "var(--teal)",
                  borderRadius: 999,
                }}
              />
            </div>

            <small style={{ color: "var(--text-muted)" }}>
              {c.fundedMilestones}/{c.totalMilestones} milestones funded
            </small>
          </div>
        );
      })}
    </div>
  );
}
