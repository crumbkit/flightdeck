// The Rules page's hook (artboards 52 and 53). Its four filters and its sort
// live in the page address. It shows the first rules and a button for the
// rest, moving focus to the first rule the button reveals, and discloses the
// imposed set's rules and, on a phone, every report.

import type { ComplianceDocument } from "@crumbkit/flightdeck-headless/document";
import {
  CHECKERS,
  KINDS,
  type RulesView,
  rulesAddress,
  rulesView,
  SHOWN,
} from "@crumbkit/flightdeck-headless/rules";
import { type AddressSource, useAddressState, usePressed, useSortHeader } from "@crumbkit/ui/react";
import { type RefCallback, useEffect, useMemo, useRef } from "react";
import { useDisclosure } from "./disclosure.ts";

/** Returns a filter's chips: each option with its words, "All" for the option that keeps every rule. */
function chipsOf<const T extends string>(options: readonly T[]) {
  return options.map((option) => ({ option, label: option === "all" ? "All" : option }));
}

/** The Kind filter's chips. */
export const KIND_CHIPS = chipsOf(["all", ...KINDS]);

/** The Checker filter's chips. */
export const CHECKER_CHIPS = chipsOf(["all", ...CHECKERS]);

/** The Shown filter's chips. */
export const SHOWN_CHIPS = chipsOf(["all", ...SHOWN]);

/** The State filter's chips. */
export const STATE_CHIPS = [
  { option: "every", label: "Every state" },
  { option: "not_current", label: "Not current" },
] as const;

export interface UseRulesOverviewOptions {
  /** How many rules show before the reader asks for the rest; 12 by default. */
  readonly initialRows?: number;
}

/** Returns the Rules page of `document`, with its filters and sort held in `address`. */
export function useRulesOverview(
  document: ComplianceDocument,
  address: AddressSource,
  { initialRows = 12 }: UseRulesOverviewOptions = {},
) {
  const [value, change] = useAddressState(rulesAddress, address);
  const view: RulesView = useMemo(() => rulesView(document, value), [document, value]);
  const chips = {
    kind: usePressed({ mode: "one", value: value.kind, onChange: (kind) => change({ kind }) }),
    checker: usePressed({
      mode: "one",
      value: value.checker,
      onChange: (checker) => change({ checker }),
    }),
    shown: usePressed({ mode: "one", value: value.shown, onChange: (shown) => change({ shown }) }),
    state: usePressed({ mode: "one", value: value.state, onChange: (state) => change({ state }) }),
  };
  const sort = useSortHeader({
    sort: view.sort,
    onSort: (next) => change({ sort: next.key, direction: next.direction }),
    // Each column sorts worst state first, or names A to Z, when first chosen.
    firstDirection: () => "ascending",
  });
  const more = useDisclosure();
  const imposed = useDisclosure();
  const reports = useDisclosure();
  const firstRevealed = useRef<HTMLElement | null>(null);
  const revealing = useRef(false);
  useEffect(() => {
    if (more.open && revealing.current) {
      revealing.current = false;
      firstRevealed.current?.focus();
    }
  }, [more.open]);
  const hidden = Math.max(0, view.rules.length - initialRows);
  const rules = more.open ? view.rules : view.rules.slice(0, initialRows);
  const moreButton = more.getButtonProps();
  return {
    view,
    /** The rules the table shows now: the first `initialRows`, or all once the reader asks. */
    rules,
    /** How many rules the button would reveal; 0 when every rule shows. */
    hidden: more.open ? 0 : hidden,
    /** The button's words, such as "Show 29 more rules". */
    moreLabel: `Show ${hidden} more ${hidden === 1 ? "rule" : "rules"}`,
    getMoreButtonProps: () => ({
      ...moreButton,
      onClick: () => {
        revealing.current = !more.open;
        moreButton.onClick();
      },
    }),
    /** The props of the rule at `index` in `rules`; the first revealed rule takes focus. */
    getRuleRowProps: (index: number): { ref?: RefCallback<HTMLElement>; tabIndex?: -1 } =>
      index === initialRows
        ? {
            ref: (element) => {
              firstRevealed.current = element;
            },
            tabIndex: -1,
          }
        : {},
    getKindProps: chips.kind.getButtonProps,
    getCheckerProps: chips.checker.getButtonProps,
    getShownProps: chips.shown.getButtonProps,
    getStateProps: chips.state.getButtonProps,
    getHeaderProps: sort.getHeaderProps,
    getSortButtonProps: sort.getButtonProps,
    imposedOpen: imposed.open,
    getImposedToggleProps: imposed.getButtonProps,
    getImposedRegionProps: imposed.getRegionProps,
    reportsOpen: reports.open,
    getReportsToggleProps: reports.getButtonProps,
    getReportsRegionProps: reports.getRegionProps,
  };
}
