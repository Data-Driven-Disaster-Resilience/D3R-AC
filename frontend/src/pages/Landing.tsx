import { Link } from "react-router-dom";
import RiskPulseGrid from "../components/RiskPulseGrid";

const LAYERS = [
  {
    n: "01",
    title: "Data layer",
    body: "Ingests hazard signals, displacement indicators, and infrastructure damage reports to decide when and where resilience funding should be pre-positioned.",
  },
  {
    n: "02",
    title: "Smart contract layer",
    body: "Deployed on TRON, with Casper in progress. Handles conditional, milestone-based fund release — auditable on-chain instead of routed through intermediaries.",
  },
  {
    n: "03",
    title: "Community access layer",
    body: "The interface you're looking at — built for NGOs and local coordinators, requiring zero blockchain literacy to track or receive funds.",
  },
];

export default function Landing() {
  return (
    <>
      <section className="container hero-grid" style={{ paddingTop: 64, paddingBottom: 48 }}>
        <div>
          <p className="eyebrow" style={{ marginBottom: 16 }}>Blockchain-powered disaster resilience</p>
          <h1 style={{ fontSize: 48, lineHeight: 1.05, marginBottom: 20 }}>
            Predict the crisis.<br />Disburse before it lands.
          </h1>
          <p style={{ fontSize: 17, color: "var(--text-muted)", maxWidth: 480, marginBottom: 32 }}>
            D3R·AC treats disaster relief as a data and infrastructure problem. On-chain smart
            contracts release funds by verified milestone — transparent, auditable, and fast.
          </p>
          <div style={{ display: "flex", gap: 12 }}>
            <Link to="/dashboard" className="btn btn-primary">View risk dashboard</Link>
            <Link to="/disburse" className="btn btn-ghost">Open disbursement console</Link>
          </div>
        </div>

        <div className="card" style={{ padding: 20 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 14 }}>
            <span className="mono" style={{ fontSize: 12, color: "var(--text-muted)" }}>LIVE WATCH · 40 monitored zones</span>
            <span style={{ display: "flex", gap: 10, fontSize: 11 }} className="mono">
              <span style={{ color: "var(--teal)" }}>● watch</span>
              <span style={{ color: "var(--amber)" }}>● elevated</span>
              <span style={{ color: "var(--coral)" }}>● critical</span>
            </span>
          </div>
          <RiskPulseGrid />
        </div>
      </section>

      <section className="container" style={{ padding: "48px 24px 80px", borderTop: "1px solid var(--border-soft)" }}>
        <h2 style={{ fontSize: 26, marginBottom: 8 }}>Three layers, one release condition</h2>
        <p style={{ color: "var(--text-muted)", maxWidth: 560, marginBottom: 40 }}>
          Risk is modeled as R(c,t) = H(t)·E(c)·V(c). When a community's score crosses the
          threshold θ, the contract layer can trigger pre-positioning automatically.
        </p>
        <div className="layers-grid">
          {LAYERS.map((l) => (
            <div key={l.n} className="card">
              <span className="mono" style={{ color: "var(--amber)", fontSize: 13 }}>{l.n}</span>
              <h3 style={{ fontSize: 18, margin: "10px 0 8px" }}>{l.title}</h3>
              <p style={{ fontSize: 14, color: "var(--text-muted)" }}>{l.body}</p>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}
