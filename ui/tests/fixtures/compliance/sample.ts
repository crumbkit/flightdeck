// The sample document the Compliance site's tests draw: the repositories,
// rules and suites of the signed artboards 52, 54 and 55, as a compliance
// build would write them.
import { type ComplianceDocument, parseDocument } from "@crumbkit/flightdeck-headless/document";
import sample from "./sample.json" with { type: "json" };

/** The sample as compliance.json holds it. */
export const sampleJson: unknown = sample;

/** Returns a fresh copy of the sample, parsed. */
export function sampleDocument(): ComplianceDocument {
  return parseDocument(structuredClone(sample));
}
