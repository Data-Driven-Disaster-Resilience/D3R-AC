// Shared form field wrapper + input styling, extracted from
// Disburse.tsx's own inline `Field`/`inputStyle` so Wallet.tsx doesn't
// duplicate them -- both pages want the exact same label-above-input
// layout and monospace input styling for address/amount entry.
import type { ReactNode, CSSProperties } from "react";

export function FormField({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <span style={{ fontSize: 13, color: "var(--text-muted)" }}>{label}</span>
      {children}
    </label>
  );
}

export const inputStyle: CSSProperties = {
  background: "var(--bg-raised)",
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: "10px 12px",
  color: "var(--text)",
  fontFamily: "var(--font-mono)",
  fontSize: 14,
};
