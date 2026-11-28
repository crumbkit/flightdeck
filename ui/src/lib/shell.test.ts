import { describe, expect, it } from "vitest";
import { shellWord } from "./shell";

describe("shellWord", () => {
  it("leaves a plain board path as it is", () => {
    expect(shellWord("/flows/development/3029fae590f5a628")).toBe(
      "/flows/development/3029fae590f5a628",
    );
  });

  it("quotes a path holding a space, a semicolon or a newline", () => {
    expect(shellWord("/changes/a b;rm -rf ~")).toBe("'/changes/a b;rm -rf ~'");
    expect(shellWord("/a\nb")).toBe("'/a\nb'");
  });

  it("writes a single quote so that it ends and restarts the quoting", () => {
    expect(shellWord("/it's")).toBe("'/it'\\''s'");
  });

  it("quotes an empty text", () => {
    expect(shellWord("")).toBe("''");
  });
});
