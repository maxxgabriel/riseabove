import { describe, expect, it } from "vitest";
import { brandFor, hashOf, hexToRgb, initials, readableOn, rgbToHsl, tintOf } from "./color";

describe("colours for crests and page tints", () => {
  it("pulls any colour in to a dark tint", () => {
    for (const c of ["#ff0000", "#ffffff", "#000000", "#00ff88", "#fdd835"]) {
      const [, s, l] = rgbToHsl(...hexToRgb(tintOf(c)));
      expect(l).toBeLessThan(0.3);
      expect(s).toBeLessThanOrEqual(0.56);
    }
  });
  it("reads dark ink on light badges and white on dark ones", () => {
    expect(readableOn("#ffffff")).toBe("#101412");
    expect(readableOn("#0d4b36")).toBe("#ffffff");
    expect(readableOn("#fdd835")).toBe("#101412");
  });
  it("makes the same colours for the same competition every time", () => {
    expect(brandFor("cup", 4, "Carabao Cup")).toEqual(brandFor("cup", 4, "Carabao Cup"));
    expect(hashOf("a")).not.toBe(hashOf("b"));
  });
  it("takes initials without club filler words", () => {
    expect(initials("FC Insi United")).toBe("IU");
    expect(initials("Vemar Athletic")).toBe("VA");
    expect(initials("Lober")).toBe("LO");
    expect(initials("N00 Cup")).toBe("N0");
  });
});
