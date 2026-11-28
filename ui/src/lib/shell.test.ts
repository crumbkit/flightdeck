import { describe, expect, it } from "vitest";
import { boardGet, shellWord } from "./shell";

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

describe("boardGet", () => {
  it("gives the command that reads a path, quoted for a POSIX shell", () => {
    expect(boardGet("/flows/development/3029fae590f5a628")).toBe(
      "board get /flows/development/3029fae590f5a628",
    );
    expect(boardGet("/changes/a b")).toBe("board get '/changes/a b'");
  });

  it("gives no command for a path holding a C0, DEL or C1 control character", () => {
    for (const path of ["/a\u001b[201~b", "/a\nb", "/a\u0000b", "/a\u007fb", "/a\u009bb"]) {
      expect(boardGet(path)).toBeNull();
    }
  });
});
