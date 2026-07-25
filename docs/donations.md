# D3R·AC Donation Addresses

D3R·AC accepts direct crypto donations to fund disaster relief disbursements
to communities on the platform. These are manual, human-facing donation
addresses — **not** the on-chain treasury address used internally by the
smart contract suite (`MultiSigAdmin` / `DisbursementController`).

| Asset | Network | Address | Notes |
|---|---|---|---|
| TRX or any TRC20 token | TRON | `TQY3soLWETYtzY9upUCzvVGNM3uz6qFPDc` | **Default D3R·AC TRON address** — use this if unsure |
| USDT | TRC20 (TRON) | `TEVicJn5i259iNdnhvdjifkiP6yb9wPu6r` | Dedicated USDT-TRC20 address |
| TRX | TRON | `TLwdE3VYBAVsQX4DGGmK1FKttkD6R6wEds` | Dedicated TRX-only address |
| BTC | Bitcoin | `bc1qp8nr2y2zqtfmv7ydvmyrcuwmdsm745ewyfn6g6` | |
| USDT | ERC20 (Ethereum) | `0x1c07b0eb7f3ddabce08a2e9ce2adad78e4601489` | For NGOs/donors using Ethereum-based USDT |

**Important:** always match the asset to its correct network. Sending
USDT-TRC20 to the ERC20 address (or vice versa) will result in permanent
loss of funds — TRON and Ethereum are separate, incompatible networks. If
you're unsure which TRON address to use, send to the **default** address
above; it accepts TRX and any TRC20 token, including USDT.

These addresses are rendered in the frontend via
`frontend/src/lib/donationAddresses.ts`.
