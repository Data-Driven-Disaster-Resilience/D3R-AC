import { AdapterNotReadyError } from "./chainAdapter";
import type {
  ChainAdapter,
  TokenBalance,
  DisbursementResult,
} from "./chainAdapter";
import { formatUnits, parseUnits } from "./units";

declare global {
  interface Window {
    tronLink?: any;
    tronWeb?: any;
  }
}

class TronAdapter implements ChainAdapter {
  id = "tron" as const;
  label = "TRON";
  nativeSymbol = "TRX";
  installUrl = "https://www.tronlink.org/";
  private address: string | null = null;

  isWalletAvailable(): boolean {
    return typeof window !== "undefined" && !!window.tronLink;
  }

  async connect(): Promise<string> {
    if (!this.isWalletAvailable()) {
      throw new AdapterNotReadyError("tron");
    }

    await window.tronLink.request({
      method: "tron_requestAccounts",
    });

    this.address = window.tronWeb.defaultAddress.base58;
    return this.address!;
  }

  getAddress(): string | null {
    return this.address;
  }

  async getTokenBalance(tokenContract: string): Promise<TokenBalance> {
  if (!this.address) {
    throw new Error("Wallet not connected.");
  }

  const contract = await window.tronWeb.contract().at(tokenContract);

  const [balance, decimals, symbol] = await Promise.all([
    contract.balanceOf(this.address).call(),
    contract.decimals().call(),
    contract.symbol().call(),
  ]);

  const raw = balance.toString();
  const human = formatUnits(raw, Number(decimals));

  return {
    symbol,
    amount: human,
    raw,
  };
  }

  async disburse(params: {
  tokenContract: string;
  to: string;
  amount: string;
}): Promise<DisbursementResult> {

  if (!this.address) {
    throw new Error("Wallet not connected.");
  }

  const contract = await window.tronWeb.contract().at(
    params.tokenContract
  );

  const decimals = await contract.decimals().call();

  const value = parseUnits(params.amount, Number(decimals));

  const txHash = await contract
    .transfer(params.to, value)
    .send();

  return {
    txHash,
    explorerUrl: `https://tronscan.org/#/transaction/${txHash}`,
  };
  }

  explorerAddressUrl(address: string): string {
    return `https://tronscan.org/#/address/${address}`;
  }
}

export const tronAdapter = new TronAdapter();