import { useState } from "react";
import type { FormEvent } from "react";
import { useWallet } from "../context/useWallet";
import type { DisbursementResult, TokenBalance } from "../lib/chainAdapter";

export default function Disburse() {
  const { adapter, chainId, address, connect, connecting } = useWallet();
  const [tokenContract, setTokenContract] = useState("");
  const [to, setTo] = useState("");
  const [amount, setAmount] = useState("");
  const [balance, setBalance] = useState<TokenBalance | null>(null);
  const [result, setResult] = useState<DisbursementResult | null>(null);
  const [busy, setBusy] = useState<"balance" | "send" | null>(null);
  const [err, setErr] = useState<string | null>(null);

  async function checkBalance() {
    setErr(null);
    setBusy("balance");
    try {
      setBalance(await adapter.getTokenBalance(tokenContract));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Could not read balance.");
    } finally {
      setBusy(null);
    }
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setErr(null);
    setResult(null);
    setBusy("send");
    try {
      setResult(await adapter.disburse({ tokenContract, to, amount }));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Disbursement failed.");
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className="container" style={{ padding: "48px 24px 96px", maxWidth: 640 }}>
      <p className="eyebrow" style={{ marginBottom: 12 }}>Disbursement console</p>
      <h1 style={{ fontSize: 30, marginBottom: 8 }}>Release a milestone payment</h1>
      <p style={{ color: "var(--text-muted)", marginBottom: 32 }}>
        Currently targeting <strong style={{ color: "var(--text)" }}>{adapter.label}</strong>.
        Switch chains from the selector in the top bar.
      </p>

      {!address ? (
        <div className="card" style={{ textAlign: "center" }}>
          <p style={{ marginBottom: 16, color: "var(--text-muted)" }}>
            Connect a {adapter.label} wallet to read balances and send funds.
          </p>
          {adapter.isWalletAvailable() ? (
            <button className="btn btn-primary" onClick={connect} disabled={connecting}>
              {connecting ? "Connecting…" : `Connect ${adapter.label} wallet`}
            </button>
          ) : (
            <a href={adapter.installUrl} target="_blank" rel="noreferrer" className="btn btn-primary">
              Install {adapter.label} wallet
            </a>
          )}
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="card" style={{ display: "flex", flexDirection: "column", gap: 18 }}>
          <Field label="Token contract address">
            <input
              value={tokenContract}
              onChange={(e) => setTokenContract(e.target.value)}
              placeholder={chainId === "tron" ? "T..." : "hash-..."}
              required
              style={inputStyle}
            />
          </Field>

          <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
            <button type="button" className="btn btn-ghost" onClick={checkBalance} disabled={!tokenContract || busy === "balance"}>
              {busy === "balance" ? "Checking…" : "Check balance"}
            </button>
            {balance && (
              <span className="mono" style={{ fontSize: 13, color: "var(--teal)" }}>
                {balance.amount} {balance.symbol}
              </span>
            )}
          </div>

          <Field label="Recipient address">
            <input value={to} onChange={(e) => setTo(e.target.value)} required style={inputStyle} />
          </Field>

          <Field label="Amount">
            <input value={amount} onChange={(e) => setAmount(e.target.value)} type="number" step="any" min="0" required style={inputStyle} />
          </Field>

          <button type="submit" className="btn btn-primary" disabled={busy === "send"}>
            {busy === "send" ? "Broadcasting…" : "Release funds"}
          </button>

          {err && <p style={{ color: "var(--coral)", fontSize: 13 }}>{err}</p>}
          {result && (
            <p style={{ color: "var(--teal)", fontSize: 13 }} className="mono">
              Sent. <a href={result.explorerUrl} target="_blank" rel="noreferrer" style={{ textDecoration: "underline" }}>View on explorer →</a>
            </p>
          )}
        </form>
      )}
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <span style={{ fontSize: 13, color: "var(--text-muted)" }}>{label}</span>
      {children}
    </label>
  );
}

const inputStyle: React.CSSProperties = {
  background: "var(--bg-raised)",
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: "10px 12px",
  color: "var(--text)",
  fontFamily: "var(--font-mono)",
  fontSize: 14,
};
