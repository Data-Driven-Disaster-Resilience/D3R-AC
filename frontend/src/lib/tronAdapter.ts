import { AdapterNotReadyError } from "./chainAdapter";
import type { ChainAdapter, TokenBalance, DisbursementResult } from "./chainAdapter";

// Minimal TRC-20 ABI — balanceOf, decimals, symbol, transfer.
// This is the "classic token" surface: read a balance, move a disbursement.
const TRC20_ABI = [
  {
    constant: true,
    inputs: [{ name: "_owner", type: "address" }],
    name: "balanceOf",
    outputs: [{ name: "balance", type: "uint256" }],
    type: "function",
  },
  { constant: true, inputs: [], name: "decimals", outputs: [{ name: "", type: "uint8" }], type: "function" },
  { constant: true, inputs: [], name: "symbol", outputs: [{ name: "", type: "string" }], type: "function" },
  {
    constant: false,
    inputs: [
      { name: "_to", type: "address" },
      { name: "_value", type: "uint256" },
    ],
    name: "transfer",
    outputs: [{ name: "", type: "bool" }],
    type: "function",
  },
];

// Precision-safe decimal <-> integer conversion using BigInt and string
// manipulation instead of floating point, so large balances/amounts with
// high decimal counts (e.g. 18) don't lose precision the way
// Number(raw) / 10**decimals or amount * 10**decimals would.
function formatUnits(raw: string, decimals: number): string {
  const negative = raw.startsWith("-");
  const digits = (negative ? raw.slice(1) : raw).padStart(decimals + 1, "0");
  const whole = digits.slice(0, digits.length - decimals) || "0";
  const frac = digits.slice(digits.length - decimals).replace(/0+$/, "");
  const value = frac ? `${whole}.${frac}` : whole;
  return negative ? `-${value}` : value;
}

function parseUnits(amount: string, decimals: number): bigint {
  const trimmed = amount.trim();
  const negative = trimmed.startsWith("-");
  const unsigned = negative ? trimmed.slice(1) : trimmed;
  const [wholePart = "0", fracPart = ""] = unsigned.split(".");
  if (!/^\d*$/.test(wholePart) || !/^\d*$/.test(fracPart)) {
    throw new Error(`"${amount}" is not a valid decimal amount.`);
  }
  const fracPadded = fracPart.slice(0, decimals).padEnd(decimals, "0");
  const combined = BigInt((wholePart || "0") + fracPadded || "0");
  return negative ? -combined : combined;
}

declare global {
  interface Window {
    tronWeb?: any;
    tronLink?: any;
  }
}

const NETWORK = (import.meta.env.VITE_TRON_NETWORK as string) || "shasta"; // testnet by default
const EXPLORER_BASE =
  NETWORK === "mainnet" ? "https://tronscan.org/#" : "https://shasta.tronscan.org/#";

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
    if (!this.isWalletAvailable()) throw new AdapterNotReadyError("tron");
    await window.tronLink.request({ method: "tron_requestAccounts" });
    const addr = window.tronWeb?.defaultAddress?.base58;
    if (!addr) throw new Error("TronLink did not return an address. Unlock the extension and retry.");
    this.address = addr;
    return addr;
  }

  getAddress(): string | null {
    return this.address ?? window.tronWeb?.defaultAddress?.base58 ?? null;
  }

  async getTokenBalance(tokenContract: string): Promise<TokenBalance> {
    if (!window.tronWeb) throw new AdapterNotReadyError("tron");
    const contract = await window.tronWeb.contract(TRC20_ABI, tokenContract);
    const owner = this.getAddress();
    if (!owner) throw new Error("Connect a wallet before reading a balance.");
    const [raw, decimals, symbol] = await Promise.all([
      contract.balanceOf(owner).call(),
      contract.decimals().call().catch(() => 6),
      contract.symbol().call().catch(() => "TOKEN"),
    ]);
    const rawStr = raw.toString();
    const amount = formatUnits(rawStr, Number(decimals));
    return { symbol, amount, raw: rawStr };
  }

  async disburse(params: { tokenContract: string; to: string; amount: string }): Promise<DisbursementResult> {
    if (!window.tronWeb) throw new AdapterNotReadyError("tron");
    const contract = await window.tronWeb.contract(TRC20_ABI, params.tokenContract);
    const decimals = await contract.decimals().call().catch(() => 6);
    const rawAmount = parseUnits(params.amount, Number(decimals));
    if (rawAmount <= 0n) throw new Error("Amount must be greater than zero.");
    const tx = await contract.transfer(params.to, rawAmount.toString()).send();
    const txHash = typeof tx === "string" ? tx : tx.txid || tx;
    return { txHash, explorerUrl: `${EXPLORER_BASE}/transaction/${txHash}` };
  }

  explorerAddressUrl(address: string): string {
    return `${EXPLORER_BASE}/address/${address}`;
  }
}

export const tronAdapter = new TronAdapter();
