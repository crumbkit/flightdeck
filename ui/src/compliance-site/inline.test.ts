import { describe, expect, it } from "vitest";
import { inlinePage, scriptText, styleText } from "./inline.ts";

describe("inlining the Compliance site", () => {
  it("writes a closing script tag inside the code so it cannot end the element", () => {
    expect(scriptText('const end = "</script>"; const END = "</SCRIPT>";')).toBe(
      'const end = "<\\/script>"; const END = "<\\/SCRIPT>";',
    );
  });

  it("writes a comment opener inside the code so the parser never leaves the script", () => {
    expect(scriptText('html("<!--$")')).toBe('html("<\\!--$")');
  });

  it("writes a closing style tag inside the styles so it cannot end the element", () => {
    expect(styleText('a::after{content:"</style>"}')).toBe('a::after{content:"<\\/style>"}');
  });

  it("puts each script and stylesheet the page names inside the page", () => {
    const html = [
      "<head>",
      '<script type="module" crossorigin src="/compliance/site.js"></script>',
      '<link rel="stylesheet" crossorigin href="/compliance/site.css">',
      "</head>",
    ].join("");
    const page = inlinePage(html, (name) => {
      if (name === "compliance/site.js") return { kind: "script", text: "run()" };
      if (name === "compliance/site.css") return { kind: "style", text: "a{}" };
      return null;
    });
    expect(page).toBe('<head><script type="module">run()</script><style>a{}</style></head>');
  });

  it("refuses a page that names a file the build did not write", () => {
    expect(() =>
      inlinePage('<script type="module" src="/elsewhere.js"></script>', () => null),
    ).toThrow("compliance.html names /elsewhere.js, which the build did not write");
  });
});
