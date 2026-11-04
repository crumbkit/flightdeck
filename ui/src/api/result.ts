import { z } from "@/api/zod";
import type { Row } from "@/lib/table";
import { FailureDocument } from "./client";
import type { Command } from "./model";

/**
 * A failed item of a batch, such as one id that `books show` could not
 * find, or a failure line of a stream. A batch's failure carries the key
 * the caller gave and that key's place among the keys, counted from 0.
 */
const ItemFailure = FailureDocument.extend({
  message: z.string(),
  key: z.string().optional(),
  index: z.number().int().nonnegative().optional(),
});
export type ItemFailure = z.infer<typeof ItemFailure>;

const RowSchema = z.record(z.string(), z.unknown());

/** One command of a next step: a call of this tool, or a line of another program. */
const StepCommand = z.union([z.object({ call: z.string() }), z.object({ run: z.string() })]);

/** A next step that a result proposes, with the sentence its text prints. */
const Step = z.object({ text: z.string(), commands: z.array(StepCommand) });
export type Step = z.infer<typeof Step>;

/** crumbkit's envelope around a result: its data and what it says about it. */
const Envelope = z.object({
  schema: z.string(),
  data: z.unknown(),
  total: z.number().nullish(),
  truncated: z.boolean().optional(),
  failures: z.array(ItemFailure).optional(),
  // A step of a shape the page does not know drops every step, so a newer
  // tool's result still draws.
  next: z.array(Step).optional().catch(undefined),
});

/** One line of a stream: an item in crumbkit's envelope. */
const StreamItem = z.object({ schema: z.string(), data: RowSchema });

/** A row of a result, or a failure in the place of a row. */
export type Entry = { kind: "row"; row: Row } | { kind: "failure"; failure: ItemFailure };

/** How a card draws a result. */
export type Result =
  | {
      kind: "table";
      entries: Entry[];
      listing: { total: number | null; truncated: boolean } | null;
    }
  /** One row, with the failures of its batch in their places. */
  | { kind: "record"; entries: Entry[] }
  | { kind: "prose"; text: string }
  | { kind: "other" }
  | { kind: "none" };

/** What a run's output holds: its JSON, how to draw it, and the steps it proposes. */
export type Output = { document: unknown; result: Result; next: Step[] };

function parse(line: string): unknown {
  try {
    return JSON.parse(line);
  } catch {
    return undefined;
  }
}

/**
 * Returns the text of the last document in `lines`: the last line that
 * starts with `{` and every line after it, or every line when none starts
 * with `{`. A one-line document is its own line. A crumbkit tool that sets
 * `JsonLayout::Pretty` prints a document across many lines, which start
 * with a `{` line and indent every line after it but the closing `}`. The
 * server's runner finds a run's last document the same way.
 *
 * This is a shortcut. It finds the start of a document from the layout that
 * crumbkit prints, so it reads both layouts without knowing the tool's, and
 * misses a document that does not start with `{` at the start of a line. A
 * parser that reads every JSON value of the output whatever its layout
 * replaces it if a tool prints another layout.
 */
function lastDocument(lines: string[]): string | undefined {
  if (lines.length === 0) {
    return undefined;
  }
  const start = lines.findLastIndex((line) => line.startsWith("{"));
  return lines.slice(Math.max(start, 0)).join("\n");
}

/**
 * Reads a run's lines of standard output as the command's `form` says.
 *
 * A document command prints one document last, on one line or indented
 * across many. A listing becomes a table, one row or an object becomes a
 * record, and text becomes prose. `document` is that document's JSON, for
 * the card's JSON view, or null when it is not JSON.
 *
 * A stream command prints each item on its own line, in crumbkit's
 * envelope, and a failure document in the place of an item that failed.
 * Every line it has printed so far becomes the table, and `document` is
 * the list of the lines' JSON.
 */
export function resultOf(lines: string[], form: Command["result"]["form"]): Output {
  if (form === "stream") {
    return streamReader()(lines);
  }
  const last = lastDocument(lines);
  const document = last === undefined ? undefined : parse(last);
  if (document === undefined) {
    return { document: null, result: { kind: "none" }, next: [] };
  }
  const envelope = Envelope.safeParse(document);
  if (!envelope.success) {
    return { document, result: { kind: "other" }, next: [] };
  }
  return { document, result: drawn(envelope.data), next: envelope.data.next ?? [] };
}

/** Returns the rows of `entries`, without their failures. */
export function rowsOf(entries: Entry[]): Row[] {
  return entries.flatMap((entry) => (entry.kind === "row" ? [entry.row] : []));
}

/** The entry of one line of a stream: a row, a failure, or none. */
function streamEntryOf(value: unknown): Entry[] {
  const item = StreamItem.safeParse(value);
  if (item.success) {
    return [{ kind: "row", row: item.data.data }];
  }
  const failure = ItemFailure.safeParse(value);
  return failure.success ? [{ kind: "failure", failure: failure.data }] : [];
}

/**
 * Returns a reader of one run's stream, which `resultOf` describes. A
 * running card reads its lines again on every new line, so the reader
 * keeps what it parsed and parses only the lines added since its last
 * read. Lines fewer than it has read start it again.
 */
export function streamReader(): (lines: string[]) => Output {
  let values: unknown[] = [];
  let entries: Entry[] = [];
  return (lines) => {
    if (lines.length < values.length) {
      values = [];
      entries = [];
    }
    if (lines.length > values.length) {
      const added = lines.slice(values.length).map(parse);
      values = [...values, ...added];
      entries = [...entries, ...added.flatMap(streamEntryOf)];
    }
    return { document: values, result: { kind: "table", entries, listing: null }, next: [] };
  };
}

/**
 * Returns the rows and failures of a batch in the order the caller gave
 * the keys. Each failure carries its key's index, and the rows fill the
 * other places in order, because a batch answers its keys in the order
 * given. A failure without an index goes after the rows.
 */
function inKeyOrder(rows: Row[], failures: ItemFailure[]): Entry[] {
  const placed = failures
    .filter((failure) => failure.index !== undefined)
    .sort((a, b) => (a.index ?? 0) - (b.index ?? 0));
  const unplaced = failures.filter((failure) => failure.index === undefined);
  const entries: Entry[] = [];
  let row = 0;
  let failure = 0;
  while (row < rows.length || failure < placed.length) {
    const next = placed[failure];
    const item = rows[row];
    if (next !== undefined && (next.index === entries.length || item === undefined)) {
      entries.push({ kind: "failure", failure: next });
      failure += 1;
    } else if (item !== undefined) {
      entries.push({ kind: "row", row: item });
      row += 1;
    }
  }
  return [...entries, ...unplaced.map((each): Entry => ({ kind: "failure", failure: each }))];
}

function drawn(envelope: z.infer<typeof Envelope>): Result {
  const failures = envelope.failures ?? [];
  const { data } = envelope;
  if (typeof data === "string") {
    return { kind: "prose", text: data };
  }
  const rows = z.array(RowSchema).safeParse(data);
  if (rows.success && rows.data.length === 1 && envelope.truncated === undefined) {
    return { kind: "record", entries: inKeyOrder(rows.data, failures) };
  }
  if (rows.success) {
    const listing =
      envelope.truncated === undefined
        ? null
        : { total: envelope.total ?? null, truncated: envelope.truncated };
    return { kind: "table", entries: inKeyOrder(rows.data, failures), listing };
  }
  const record = RowSchema.safeParse(data);
  return record.success
    ? { kind: "record", entries: inKeyOrder([record.data], failures) }
    : { kind: "other" };
}
