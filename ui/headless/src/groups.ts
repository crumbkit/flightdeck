// The groups of controls (artboard 58), such as Rust and Server, and the
// state each control and each group rolls up to over the repositories. A
// control's state in a repository is its cell's, except that a repository
// that overrides or lacks it drifts; a group's is its worst control's. The
// expanded group and the marked control live in the page address.
import { counted } from "@crumbkit/ui/a11y";
import { defineAddress, id } from "@crumbkit/ui/address";
import { rollUp } from "@crumbkit/ui/table";
import type { Cell, ComplianceDocument, Group, RepositoryColumn, Row } from "./document.ts";
import { driftsOn } from "./drift.ts";
import { ruleRows } from "./rules.ts";
import { andList, checkerWord, INVALID_REPORT, NO_REPORT, rowName, shortName } from "./words.ts";

/**
 * The value of the address's `group` when the reader has closed every
 * group. A group's id is a path, which starts with "/", so it never reads
 * as this word.
 */
const NO_GROUP = "none";

/**
 * The groups page's address: the expanded group and the marked control. An
 * address that names no group opens the first; `group=none` opens none.
 */
export const groupsAddress = defineAddress({ group: id(), rule: id() });

/** The values the groups page's address holds. */
export type GroupsAddress = ReturnType<typeof groupsAddress.parse>;

/** What a control reads in one repository, worst first: failing, then drifts, then stale, then no report. */
export type ControlState =
  | "failing"
  | "drifts"
  | "stale"
  | "no_report"
  | "not_readable"
  | "current"
  | "written";

/** The control states, worst first. */
const CONTROL_STATES: readonly ControlState[] = [
  "failing",
  "drifts",
  "stale",
  "no_report",
  "not_readable",
  "current",
  "written",
];

/** Returns a control state's word on its chip. */
function controlWord(state: ControlState): string {
  switch (state) {
    case "failing":
      return "failing";
    case "drifts":
      return "drifts";
    case "stale":
      return "stale";
    case "no_report":
      return "no report";
    case "not_readable":
      return "not readable";
    case "current":
      return "current";
    case "written":
      return "written";
  }
}

/** Returns a control state's word in a roll-up's counts, such as "1 drifting". */
function countWord(state: ControlState): string {
  return state === "drifts" ? "drifting" : controlWord(state);
}

/** Returns what a control reads in `column`, or null when it does not apply there. */
function controlState(
  row: Row,
  cell: Cell | undefined,
  column: RepositoryColumn,
): ControlState | null {
  if (cell === undefined || cell.state === "absent") return null;
  if (cell.code === NO_REPORT || cell.code === INVALID_REPORT) return "no_report";
  if (cell.state === "failing") return "failing";
  if (row.source === "imposed" && driftsOn(column, row.id)) return "drifts";
  switch (cell.state) {
    case "stale":
      return "stale";
    case "not_readable":
      return "not_readable";
    case "current":
      return "current";
    case "written":
      return "written";
  }
}

/** Returns a roll-up's words, worst first, such as "1 failing · 1 drifting · 1 current". */
function rollUpWords(states: readonly ControlState[]): string {
  return CONTROL_STATES.flatMap((state) => {
    const count = states.filter((each) => each === state).length;
    return count === 0 ? [] : [`${count} ${countWord(state)}`];
  }).join(" · ");
}

/** Returns a control's state in each repository it applies to. */
function statesOf(document: ComplianceDocument, row: Row): (ControlState | null)[] {
  return document.repositories.map((column, index) => controlState(row, row.cells[index], column));
}

/** One control in a group. */
export interface MemberEntry {
  readonly id: string;
  readonly short: string;
  readonly name: string;
  readonly checker: string;
  readonly rollUp: string;
  readonly state: ControlState;
  readonly word: string;
  /** The other groups it is in, each with the address that opens it with this control marked. */
  readonly alsoIn: readonly {
    readonly id: string;
    readonly title: string;
    readonly search: string;
  }[];
  readonly marked: boolean;
  readonly spoken: string;
}

/** One group and its controls. */
export interface GroupEntry {
  readonly id: string;
  readonly title: string;
  /** Such as "3 controls". */
  readonly controls: string;
  readonly rollUp: string;
  readonly state: ControlState;
  readonly word: string;
  readonly expanded: boolean;
  /** The address's `group` after the reader presses the group's button: the group, or none when it is open. */
  readonly toggleTo: string;
  /** The address that opens this group, or closes it when it is open. */
  readonly toggle: string;
  readonly members: readonly MemberEntry[];
  /** How many repositories a control of the group drifts in. */
  readonly drifting: number;
  readonly spoken: string;
}

