import { describe, expect, it } from "vitest";
import { durationWords, elapsedWords } from "./duration";

describe("durationWords", () => {
  it("words hours and minutes as a person reads them", () => {
    expect(durationWords("PT1H40M")).toBe("1 h 40 min");
  });

  it("words every unit from years to seconds in order", () => {
    expect(durationWords("P1Y2M3W4DT5H6M7.5S")).toBe("1 y 2 mo 3 w 4 d 5 h 6 min 7.5 s");
  });

  it("words a zero duration as zero seconds", () => {
    expect(durationWords("PT0S")).toBe("0 s");
  });

  it("reads no duration in a string that is not one", () => {
    expect(durationWords("P")).toBeNull();
    expect(durationWords("PT")).toBeNull();
    expect(durationWords("Pride and Prejudice")).toBeNull();
    expect(durationWords("1H40M")).toBeNull();
  });
});

describe("elapsedWords", () => {
  it("words a run's duration in milliseconds below a second and seconds above", () => {
    expect(elapsedWords(412)).toBe("412 ms");
    expect(elapsedWords(2412)).toBe("2.4 s");
  });
});
