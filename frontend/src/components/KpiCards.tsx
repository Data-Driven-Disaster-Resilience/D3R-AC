import { COMMUNITIES, riskScore, RISK_THRESHOLD } from "../lib/riskModel";

export default function KpiCards() {
  const scores = COMMUNITIES.map(riskScore);

  const critical = scores.filter((s) => s >= RISK_THRESHOLD).length;
  const average =
    scores.reduce((a, b) => a + b, 0) / scores.length;

  const funded = COMMUNITIES.reduce(
    (sum, c) => sum + c.fundedMilestones,
    0
  );

  const total = COMMUNITIES.reduce(
    (sum, c) => sum + c.totalMilestones,
    0
  );

  const cards = [
    {
      title: "Communities",
      value: COMMUNITIES.length,
    },
    {
      title: "Critical Risk",
      value: critical,
    },
    {
      title: "Average Risk",
      value: average.toFixed(2),
    },
    {
      title: "Milestones",
      value: `${funded}/${total}`,
    },
  ];

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit,minmax(180px,1fr))",
        gap: 16,
        marginBottom: 24,
      }}
    >
      {cards.map((card) => (
        <div
          key={card.title}
          className="card"
          style={{ padding: 20 }}
        >
          <div
            style={{
              color: "var(--text-muted)",
              fontSize: 14,
              marginBottom: 8,
            }}
          >
            {card.title}
          </div>

          <div
            style={{
              fontSize: 30,
              fontWeight: 700,
            }}
          >
            {card.value}
          </div>
        </div>
      ))}
    </div>
  );
}
