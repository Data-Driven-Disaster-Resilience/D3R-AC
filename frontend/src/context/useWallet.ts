import { useContext } from "react";
import { WalletContext } from "./WalletContext";
import type { WalletState } from "./WalletContext";

export function useWallet(): WalletState {
  const ctx = useContext(WalletContext);
  if (!ctx) throw new Error("useWallet must be used inside WalletProvider");
  return ctx;
}
