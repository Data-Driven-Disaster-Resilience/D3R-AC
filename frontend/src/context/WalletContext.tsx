import { createContext, useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { ChainAdapter, ChainId } from "../lib/chainAdapter";
import { tronAdapter } from "../lib/tronAdapter";
import { casperAdapter } from "../lib/casperAdapter";

const ADAPTERS: Record<ChainId, ChainAdapter> = {
  tron: tronAdapter,
  casper: casperAdapter,
};

export interface WalletState {
  chainId: ChainId;
  adapter: ChainAdapter;
  address: string | null;
  connecting: boolean;
  error: string | null;
  setChain: (id: ChainId) => void;
  connect: () => Promise<void>;
  availableChains: { id: ChainId; label: string; available: boolean }[];
}

export const WalletContext = createContext<WalletState | null>(null);

function pickAvailableChain(): ChainId | null {
  if (tronAdapter.isWalletAvailable()) return "tron";
  if (casperAdapter.isWalletAvailable()) return "casper";
  return null;
}

export function WalletProvider({ children }: { children: ReactNode }) {
  // Browser wallet extensions (TronLink, Casper Wallet) often inject their
  // provider onto `window` a beat after the page loads, not before —
  // checking availability only once at mount can wrongly report "no
  // wallet" for a wallet that's actually installed. `detectTick` forces a
  // few re-checks over the first couple seconds to catch late injection.
  const [detectTick, setDetectTick] = useState(0);
  useEffect(() => {
    if (detectTick >= 5) return;
    const t = setTimeout(() => setDetectTick((n) => n + 1), 400);
    return () => clearTimeout(t);
  }, [detectTick]);

  const [chainId, setChainId] = useState<ChainId>(() => pickAvailableChain() ?? "tron");
  const [chainManuallySet, setChainManuallySet] = useState(false);
  const [address, setAddress] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const availableChains = useMemo(
    () =>
      (Object.keys(ADAPTERS) as ChainId[]).map((id) => ({
        id,
        label: ADAPTERS[id].label,
        available: ADAPTERS[id].isWalletAvailable(),
      })),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [detectTick]
  );

  // If the user hasn't explicitly picked a chain and a wallet shows up
  // late for a different chain than the current default, follow it once.
  useEffect(() => {
    if (chainManuallySet || address) return;
    const detected = pickAvailableChain();
    if (detected && detected !== chainId) setChainId(detected);
  }, [detectTick, chainManuallySet, address, chainId]);

  const adapter = ADAPTERS[chainId];

  const setChain = useCallback((id: ChainId) => {
    setChainId(id);
    setChainManuallySet(true);
    setAddress(null);
    setError(null);
  }, []);

  const connect = useCallback(async () => {
    setConnecting(true);
    setError(null);
    try {
      const addr = await ADAPTERS[chainId].connect();
      setAddress(addr);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Connection failed.");
    } finally {
      setConnecting(false);
    }
  }, [chainId]);

  const value: WalletState = {
    chainId,
    adapter,
    address,
    connecting,
    error,
    setChain,
    connect,
    availableChains,
  };

  return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>;
}
