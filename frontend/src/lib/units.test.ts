import { describe, it, expect } from "vitest";
import { formatUnits, parseUnits } from "./units";

describe("formatUnits", () => {
  it("formats a simple 18-decimal amount", () => {
    expect(formatUnits("1000000000000000000", 18)).toBe("1");
    expect(formatUnits("1500000000000000000", 18)).toBe("1.5");
  });

  it("formats zero", () => {
    expect(formatUnits("0", 18)).toBe("0");
  });

  it("formats amounts smaller than 1 unit", () => {
    expect(formatUnits("500000000000000000", 18)).toBe("0.5");
    expect(formatUnits("1", 18)).toBe("0.000000000000000001");
  });

  it("does not lose precision above Number.MAX_SAFE_INTEGER", () => {
    // This exact case (a large 18-decimal raw balance) would silently
    // round with `Number(raw) / 10**18` — the bug this file replaces.
    const raw = "123456789012345678901234567890"; // far beyond 2^53
    expect(formatUnits(raw, 18)).toBe("123456789012.34567890123456789");
  });

  it("handles decimals = 0", () => {
    expect(formatUnits("42", 0)).toBe("42");
  });

  it("handles negative amounts", () => {
    expect(formatUnits("-1500000000000000000", 18)).toBe("-1.5");
  });

  it("accepts bigint input directly", () => {
    expect(formatUnits(1500000000000000000n, 18)).toBe("1.5");
  });

  it("rejects invalid decimals", () => {
    expect(() => formatUnits("1", -1)).toThrow();
    expect(() => formatUnits("1", 1.5)).toThrow();
  });
});

describe("parseUnits", () => {
  it("parses a whole number", () => {
    expect(parseUnits("1", 18)).toBe("1000000000000000000");
  });

  it("parses a decimal amount", () => {
    expect(parseUnits("1.5", 18)).toBe("1500000000000000000");
  });

  it("parses a small fractional amount", () => {
    expect(parseUnits("0.000000000000000001", 18)).toBe("1");
  });

  it("round-trips through formatUnits", () => {
    const amounts = ["1", "1.5", "0.001", "999999.999999", "0"];
    for (const amount of amounts) {
      expect(formatUnits(parseUnits(amount, 18), 18)).toBe(amount === "0" ? "0" : amount);
    }
  });

  it("rejects more precision than the token supports", () => {
    expect(() => parseUnits("1.0000000000000000001", 18)).toThrow();
  });

  it("rejects malformed input", () => {
    expect(() => parseUnits("abc", 18)).toThrow();
    expect(() => parseUnits("", 18)).toThrow();
    expect(() => parseUnits("1.2.3", 18)).toThrow();
    expect(() => parseUnits("-", 18)).toThrow();
  });

  it("handles decimals = 0", () => {
    expect(parseUnits("42", 0)).toBe("42");
    expect(() => parseUnits("42.5", 0)).toThrow();
  });

  it("does not silently round the way `Number(amount) * 10**decimals` would", () => {
    // 0.1 * 10**18 in floating point is 100000000000000001.19... —
    // Number() rounds/serializes this unpredictably. BigInt-based
    // parsing must give the exact expected integer instead.
    expect(parseUnits("0.1", 18)).toBe("100000000000000000");
  });
});
