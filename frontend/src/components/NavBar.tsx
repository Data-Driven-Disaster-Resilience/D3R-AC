import { NavLink } from "react-router-dom";
import ChainSelector from "./ChainSelector";

const LINKS = [
  { to: "/", label: "Overview", end: true },
  { to: "/dashboard", label: "Risk Dashboard", end: false },
  { to: "/disburse", label: "Disbursement", end: false },
];

export default function NavBar() {
  return (
    <header style={{ borderBottom: "1px solid var(--border-soft)", position: "sticky", top: 0, background: "rgba(10,15,26,0.85)", backdropFilter: "blur(8px)", zIndex: 20 }}>
      <div className="container nav-row">
        <NavLink to="/" style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span style={{ fontFamily: "var(--font-display)", fontWeight: 700, fontSize: 18, letterSpacing: "-0.01em" }}>
            D3R<span style={{ color: "var(--amber)" }}>·</span>AC
          </span>
        </NavLink>

        <nav className="nav-links">
          {LINKS.map((l) => (
            <NavLink
              key={l.to}
              to={l.to}
              end={l.end}
              style={({ isActive }) => ({
                fontSize: 14,
                fontWeight: 500,
                color: isActive ? "var(--text)" : "var(--text-muted)",
                borderBottom: isActive ? "2px solid var(--amber)" : "2px solid transparent",
                paddingBottom: 4,
              })}
            >
              {l.label}
            </NavLink>
          ))}
        </nav>

        <ChainSelector />
      </div>
    </header>
  );
}
