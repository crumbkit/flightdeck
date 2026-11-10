import type { Tool } from "@/api/model";
import type { Page, WeightReply } from "@/api/weight";
import { grouped } from "./cost";
import { counted } from "./words";

/**
 * crumbkit's default budgets, in bytes: a command's whole path, the tool's
 * own help page, a group's page and a command's page.
 *
 * RUST-10: a tool may lower its own budgets, which `commands list` does not
 * export, so every tool is judged against the defaults and the page labels
 * them "default budget". A `commands list` that names the tool's budgets
 * replaces these.
 */
export const budgets = { path: 8000, root: 1500, group: 800, command: 1200 } as const;

/** What a step of a path is: a help page, or the command's output. */
export type StepKind = "root" | "group" | "command" | "output";

/**
 * One step of a command's path: its label, such as "shelf books -h", its
 * bytes or null when it was not measured, its page budget or null for the
 * output, and a note on a step without bytes or an output without runs.
 */
export type Step = {
  label: string;
  kind: StepKind;
  bytes: number | null;
  budget: number | null;
  note: string | null;
};

/**
 * One command's path: its steps, the sum of their bytes, whether a page
 * was not measured, so the sum leaves it out, and its heaviest step.
 */
export type WeightPath = {
  command: string;
  steps: Step[];
  total: number;
  partial: boolean;
  heaviest: Step | null;
};

/** Returns why `page` has no bytes, in words. */
export function failureWords(page: Page): string {
  switch (page.failure) {
    case "exit":
      return `not measured: exited with ${page.exit}`;
    case "signal":
      return `not measured: ended by signal ${page.exit}`;
    case "empty":
      return "not measured: printed nothing";
    case "timed_out":
      return "not measured: timed out";
    case "too_large":
      return "over 64 KiB, not a help page";
    case "not_started":
      return "not measured: would not start";
    case "not_run":
      return "not measured: flightdeck stopped measuring before this page, at its 30 s limit or its own stop";
    case null:
      return "";
  }
}

/** The budget of each kind of help page. */
const pageBudgets = { root: budgets.root, group: budgets.group, command: budgets.command } as const;

/** Returns the help-page step of `path` from the measured `pages`. */
function pageStep(
  tool: Tool,
  pages: Map<string, Page>,
  path: string[],
  kind: keyof typeof pageBudgets,
): Step {
  const label = [tool.name, ...path, "-h"].join(" ");
  const budget = pageBudgets[kind];
  const page = pages.get(path.join(" "));
  if (page === undefined) {
    return { label, kind, bytes: null, budget, note: "not measured: past the 400-page cap" };
  }
  return page.bytes === null
    ? { label, kind, bytes: null, budget, note: failureWords(page) }
    : { label, kind, bytes: page.bytes, budget, note: null };
}

/** Returns the output step of `command`: the p95 of its real output, or why it has none. */
function outputStep(command: Tool["commands"][number]): Step {
  const step = { label: "output", kind: "output" as const, budget: null };
  if (!command.counted) {
    return { ...step, bytes: 0, note: "not counted" };
  }
  const figures = command.cost?.figures ?? null;
  if (figures === null || figures.runs === 0) {
    return { ...step, bytes: 0, note: "no runs, so no output weight" };
  }
  if (command.prints_no_result) {
    // crumbkit's figures report no output size for a command that prints
    // no result, because it prints nothing.
    return { ...step, bytes: 0, note: "prints no result" };
  }
  if (figures.p95_out_bytes === null) {
    // The command ran, but the figures do not say how much it printed, so
    // the path's sum is a lower bound.
    return { ...step, bytes: null, note: "output size unknown" };
  }
  return { ...step, bytes: figures.p95_out_bytes, note: null };
}

/**
 * Returns each command's path through the tool's help: the tool's own
 * page, each group page above the command, the command's own page, and
 * the 95th percentile of its real output.
 */
export function weightPaths(tool: Tool, reply: WeightReply): WeightPath[] {
  const pages = new Map(reply.pages.map((page) => [page.path.join(" "), page]));
  return tool.commands.map((command) => {
    const words = command.path;
    const steps: Step[] = [
      pageStep(tool, pages, [], "root"),
      ...words
        .slice(0, -1)
        .map((_, index) => pageStep(tool, pages, words.slice(0, index + 1), "group")),
      pageStep(tool, pages, words, "command"),
      outputStep(command),
    ];
    const heaviest = steps.reduce<Step | null>(
      (best, step) =>
        step.bytes !== null && (best === null || step.bytes > (best.bytes ?? 0)) ? step : best,
      null,
    );
    return {
      command: words.join(" "),
      steps,
      total: steps.reduce((sum, step) => sum + (step.bytes ?? 0), 0),
      partial: steps.some((step) => step.bytes === null),
      heaviest,
    };
  });
}

/** Returns the tokens that `bytes` cost an agent, at four bytes a token. */
export function tokens(bytes: number): number {
  return Math.round(bytes / 4);
}

/**
 * Returns `bytes` against `budget` in words: "over by 120 B" or "880 B
 * under", or "partial" for a sum that leaves out a page, which is never
 * called over or under.
 */
export function overWords(bytes: number, budget: number, partial = false): string {
  if (partial) {
    return "partial";
  }
  return bytes > budget
    ? `over by ${grouped(bytes - budget)} B`
    : `${grouped(budget - bytes)} B under`;
}

/** Returns whether a whole path is over the path budget. A partial path never is. */
export function isOver(path: WeightPath): boolean {
  return !path.partial && path.total > budgets.path;
}

/** A column the weight table sorts by. */
export type WeightKey = "command" | "bytes";

/** Returns `paths` sorted by `key` in `direction`, with ties in name order. */
export function sortedPaths(
  paths: WeightPath[],
  key: WeightKey,
  direction: "ascending" | "descending",
): WeightPath[] {
  const sign = direction === "ascending" ? 1 : -1;
  return [...paths].sort((a, b) => {
    const byName = a.command.localeCompare(b.command);
    return key === "command" ? sign * byName : sign * (a.total - b.total) || byName;
  });
}

/**
 * Returns the weight tab's summary, such as "19 commands from 28 help
 * pages measured when this view opened · 4 paths over budget", with the
 * commands left out past the server's cap when there are any.
 */
export function weightSummary(paths: WeightPath[], reply: WeightReply): string {
  const over = paths.filter(isOver).length;
  const parts = [
    `${counted(paths.length, "command")} from ${counted(reply.pages.length, "help page")} measured when this view opened`,
    `${counted(over, "path")} over budget`,
  ];
  if (reply.left_out > 0) {
    parts.push(`${counted(reply.left_out, "command")} left out past the 400-page cap`);
  }
  return parts.join(" · ");
}
