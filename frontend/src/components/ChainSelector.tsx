import { useWallet } from "../context/useWallet";
import type { ChainId } from "../lib/chainAdapter";

function truncate(addr: string): string {
  return addr.length > 12 ? `${addr.slice(0, 6)}…${addr.slice(-4)}` : addr;
}

export default function ChainSelector() {
  const { chainId, adapter, setChain, availableChains, address, connecting, connect, error } = useWallet();
  const currentAvailable = availableChains.find((c) => c.id === chainId)?.available ?? false;

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
      <select
        value={chainId}
        onChange={(e) => setChain(e.target.value as ChainId)}
        aria-label="Select blockchain"
        style={{
          background: "var(--bg-raised)",
          color: "var(--text)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          padding: "8px 10px",
          fontSize: 13,
          fontFamily: "var(--font-mono)",
        }}
      >
        {availableChains.map((c) => (
          <option key={c.id} value={c.id}>
            {c.label}{c.available ? "" : " (no wallet detected)"}
          </option>
        ))}
      </select>

      {address ? (
        <span className="pill" title={address}>
          <span className="dot" style={{ background: "var(--teal)" }} />
          {truncate(address)}
        </span>
      ) : currentAvailable ? (
        <button className="btn btn-primary" onClick={connect} disabled={connecting}>
          {connecting ? "Connecting…" : "Connect wallet"}
        </button>
      ) : (
        <a
          href={adapter.installUrl}
          target="_blank"
          rel="noreferrer"
          className="btn btn-ghost"
          title={`${adapter.label} wallet not detected — install it to connect`}
        >
          Install {adapter.label} wallet
        </a>
      )}
      {error && (
        <span style={{ fontSize: 12, color: "var(--coral)", maxWidth: 220 }}>{error}</span>
      )}
    </div>
  );
}
