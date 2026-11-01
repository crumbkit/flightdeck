import { z } from "@/api/zod";
import type { Row } from "@/lib/table";

/** A failed item of a batch, such as one id that `books show` could not find. */
const ItemFailure = z.object({
  key: z.string().optional(),
  code: z.string(),
  message: z.string(),
});
export type ItemFailure = z.infer<typeof ItemFailure>;

const RowSchema = z.record(z.string(), z.unknown());

/** crumbkit's envelope around a result: its data and what it says about it. */
const Envelope = z.object({
  schema: z.string(),
  data: z.unknown(),
  total: z.number().nullish(),
  truncated: z.boolean().optional(),
  failures: z.array(ItemFailure).optional(),
});

/** How a card draws a result. */
export type Result =
  | {
      kind: "table";
      rows: Row[];
      listing: { total: number | null; truncated: boolean } | null;
      failures: ItemFailure[];
    }
  | { kind: "record"; record: Row; failures: ItemFailure[] }
  | { kind: "prose"; text: string }
  | { kind: "other" }
  | { kind: "none" };

function parse(line: string): unknown {
  try {
    return JSON.parse(line);
  } catch {
    return undefined;
  }
}

/**
 * Reads a run's lines of standard output. The last line is the result
 * document. A streaming command prints a row on each earlier line. A
 * listing becomes a table, one row or an object becomes a record, and text
 * becomes prose. `document` is the last line's JSON, for the card's JSON
 * view, or null when the last line is not JSON.
 */
export function resultOf(lines: string[]): { document: unknown; result: Result } {
  const last = lines.at(-1);
  const document = last === undefined ? undefined : parse(last);
  if (document === undefined) {
    return { document: null, result: { kind: "none" } };
  }
  const streamed = lines.slice(0, -1).map(parse);
  const streamRows = streamed.flatMap((value) => {
    const row = RowSchema.safeParse(value);
    return row.success ? [row.data] : [];
  });
  if (streamRows.length > 0) {
    return { document, result: { kind: "table", rows: streamRows, listing: null, failures: [] } };
  }
  const envelope = Envelope.safeParse(document);
  if (!envelope.success) {
    return { document, result: { kind: "other" } };
  }
  return { document, result: drawn(envelope.data) };
}

function drawn(envelope: z.infer<typeof Envelope>): Result {
  const failures = envelope.failures ?? [];
  const { data } = envelope;
  if (typeof data === "string") {
    return { kind: "prose", text: data };
  }
  const rows = z.array(RowSchema).safeParse(data);
  if (rows.success && rows.data.length === 1 && envelope.truncated === undefined) {
    const [record = {}] = rows.data;
    return { kind: "record", record, failures };
  }
  if (rows.success) {
    const listing =
      envelope.truncated === undefined
        ? null
        : { total: envelope.total ?? null, truncated: envelope.truncated };
    return { kind: "table", rows: rows.data, listing, failures };
  }
  const record = RowSchema.safeParse(data);
  return record.success ? { kind: "record", record: record.data, failures } : { kind: "other" };
}
