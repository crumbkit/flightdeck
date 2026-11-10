import { describe, expect, it } from "vitest";
import { counted } from "./words";

describe("counted", () => {
  it("names one in the singular and groups the thousands of any other count", () => {
    expect(counted(1, "run")).toBe("1 run");
    expect(counted(0, "run")).toBe("0 runs");
    expect(counted(1384, "row")).toBe("1,384 rows");
    expect(counted(2, "match", "matches")).toBe("2 matches");
  });
});
