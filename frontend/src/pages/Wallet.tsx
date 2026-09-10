// Wallet overview page -- connect, view the connected address, and
// look up any token's balance for it. Deliberately scoped to what
// ChainAdapter actually exposes (chainAdapter.ts): there is no generic
// "transfer arbitrary token" method on the adapter interface, only
// getTokenBalance and the disbursement-specific `disburse` (used by
// the existing Disburse console page) -- this page doesn't invent a
// raw transfer capability the adapters don't have; ordinary
// wallet-to-wallet sends are exactly what a user's actual TronLink/
// Casper Wallet extension already does natively, outside this app.
import { useState } from "react";
import type { FormEvent } from "react";
import { useWallet } from "../context/useWallet";
import type { TokenBalance } from "../lib/chainAdapter";
import { FormField, inputStyle } from "../components/FormField";

export default function Wallet() {
  const { adapter, chainId, address, connect, connecting, availableChains, setChain } = useWallet();
  const [tokenContract, setTokenContract] = useState("");
  const [balance, setBalance] = useState<TokenBalance | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function handleLookup(e: FormEvent) {
    e.preventDefault();
    setErr(null);
    setBalance(null);
    setBusy(true);
    try {
      setBalance(await adapter.getTokenBalance(tokenContract));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Could not read balance.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="container" style={{ padding: "48px 24px 96px", maxWidth: 640 }}>
      <p className="eyebrow" style={{ marginBottom: 12 }}>Wallet</p>
      <h1 style={{ fontSize: 30, marginBottom: 8 }}>
        {address ? "Connected wallet" : "Connect a wallet"}
      </h1>
      <p style={{ color: "var(--text-muted)", marginBottom: 32 }}>
        Currently targeting <strong style={{ color: "var(--text)" }}>{adapter.label}</strong>.
      </p>

      {!address ? (
        <div className="card" style={{ textAlign: "center" }}>
          <p style={{ marginBottom: 16, color: "var(--text-muted)" }}>
            Connect a {adapter.label} wallet to view its address and token balances.
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

          <p style={{ marginTop: 20, fontSize: 13, color: "var(--text-muted)" }}>
            Looking for a different chain?{" "}
            {availableChains
              .filter((c) => c.id !== chainId)
              .map((c) => (
                <button
                  key={c.id}
                  onClick={() => setChain(c.id)}
                  style={{ background: "none", border: "none", color: "var(--teal)", cursor: "pointer", textDecoration: "underline", padding: 0, font: "inherit" }}
                >
                  Switch to {c.label}
                </button>
              ))}
          </p>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
          <div className="card">
            <p style={{ fontSize: 13, color: "var(--text-muted)", marginBottom: 6 }}>Address ({adapter.label})</p>
            <p className="mono" style={{ fontSize: 15, wordBreak: "break-all", marginBottom: 12 }}>{address}</p>
            <a
              href={adapter.explorerAddressUrl(address)}
              target="_blank"
              rel="noreferrer"
              style={{ fontSize: 13, color: "var(--teal)", textDecoration: "underline" }}
            >
              View on explorer →
            </a>
          </div>

          <form onSubmit={handleLookup} className="card" style={{ display: "flex", flexDirection: "column", gap: 18 }}>
            <p style={{ fontSize: 15, fontWeight: 600 }}>Look up a token balance</p>
            <FormField label="Token contract address">
              <input
                value={tokenContract}
                onChange={(e) => setTokenContract(e.target.value)}
                placeholder={chainId === "tron" ? "T..." : "hash-..."}
                required
                style={inputStyle}
              />
            </FormField>

            <button type="submit" className="btn btn-primary" disabled={!tokenContract || busy}>
              {busy ? "Checking…" : "Check balance"}
            </button>

            {err && <p style={{ color: "var(--coral)", fontSize: 13 }}>{err}</p>}
            {balance && (
              <p className="mono" style={{ fontSize: 15, color: "var(--teal)" }}>
                {balance.amount} {balance.symbol}
              </p>
            )}
          </form>

          <p style={{ fontSize: 13, color: "var(--text-muted)" }}>
            To send funds, use your {adapter.label} wallet extension directly for an
            ordinary transfer, or the{" "}
            <a href="/disburse" style={{ color: "var(--teal)", textDecoration: "underline" }}>
              Disbursement console
            </a>{" "}
            for a milestone-gated release.
          </p>
        </div>
      )}
    </section>
  );
}
