import { describe, expect, it } from "vitest";
import { href, refPath } from "./router";

describe("router paths", () => {
  it("builds hash links", () => {
    expect(href("/people")).toBe("#/people");
  });
  it("sends each kind of reference to its page", () => {
    expect(refPath({ k: "person", id: 7 })).toBe("/person/7");
    expect(refPath({ k: "club", id: 3 })).toBe("/club/3");
    expect(refPath({ k: "comp", id: 0 })).toBe("/comp/0");
    expect(refPath({ k: "nation", id: 1 })).toBe("/nation/1");
    expect(refPath({ k: "match", id: 12 })).toBe("/match/12");
    expect(refPath({ k: "inst", id: 4 })).toBe("/institution/4");
  });
});
