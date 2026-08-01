/**
 * Public donation addresses for D3R·AC.
 *
 * These are human-facing addresses for manual donations from individuals
 * and NGOs. They are NOT on-chain treasury/admin addresses used by the
 * smart contract suite (see MultiSigAdmin / DisbursementController for
 * that) and are never read by contract code — different chains here use
 * incompatible address formats and cannot be mixed into TRON or Casper
 * contract logic.
 *
 * Always display the network label next to the address in the UI so
 * donors don't send funds on the wrong network. Where two addresses exist
 * on the same network (e.g. the general-purpose TRON default vs. the
 * TRX-only address), show both labels distinctly — do not collapse them.
 */

export interface DonationAddress {
  /** Human-readable network/asset label shown in the UI */
  label: string;
  address: string;
  /** True if this is the official "send here if unsure" default address */
  isDefault?: boolean;
}

export const DONATION_ADDRESSES: DonationAddress[] = [
  {
    label: "TRON (Default D3R·AC address — TRX or any TRC20 token)",
    address: "TQY3soLWETYtzY9upUCzvVGNM3uz6qFPDc",
    isDefault: true,
  },
  {
    label: "USDT (TRC20 / TRON)",
    address: "TEVicJn5i259iNdnhvdjifkiP6yb9wPu6r",
  },
  {
    label: "TRX (TRON)",
    address: "TLwdE3VYBAVsQX4DGGmK1FKttkD6R6wEds",
  },
  {
    label: "Bitcoin (BTC)",
    address: "bc1qp8nr2y2zqtfmv7ydvmyrcuwmdsm745ewyfn6g6",
  },
  {
    label: "USDT (ERC20 / Ethereum)",
    address: "0x1c07b0eb7f3ddabce08a2e9ce2adad78e4601489",
  },
];
