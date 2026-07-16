import { Link } from "react-router-dom";

export default function NotFound() {
  return (
    <section className="container" style={{ padding: "96px 24px", textAlign: "center" }}>
      <p className="eyebrow" style={{ marginBottom: 12 }}>404</p>
      <h1 style={{ fontSize: 30, marginBottom: 12 }}>Nothing monitored here.</h1>
      <p style={{ color: "var(--text-muted)", marginBottom: 24 }}>
        That page doesn't exist. Head back to the overview.
      </p>
      <Link to="/" className="btn btn-primary">Back to overview</Link>
    </section>
  );
}
