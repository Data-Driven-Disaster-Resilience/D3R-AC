import { useContext } from "react";
import { WalletContext } from "./walletContextValue";
import type { WalletState } from "./walletContextValue";

export function useWallet(): WalletState {
  const ctx = useContext(WalletContext);
  if (!ctx) throw new Error("useWallet must be used inside WalletProvider");
  return ctx;
}
