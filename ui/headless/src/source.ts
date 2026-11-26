// Where the compliance pages read their document from. The app supplies the
// source, such as a fetch of compliance.json, so the core itself calls no
// network and stores nothing.
import { type ComplianceDocument, parseDocument } from "./document.ts";

/**
 * The part of an AbortSignal the core reads. The core's types hold no DOM,
 * so it names only what it needs, and a DOM or Node AbortSignal fits.
 */
export interface AbortSignalLike {
  readonly aborted: boolean;
  readonly reason?: unknown;
}

/** Reads the parsed JSON of a compliance.json, such as with fetch, and stops when `signal` aborts. */
export interface ComplianceSource {
  read(signal: AbortSignalLike): Promise<unknown>;
}

/** The source failed to read the document; `cause` holds what it threw. */
export class SourceFailed extends Error {
  readonly code = "source_failed";

  constructor(options: ErrorOptions) {
    super("the compliance document could not be read", options);
    this.name = "SourceFailed";
  }
}

/**
 * Returns the document `source` reads. A read that throws or aborts rejects
 * with SourceFailed, its error as the cause, and a read that is not a
 * compliance document rejects with InvalidDocument.
 */
export async function readDocument(
  source: ComplianceSource,
  signal: AbortSignalLike,
): Promise<ComplianceDocument> {
  let json: unknown;
  try {
    json = await source.read(signal);
  } catch (error) {
    throw new SourceFailed({ cause: error });
  }
  return parseDocument(json);
}
