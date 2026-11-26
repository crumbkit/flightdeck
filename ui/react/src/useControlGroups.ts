// The groups page's hook (artboard 58). The expanded group and the marked
// control live in the page address. Following a control to another group
// it is in opens that group with the control marked, and moves focus to it.
import { type AddressSource, useAddressState } from "@crumbkit/ui/react";
import type { ComplianceDocument } from "@flightdeck/headless/document";
import {
  type GroupEntry,
  type GroupsView,
  groupsAddress,
  groupsView,
  type MemberEntry,
} from "@flightdeck/headless/groups";
import { type RefCallback, useCallback, useEffect, useId, useMemo, useRef } from "react";

/** The props of the button that expands or collapses a group. */
export interface GroupToggleProps {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string;
  readonly onClick: () => void;
}

/** The props of a control's row. */
export interface MemberRowProps {
  readonly tabIndex: -1;
  readonly "aria-current": "true" | undefined;
  readonly ref: RefCallback<HTMLElement>;
}

/** The props of a link to another group a control is in. */
export interface AlsoInProps {
  readonly href: string;
  readonly onClick: (event: { preventDefault: () => void }) => void;
}

/** Returns the groups page of `document`, with the expanded group and marked control held in `address`. */
export function useControlGroups(document: ComplianceDocument, address: AddressSource) {
  const [value, change] = useAddressState(groupsAddress, address);
  const view: GroupsView = useMemo(() => groupsView(document, value), [document, value]);
  const base = useId();
  const rows = useRef(new Map<string, HTMLElement>());
  // Set when the reader follows a control to another group, so focus moves
  // only then and not when the page first opens.
  const following = useRef(false);
  const marked = view.marked;
  const open = view.groups.find((group) => group.expanded)?.id ?? null;
  useEffect(() => {
    if (!following.current || marked === null || open === null) return;
    following.current = false;
    rows.current.get(`${open} ${marked}`)?.focus();
  }, [marked, open]);
  const rowRef = useCallback(
    (key: string): RefCallback<HTMLElement> =>
      (element) => {
        if (element === null) rows.current.delete(key);
        else rows.current.set(key, element);
      },
    [],
  );
  return {
    view,
    getGroupToggleProps: (group: GroupEntry): GroupToggleProps => ({
      type: "button",
      "aria-expanded": group.expanded,
      "aria-controls": `${base}-${group.id}`,
      onClick: () => change({ group: group.toggleTo, rule: null }),
    }),
    /** The props of the element that holds a group's controls. */
    getGroupRegionProps: (group: GroupEntry) => ({
      id: `${base}-${group.id}`,
      hidden: !group.expanded,
    }),
    getMemberRowProps: (group: GroupEntry, member: MemberEntry): MemberRowProps => ({
      tabIndex: -1,
      "aria-current": member.marked ? "true" : undefined,
      ref: rowRef(`${group.id} ${member.id}`),
    }),
    getAlsoInProps: (member: MemberEntry, other: MemberEntry["alsoIn"][number]): AlsoInProps => ({
      href: other.search === "" ? "?" : other.search,
      onClick: (event) => {
        event.preventDefault();
        following.current = true;
        change({ group: other.id, rule: member.id });
      },
    }),
  };
}
