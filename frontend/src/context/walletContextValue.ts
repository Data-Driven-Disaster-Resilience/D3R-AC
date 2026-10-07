// Context object + state type live apart from WalletContext.tsx (the provider
// component) so that file only exports a component, as Vite fast refresh requires.
import { createContext } from "react";
import type { ChainAdapter, ChainId } from "../lib/chainAdapter";

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
