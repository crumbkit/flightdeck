// The build step that makes compliance.html one self-contained file: it puts
// the script and the stylesheet that Vite writes for the page inside the page
// and drops them from the output, so the page loads no other file. A Flare
// share is one HTML file, and its frame blocks every request the page makes.
import type { Plugin } from "vite";

/** A file the page names, as the build wrote it. */
export interface Inlined {
  readonly kind: "script" | "style";
  readonly text: string;
}

/**
 * Returns `code` as the text of a script element. A closing script tag in
 * the code, which a string can hold, would end the element, so it gets a
 * backslash that a JavaScript string reads as nothing. A comment opener can
 * make the parser read a later closing tag as text, and no escape keeps
 * every place it can appear, such as a regular expression or a raw
 * template, meaning the same, so the build refuses code that holds one and
 * a person decides.
 */
export function scriptText(code: string): string {
  if (code.includes("<!--")) {
    throw new Error(
      "the compliance page's script holds <!--, which can keep its script element open; find the module that writes it",
    );
  }
  return code.replaceAll(/<\/(script)/gi, "<\\/$1");
}

/** Returns `css` as the text of a style element, with each closing style tag escaped as a CSS string reads it. */
export function styleText(css: string): string {
  return css.replaceAll(/<\/(style)/gi, "<\\/$1");
}

const SCRIPT = /<script type="module"(?: crossorigin)? src="\/([^"]+)"><\/script>/g;
const STYLESHEET = /<link rel="stylesheet"(?: crossorigin)? href="\/([^"]+)">/g;

/**
 * Returns `html` with each module script and stylesheet it names replaced by
 * the file's text, which `read` returns for a path such as
 * "compliance/site.js". It throws for a file `read` does not know, so the
 * page never names a file that is not inside it.
 */
export function inlinePage(html: string, read: (name: string) => Inlined | null): string {
  const take = (name: string, kind: Inlined["kind"]): string => {
    const file = read(name);
    if (file === null || file.kind !== kind) {
      throw new Error(`compliance.html names /${name}, which the build did not write`);
    }
    return file.text;
  };
  return html
    .replaceAll(SCRIPT, (_, name: string) => {
      return `<script type="module">${scriptText(take(name, "script"))}</script>`;
    })
    .replaceAll(
      STYLESHEET,
      (_, name: string) => `<style>${styleText(take(name, "style"))}</style>`,
    );
}

/**
 * The plugin that builds compliance.html, for `vite build --mode
 * compliance`: one entry with no code splitting, no files from public/
 * such as fonts, and the script and styles inlined by `inlinePage`. The
 * app's own build, in every other mode, is left as it is.
 */
export function complianceSite(): Plugin {
  let active = false;
  return {
    name: "flightdeck-compliance-site",
    config(config, { mode }) {
      active = mode === "compliance";
      if (!active) return;
      config.publicDir = false;
      config.build = {
        ...config.build,
        emptyOutDir: false,
        cssCodeSplit: false,
        modulePreload: false,
        assetsInlineLimit: Number.MAX_SAFE_INTEGER,
        rolldownOptions: {
          input: "compliance.html",
          output: {
            codeSplitting: false,
            entryFileNames: "compliance/site.js",
            assetFileNames: "compliance/site[extname]",
          },
        },
      };
    },
    // Vite writes the page's HTML after the bundle's other hooks, so the
    // inlining runs as its last HTML transform, which sees the bundle.
    transformIndexHtml: {
      order: "post",
      handler(html, context) {
        const bundle = context.bundle;
        if (!active || bundle === undefined) return html;
        const read = (name: string): Inlined | null => {
          const file = bundle[name];
          if (file === undefined) return null;
          delete bundle[name];
          if (file.type === "chunk") return { kind: "script", text: file.code };
          const text =
            typeof file.source === "string" ? file.source : new TextDecoder().decode(file.source);
          return { kind: "style", text };
        };
        return inlinePage(html, read);
      },
    },
    // After Vite writes the page, nothing else is left in the bundle.
    generateBundle: {
      order: "post",
      handler(_, bundle) {
        if (!active) return;
        const left = Object.keys(bundle).filter((name) => name !== "compliance.html");
        if (left.length > 0) {
          throw new Error(
            `the compliance build wrote files the page does not hold: ${left.join(", ")}`,
          );
        }
      },
    },
  };
}
