// Precision-safe token amount conversion.
//
// CONTRIBUTING.md requires "Token amount math must stay precision-safe
// (BigInt/string-based), not floating point" for exactly the reason this
// file exists: `Number(rawBalance) / 10**decimals` silently loses
// precision above Number.MAX_SAFE_INTEGER (2^53-1), which an 18-decimal
// token balance exceeds with a total supply as small as ~9 tokens. For
// a disbursement app, that's not a cosmetic rounding issue — it's real
// funds displayed or sent as the wrong amount.
//
// These two functions are the ethers.js-style `formatUnits`/`parseUnits`
// pair every ChainAdapter implementation should route through, rather
// than each adapter reimplementing (and re-risking) the conversion.

/**
 * Convert an on-chain raw integer amount (as a string or bigint) into a
 * human-readable decimal string, without ever passing through `Number`.
 *
 * @param raw on-chain integer amount, e.g. balanceOf() result
 * @param decimals token decimals (e.g. 18)
 */
export function formatUnits(raw: string | bigint, decimals: number): string {
  if (!Number.isInteger(decimals) || decimals < 0) {
    throw new Error(`Invalid decimals: ${decimals}`);
  }

  const negative = typeof raw === "string" ? raw.trim().startsWith("-") : raw < 0n;
  const value = typeof raw === "string" ? BigInt(raw.trim()) : raw;
  const abs = value < 0n ? -value : value;

  if (decimals === 0) {
    return (negative ? "-" : "") + abs.toString();
  }

  const divisor = 10n ** BigInt(decimals);
  const whole = abs / divisor;
  const fraction = (abs % divisor).toString().padStart(decimals, "0");

  // Trim trailing zeros in the fractional part, but keep at least one
  // digit only if there's a non-zero fraction; otherwise drop the
  // decimal point entirely (matches ethers/viem formatUnits behavior).
  const trimmedFraction = fraction.replace(/0+$/, "");
  const result = trimmedFraction.length > 0
    ? `${whole.toString()}.${trimmedFraction}`
    : whole.toString();

  return (negative && (whole !== 0n || trimmedFraction.length > 0) ? "-" : "") + result;
}

/**
 * Convert a human-readable decimal amount string into the raw on-chain
 * integer amount, as a string (safe to pass to a contract call), without
 * ever passing through `Number`.
 *
 * @param amount human-readable amount, e.g. "12.5"
 * @param decimals token decimals (e.g. 18)
 */
export function parseUnits(amount: string, decimals: number): string {
  if (!Number.isInteger(decimals) || decimals < 0) {
    throw new Error(`Invalid decimals: ${decimals}`);
  }

  const trimmed = amount.trim();
  if (trimmed === "" || !/^-?\d*\.?\d*$/.test(trimmed) || trimmed === "-" || trimmed === ".") {
    throw new Error(`Invalid amount: "${amount}"`);
  }

  const negative = trimmed.startsWith("-");
  const unsigned = negative ? trimmed.slice(1) : trimmed;
  const [wholePartRaw = "", fractionPartRaw = ""] = unsigned.split(".");
  const wholePart = wholePartRaw === "" ? "0" : wholePartRaw;

  if (fractionPartRaw.length > decimals) {
    throw new Error(
      `Amount "${amount}" has more precision (${fractionPartRaw.length} decimal places) than the token supports (${decimals}).`
    );
  }

  const fractionPart = fractionPartRaw.padEnd(decimals, "0");
  const raw = BigInt(wholePart) * 10n ** BigInt(decimals) + (decimals > 0 ? BigInt(fractionPart) : 0n);

  return (negative && raw !== 0n ? "-" : "") + raw.toString();
}
