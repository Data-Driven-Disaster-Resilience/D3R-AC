#!/usr/bin/env node
// Mines a real TRON keypair whose base58 address (the "T..." form
// wallets display, e.g. TronLink) matches a chosen vanity
// prefix/suffix. Pure brute-force key generation + address derivation
// -- there is no shortcut for this the way CREATE2 lets a contract
// factory's deploy address be mined without touching a private key;
// an account address is derived directly from its public key, so
// finding a vanity one means generating real keypairs until one matches.
//
// Usage:
//   node scripts/find-vanity-tron-wallet.js --prefix D3R
//   node scripts/find-vanity-tron-wallet.js --suffix AID --case-sensitive
//
// SECURITY: this prints a real private key to stdout. Treat the
// output like any other real key -- don't paste it into chat, a
// screenshot, or anywhere logged. Redirect to a file with restrictive
// permissions if scripting this, and clear your terminal scrollback
// after.
import TronWeb from "tronweb";

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = { prefix: "", suffix: "", caseSensitive: false };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--prefix") opts.prefix = args[++i];
    else if (args[i] === "--suffix") opts.suffix = args[++i];
    else if (args[i] === "--case-sensitive") opts.caseSensitive = true;
  }
  return opts;
}

function matches(address, opts) {
  // TRON addresses' base58 form always starts with "T" -- the vanity
  // portion is whatever immediately follows it, so prefix matching
  // skips that fixed leading character rather than requiring the user
  // to include "T" in every --prefix themselves.
  const body = address.slice(1);
  const target = opts.caseSensitive ? body : body.toLowerCase();
  const prefix = opts.caseSensitive ? opts.prefix : opts.prefix.toLowerCase();
  const suffix = opts.caseSensitive ? opts.suffix : opts.suffix.toLowerCase();
  if (prefix && !target.startsWith(prefix)) return false;
  if (suffix && !target.endsWith(suffix)) return false;
  return true;
}

async function main() {
  const opts = parseArgs();
  if (!opts.prefix && !opts.suffix) {
    console.error("Usage: node find-vanity-tron-wallet.js --prefix <str> [--suffix <str>] [--case-sensitive]");
    process.exit(1);
  }

  console.log(`Mining for address matching prefix="${opts.prefix}" suffix="${opts.suffix}" (case-sensitive: ${opts.caseSensitive})...`);
  console.log("Base58 uses a ~58-character alphabet, so each additional character narrows the search space by ~58x -- 4-5 characters is minutes, much more than that can be effectively unbounded.");

  const startedAt = Date.now();
  let attempts = 0;

  while (true) {
    attempts++;
    const account = await TronWeb.utils.accounts.generateAccount();
    if (matches(account.address.base58, opts)) {
      const elapsedSeconds = ((Date.now() - startedAt) / 1000).toFixed(1);
      console.log(`\nFound after ${attempts.toLocaleString()} attempts (${elapsedSeconds}s):`);
      console.log(`  Address (base58): ${account.address.base58}`);
      console.log(`  Address (hex):    ${account.address.hex}`);
      console.log(`  Private key:      ${account.privateKey}`);
      console.log("\nSECURITY: the private key above controls real funds once this address is");
      console.log("funded. Do not paste it anywhere logged (chat, CI output, a committed file).");
      return;
    }
    if (attempts % 10000 === 0) {
      const elapsedSeconds = ((Date.now() - startedAt) / 1000).toFixed(1);
      console.log(`  ...${attempts.toLocaleString()} attempts (${elapsedSeconds}s)`);
    }
  }
}

main();
