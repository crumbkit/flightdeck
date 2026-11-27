// The note at the top of every snapshot page (artboards 54 to 56): when the
// document was built, when the next build runs, and the run that built it.
import type { ComplianceDocument } from "./document.ts";
import { moment, shortName } from "./words.ts";

/** When the snapshot was built and by which run. */
export interface Snapshot {
  readonly builtAt: string;
  readonly nextBuildAt: string | null;
  /** The note's sentence, such as "This page is a snapshot built at 06:00 UTC on 25 November 2026. …". */
  readonly sentence: string;
  /** The short form a phone shows, such as "Snapshot built at 06:00 UTC on 25 November 2026. …". */
  readonly short: string;
  /** The run that built the document, or null when the build was not told it. */
  readonly buildRun: { readonly label: string | null; readonly url: string | null } | null;
  /** The imposed set's origin, such as "owner/controls". */
  readonly from: string;
  /**
   * The imposed set's repository and branch beside the build run, such as
   * "platform-controls · main", or its origin when the build was not told
   * the branch.
   */
  readonly rules: string;
}

/** Returns the day of an RFC 3339 time in UTC, such as "2026-11-25". */
function day(iso: string): string {
  return new Date(iso).toISOString().slice(0, 10);
}

/** Returns when the next build runs, saying "tomorrow" for the day after the build. */
function nextWords(builtAt: string, next: string): string {
  const tomorrow = new Date(new Date(builtAt).getTime() + 86_400_000).toISOString();
  if (day(next) === day(tomorrow)) {
    return `${moment(next).replace(/ on .*$/, "")} tomorrow`;
  }
  return moment(next);
}

/** Returns the snapshot note of `document`. */
export function snapshotOf(document: ComplianceDocument): Snapshot {
  const built = moment(document.built_at);
  const next =
    document.next_build_at === null ? null : nextWords(document.built_at, document.next_build_at);
  const nextSentence = next === null ? "" : `, and the next build is at ${next}`;
  return {
    builtAt: document.built_at,
    nextBuildAt: document.next_build_at,
    sentence: `This page is a snapshot built at ${built}. It shows each repository's newest report at that time${nextSentence}.`,
    short: `Snapshot built at ${built}.${next === null ? "" : ` The next build is at ${next}.`}`,
    buildRun: document.build_run,
    from: document.imposed.from,
    rules:
      (document.imposed.branch ?? null) === null
        ? document.imposed.from
        : `${shortName(document.imposed.from)} · ${document.imposed.branch}`,
  };
}
