// The Compliance page's hook (artboards 46, 54 and 56). It holds the Show
// filter in the page address, the matrix's keyboard grid and the written
// rules' disclosure, and returns the view with prop getters. It renders
// nothing, so the app draws every element in its own livery.
import { type AddressSource, useAddressState, useGrid, usePressed } from "@crumbkit/ui/react";
import {
  type ComplianceAddress,
  type ComplianceView,
  complianceAddress,
  complianceView,
  SHOW_WORDS,
  type Tile,
} from "@flightdeck/headless/compliance";
import type { ComplianceDocument } from "@flightdeck/headless/document";
import { useId, useMemo } from "react";
import { useDisclosure } from "./disclosure.ts";

/** One of the lists a tile links to. */
export type ListKey = NonNullable<Tile["target"]>;

/** Returns the Compliance page of `document`, with its filter held in `address`. */
export function useCompliance(document: ComplianceDocument, address: AddressSource) {
  const [value, change] = useAddressState(complianceAddress, address);
  const view: ComplianceView = useMemo(() => complianceView(document, value), [document, value]);
  const show = usePressed<ComplianceAddress["show"]>({
    mode: "one",
    value: value.show,
    onChange: (next) => change({ show: next }),
  });
  const written = useDisclosure();
  // The grid holds every row the matrix shows, the written rules once they
  // are open, and a column for the name, the checker and each repository.
  const rows = useMemo(
    () => [
      ...view.sections.flatMap((section) => section.rows),
      ...(written.open ? view.written.rows : []),
    ],
    [view, written.open],
  );
  const grid = useGrid({ rows: rows.length, columns: view.columns.length + 2 });
  const base = useId();
  const listId = (list: ListKey) => `${base}-${list}`;
  return {
    view,
    /** The rows of the grid, in order: the sections' rows, then the written rules when open. */
    rows,
    /** The Show filter's chips, "Every state" and "Not current". */
    showOptions: (["every", "not_current"] as const).map((option) => ({
      option,
      label: SHOW_WORDS[option],
    })),
    getShowProps: show.getButtonProps,
    /** The link from a tile to its list, or null for a tile with none. */
    getTileProps: (tile: Tile) =>
      tile.target === null ? null : { href: `#${listId(tile.target)}` },
    getListProps: (list: ListKey) => ({ id: listId(list) }),
    getGridProps: grid.getGridProps,
    getRowProps: grid.getRowProps,
    /** A cell's props, by its row in `rows` and its column: 0 the name, 1 the checker, then each repository. */
    getCellProps: grid.getCellProps,
    writtenOpen: written.open,
    getWrittenToggleProps: written.getButtonProps,
    getWrittenRegionProps: written.getRegionProps,
  };
}
