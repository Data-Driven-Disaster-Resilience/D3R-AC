import { COMMUNITIES, riskScore, riskTier } from "../lib/riskModel";

const TIER_COLOR: Record<string, string> = {
  watch: "var(--teal)",
  elevated: "var(--amber)",
  critical: "var(--coral)",
};

// A 6x6 abstract grid — each cell is a synthetic community sampled around
// the real dataset's spread, so the hero reads as "many communities being
// watched at once" rather than one static illustration.
function generateGridScores(seed: number, count: number): number[] {
  const base = COMMUNITIES.map((c) => riskScore(c));
  const scores: number[] = [];
  for (let i = 0; i < count; i++) {
    const b = base[i % base.length] ?? 0.3;
    const jitter = (Math.sin(seed + i * 12.9898) * 0.5 + 0.5) * 0.18 - 0.09;
    scores.push(Math.min(1, Math.max(0.02, b + jitter)));
  }
  return scores;
}

export default function RiskPulseGrid() {
  const cols = 8;
  const rows = 5;
  const scores = generateGridScores(3.14, cols * rows);
  const cell = 32;
  const gap = 10;
  const width = cols * cell + (cols - 1) * gap;
  const height = rows * cell + (rows - 1) * gap;

  return (
    <svg viewBox={`0 0 ${width} ${height}`} width="100%" height="100%" role="img" aria-label="Grid of community risk scores, pulsing amber and coral where hazard risk is elevated">
      {scores.map((s, i) => {
        const col = i % cols;
        const row = Math.floor(i / cols);
        const x = col * (cell + gap) + cell / 2;
        const y = row * (cell + gap) + cell / 2;
        const tier = riskTier(s);
        const color = TIER_COLOR[tier];
        const r = 4 + s * 10;
        const delay = (i % cols) * 0.12 + Math.floor(i / cols) * 0.07;
        return (
          <g key={i}>
            <circle cx={x} cy={y} r={cell / 2 - 2} fill="none" stroke="var(--border-soft)" strokeWidth="1" />
            <circle cx={x} cy={y} r={r} fill={color} opacity={0.85}>
              {tier !== "watch" && (
                <animate
                  attributeName="opacity"
                  values="0.85;0.35;0.85"
                  dur={`${2.4 + (i % 3) * 0.4}s`}
                  begin={`${delay}s`}
                  repeatCount="indefinite"
                />
              )}
            </circle>
          </g>
        );
      })}
    </svg>
  );
}
