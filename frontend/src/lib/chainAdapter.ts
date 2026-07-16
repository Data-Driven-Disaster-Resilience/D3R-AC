// Shared contract every chain adapter must implement.
// The UI (ChainSelector / WalletContext) talks only to this interface,
// so adding a new chain later never touches page components.

export type ChainId = "tron" | "casper";

export interface TokenBalance {
  symbol: string;
  amount: string; // human-readable, already decimal-adjusted
  raw: string; // on-chain integer amount as string
}

export interface DisbursementResult {
  txHash: string;
  explorerUrl: string;
}

export interface ChainAdapter {
  id: ChainId;
  label: string;
  nativeSymbol: string;
  installUrl: string; // where to install the wallet extension, for adaptive "not detected" guidance
  isWalletAvailable(): boolean;
  connect(): Promise<string>; // returns connected address
  getAddress(): string | null;
  getTokenBalance(tokenContract: string): Promise<TokenBalance>;
  disburse(params: {
    tokenContract: string;
    to: string;
    amount: string; // human-readable amount
  }): Promise<DisbursementResult>;
  explorerAddressUrl(address: string): string;
}

export class AdapterNotReadyError extends Error {
  constructor(chain: ChainId) {
    super(`${chain} wallet is not available in this browser.`);
    this.name = "AdapterNotReadyError";
  }
}