/** Everything the groups page shows. */
export interface GroupsView {
  readonly headline: string;
  readonly groups: readonly GroupEntry[];
  /** How many controls the groups hold, each counted once however many groups list it. */
  readonly total: number;
  readonly marked: string | null;
  readonly note: string;
}

/** Returns each group's roll-up, with `address` saying which is expanded and which control is marked. */
export function groupEntries(document: ComplianceDocument, address: GroupsAddress): GroupEntry[] {
  const rules = new Map(ruleRows(document).map((row) => [row.id, row]));
  const titles = new Map(document.groups.map((group) => [group.id, group.title]));
  const open =
    address.group === NO_GROUP ? null : (address.group ?? document.groups[0]?.id ?? null);
  return document.groups.map((group: Group): GroupEntry => {
    const members = group.members.flatMap((member) => {
      const row = rules.get(member);
      return row === undefined ? [] : [row];
    });
    const memberStates = members.map((row) => statesOf(document, row));
    const repositoryStates = document.repositories.flatMap((_, index) => {
      const states = memberStates.flatMap((states) => {
        const state = states[index];
        return state === null || state === undefined ? [] : [state];
      });
      const worst = rollUp(states, CONTROL_STATES);
      return worst === undefined ? [] : [worst];
    });
    const drifting = document.repositories.filter((_, index) =>
      memberStates.some((states) => states[index] === "drifts"),
    ).length;
    const state = rollUp(repositoryStates, CONTROL_STATES) ?? "current";
    const expanded = group.id === open;
    const controls = counted(members.length, "control");
    const rolled = rollUpWords(repositoryStates);
    return {
      id: group.id,
      title: group.title,
      controls,
      rollUp: rolled,
      state,
      word: controlWord(state),
      expanded,
      toggleTo: expanded ? NO_GROUP : group.id,
      toggle: groupsAddress.format({ group: expanded ? NO_GROUP : group.id, rule: null }),
      members: members.map((row, at): MemberEntry => {
        const states = (memberStates[at] ?? []).flatMap((each) => (each === null ? [] : [each]));
        const memberState = rollUp(states, CONTROL_STATES) ?? "current";
        const short = shortName(row.id);
        const memberRollUp = rollUpWords(states);
        return {
          id: row.id,
          short,
          name: rowName(row),
          checker: checkerWord(row.checker, true),
          rollUp: memberRollUp,
          state: memberState,
          word: controlWord(memberState),
          alsoIn: row.groups
            .filter((other) => other !== group.id)
            .map((other) => ({
              id: other,
              title: titles.get(other) ?? other,
              search: groupsAddress.format({ group: other, rule: row.id }),
            })),
          marked: expanded && address.rule === row.id,
          spoken: `${short}: ${controlWord(memberState)}, ${memberRollUp}`,
        };
      }),
      drifting,
      spoken: `${group.title}, ${controls}: ${controlWord(state)}, ${rolled}`,
    };
  });
}

/** Returns the groups page of `document` with the group and control `address` names. */
export function groupsView(document: ComplianceDocument, address: GroupsAddress): GroupsView {
  const groups = groupEntries(document, address);
  const failing = groups.filter((group) => group.state === "failing").length;
  let headline: string;
  if (groups.length === 0) headline = "No report names a group of rules";
  else if (failing === groups.length) headline = "Each group has a failing control";
  else if (failing === 0) headline = "No group has a failing control";
  else headline = `${failing} of ${groups.length} groups have a failing control`;
  const drifting = groups.filter((group) => group.drifting > 0);
  const only = drifting[0];
  if (drifting.length === 1 && only !== undefined) {
    headline += `, and ${only.title} also drifts in ${counted(only.drifting, "repository", "repositories")}`;
  } else if (drifting.length > 1) {
    headline += `, and ${andList(drifting.map((group) => group.title))} also drift`;
  }
  // A control can sit in several groups; the total counts each control
  // the groups list once.
  const shown = new Set(groups.flatMap((group) => group.members.map((member) => member.id)));
  return {
    headline,
    groups,
    total: shown.size,
    marked: groups.some((group) => group.members.some((member) => member.marked))
      ? address.rule
      : null,
    note: "Groups come from each report's rules. A control in two groups shows in each of them, and counts once.",
  };
}
