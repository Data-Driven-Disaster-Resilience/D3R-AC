import { AdapterNotReadyError } from "./chainAdapter";
import type { ChainAdapter, TokenBalance, DisbursementResult } from "./chainAdapter";

// Casper deployment is marked "in progress" in the project README.
// This adapter implements the same ChainAdapter contract as TRON so the
// UI can already offer Casper as a destination — swap the method bodies
// for casper-js-sdk calls once the Casper contract lands, no page changes needed.

declare global {
  interface Window {
    CasperWalletProvider?: () => any;
  }
}

class CasperAdapter implements ChainAdapter {
  id = "casper" as const;
  label = "Casper";
  nativeSymbol = "CSPR";
  installUrl = "https://www.casperwallet.io/";
  private address: string | null = null;

  isWalletAvailable(): boolean {
    return typeof window !== "undefined" && !!window.CasperWalletProvider;
  }

  async connect(): Promise<string> {
    if (!this.isWalletAvailable()) throw new AdapterNotReadyError("casper");
    const provider = window.CasperWalletProvider!();
    const connected = await provider.requestConnection();
    if (!connected) throw new Error("Casper Wallet connection was declined.");
    this.address = await provider.getActivePublicKey();
    return this.address!;
  }

  getAddress(): string | null {
    return this.address;
  }

  async getTokenBalance(_tokenContract: string): Promise<TokenBalance> {
    throw new Error("Casper token contracts are not deployed yet — see project README status.");
  }

  async disburse(_params: { tokenContract: string; to: string; amount: string }): Promise<DisbursementResult> {
    throw new Error("Casper disbursement is not available yet — Casper contracts are in progress.");
  }

  explorerAddressUrl(address: string): string {
    return `https://testnet.cspr.live/account/${address}`;
  }
}

export const casperAdapter = new CasperAdapter();
