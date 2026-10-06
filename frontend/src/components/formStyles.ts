// Shared input styling for address/amount entry (kept out of FormField.tsx so that
// file only exports a component, which Vite fast refresh requires).
import type { CSSProperties } from "react";

export const inputStyle: CSSProperties = {
  background: "var(--bg-raised)",
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: "10px 12px",
  color: "var(--text)",
  fontFamily: "var(--font-mono)",
  fontSize: 14,
};
