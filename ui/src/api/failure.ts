import { ApiError, type Problem } from "./client";

/**
 * The one wording of every failed request the page shows (APP-14). It says
 * what happened and what to do next, and never shows the message of an
 * error it does not know.
 */
export function describeFailure(error: unknown): string {
  if (!(error instanceof ApiError)) {
    return "Something went wrong. Reload the page to try again.";
  }
  const { failure } = error;
  switch (failure.kind) {
    case "network":
      return "The flightdeck server did not answer. Check that flightdeck open is still running in your terminal, then try again.";
    case "shape":
      return "The server sent data that this page cannot read. Reload the page, and reinstall flightdeck if that does not help.";
    case "status": {
      const said = failure.problem?.cause ?? failure.problem?.message ?? failure.problem?.detail;
      if (failure.status === 401) {
        return "This page is no longer signed in. Run flightdeck open again in your terminal.";
      }
      if (failure.status === 413) {
        return "The request is larger than the server accepts. Send fewer or shorter values.";
      }
      return said === undefined
        ? `The server refused the request with status ${failure.status}. Try again.`
        : said;
    }
  }
}

/** Returns true when the server refused the page's token. */
export function isSignedOut(error: unknown): boolean {
  return (
    error instanceof ApiError && error.failure.kind === "status" && error.failure.status === 401
  );
}

/**
 * Returns the composer field that a refusal marks, and its words. The
 * refusal's `argument` names the field. When it carries `earlier`, the
 * earlier argument that needs a value first is marked instead, and when it
 * carries `index`, the words name the refused value's place in its list.
 */
export function fieldErrorOf(
  problem: Problem | null,
  words: string,
): { name: string; words: string } | null {
  const fields = problem?.fields ?? {};
  const { argument, earlier, index } = fields;
  const name = typeof earlier === "string" ? earlier : argument;
  if (typeof name !== "string") {
    return null;
  }
  return { name, words: typeof index === "number" ? `Value ${index + 1}: ${words}` : words };
}
