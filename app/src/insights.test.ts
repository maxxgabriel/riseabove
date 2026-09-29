import { describe, expect, it } from "vitest";
import { splitOnName } from "./components/Insights";

describe("splitOnName", () => {
  it("splits a sentence around the first mention", () => {
    expect(splitOnName("T. Kavona has 9 of the 21 goals, T. Kavona leads", "T. Kavona")).toEqual(["", " has 9 of the 21 goals, T. Kavona leads"]);
  });
  it("gives null when the name is not in the sentence", () => {
    expect(splitOnName("Nobody is named here", "T. Kavona")).toBeNull();
    expect(splitOnName("Anything", "")).toBeNull();
  });
});
