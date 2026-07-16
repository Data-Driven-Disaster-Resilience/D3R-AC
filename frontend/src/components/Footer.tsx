export default function Footer() {
  return (
    <footer style={{ borderTop: "1px solid var(--border-soft)", marginTop: 80 }}>
      <div className="container" style={{ padding: "28px 24px", display: "flex", justifyContent: "space-between", flexWrap: "wrap", gap: 12 }}>
        <p className="mono" style={{ fontSize: 12, color: "var(--text-faint)" }}>
          Built by TAAD — The Abuja Algorithmic Defenders · Abuja, Nigeria
        </p>
        <p className="mono" style={{ fontSize: 12, color: "var(--text-faint)" }}>
          R(c,t) = H(t) · E(c) · V(c) — θ = {"0.35"}
        </p>
      </div>
    </footer>
  );
}
