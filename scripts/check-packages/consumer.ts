// A front end that adopts the packages as their docs say: it supplies its
// own ComplianceSource, reads the document through the core and renders the
// Compliance page's headline and tile counts with the React hook, in its
// own markup. check.sh type-checks it against the installed tarballs and
// runs it under Node, which prints the page's HTML.
//
// Usage: node consumer.ts <compliance.json>
import { readFile } from "node:fs/promises";
import type { ComplianceDocument } from "@crumbkit/flightdeck-headless/document";
import { type ComplianceSource, readDocument } from "@crumbkit/flightdeck-headless/source";
import { useCompliance } from "@crumbkit/flightdeck-react/useCompliance";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

const path = process.argv[2];
if (path === undefined) throw new Error("usage: node consumer.ts <compliance.json>");

const source: ComplianceSource = {
  read: async () => JSON.parse(await readFile(path, "utf8")),
};

// The page address of a server render: no query string, and nowhere to go.
const address = { search: "", navigate: () => {} };

function CompliancePage({ document }: { document: ComplianceDocument }) {
  const page = useCompliance(document, address);
  return createElement(
    "main",
    null,
    createElement("h1", null, page.view.title),
    createElement("p", null, page.view.headline),
    createElement(
      "ul",
      null,
      page.view.tiles.map((tile) => createElement("li", { key: tile.key }, `${tile.label}: ${tile.count}`)),
    ),
  );
}

const document = await readDocument(source, new AbortController().signal);
console.log(renderToStaticMarkup(createElement(CompliancePage, { document })));
