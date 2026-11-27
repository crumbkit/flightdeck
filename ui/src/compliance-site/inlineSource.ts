// The source of the site's document: the `<script type="application/json">`
// element that `flightdeck compliance build` writes into compliance.html.
// Reading it makes no request.
import type { ComplianceSource } from "@crumbkit/flightdeck-headless/source";

/** The id of the element that holds the document, as src/compliance/site.rs writes it. */
export const DOCUMENT_ID = "compliance-document";

/** The page holds no document element, such as a page served from `pnpm dev`. */
export class NoDocument extends Error {
  readonly code = "no_document";

  constructor() {
    super("the page holds no compliance document; build it with flightdeck compliance build --out");
    this.name = "NoDocument";
  }
}

/** Returns the source that reads the document element of `page`. */
export function inlineSource(page: Pick<Document, "getElementById">): ComplianceSource {
  return {
    read: async () => {
      const element = page.getElementById(DOCUMENT_ID);
      if (element === null) throw new NoDocument();
      return JSON.parse(element.textContent ?? "");
    },
  };
}
