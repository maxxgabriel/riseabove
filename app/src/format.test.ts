import { describe, expect, it } from "vitest";
import { ageAt, dateIn, duration, money, ordinal, prose, relativeDays } from "./format";

// 2026-07-01 as days since 1970.
const JUL_1_2026 = Date.UTC(2026, 6, 1) / 86_400_000;

describe("money", () => {
  it("shows small amounts in full and larger ones compactly", () => {
    expect(money(350)).toBe("£350");
    expect(money(9_999)).toBe("£9,999");
    expect(money(12_500)).toBe("£12.5k");
    expect(money(2_035_000)).toBe("£2.04m");
    expect(money(11_300_000)).toBe("£11.3m");
    expect(money(1_250_000_000)).toBe("£1.25bn");
  });
  it("can show exact figures and signs", () => {
    expect(money(2_035_000, { exact: true })).toBe("£2,035,000");
    expect(money(-40_000)).toBe("−£40k");
    expect(money(5_000, { sign: true })).toBe("+£5,000");
    expect(money(null)).toBe("");
  });
});

describe("dates", () => {
  it("formats in each style", () => {
    expect(dateIn("short", JUL_1_2026)).toBe("1 Jul 2026");
    expect(dateIn("iso", JUL_1_2026)).toBe("2026-07-01");
    expect(dateIn("us", JUL_1_2026)).toBe("07/01/2026");
    expect(dateIn("short", JUL_1_2026, false)).toBe("1 Jul");
  });
  it("rewrites ISO dates inside sentences", () => {
    expect(prose("Your team played on 2026-08-08. Reply by 2026-08-24.")).toBe("Your team played on 8 Aug 2026. Reply by 24 Aug 2026.");
    expect(prose("No dates here, only 2026 and 12-13.")).toBe("No dates here, only 2026 and 12-13.");
  });
  it("works out ages and distances", () => {
    const dob = Date.UTC(2004, 10, 21) / 86_400_000;
    expect(ageAt(dob, JUL_1_2026)).toBe(21);
    expect(ageAt(dob, Date.UTC(2026, 10, 21) / 86_400_000)).toBe(22);
    expect(relativeDays(10, 10)).toBe("today");
    expect(relativeDays(11, 10)).toBe("tomorrow");
    expect(relativeDays(3, 10)).toBe("7 days ago");
  });
});

describe("words", () => {
  it("writes ordinals", () => {
    expect([1, 2, 3, 4, 11, 12, 13, 21, 22, 101, 111].map(ordinal)).toEqual(["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "101st", "111th"]);
  });
  it("describes durations in the unit that reads best", () => {
    expect(duration(1)).toBe("1 day");
    expect(duration(10)).toBe("10 days");
    expect(duration(60)).toBe("9 weeks");
    expect(duration(400)).toBe("13 months");
    expect(duration(1100)).toBe("3.0 years");
    expect(duration(-3)).toBe("ended");
  });
});
