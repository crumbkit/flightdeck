// What the hooks' tests share: the fixture, parsed, and an address held in
// memory in place of the page's.
import { parseDocument } from "@flightdeck/headless/document";
import { useState } from "react";
import fixture from "../../../tests/fixtures/compliance/compliance.json" with { type: "json" };

/** Returns a fresh copy of the compliance build's fixture, parsed. */
export function fixtureDocument() {
  return parseDocument(fixture);
}

/** The fixture as the source reads it. */
export const fixtureJson: unknown = fixture;

/** Holds a page address in memory, starting at `initial`. */
export function useMemoryAddress(initial = "") {
  const [search, setSearch] = useState(initial);
  return { search, navigate: setSearch };
}
