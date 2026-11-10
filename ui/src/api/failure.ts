import { ApiError, type FailureDocument } from "./client";

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
    case "broken":
      return "The connection to flightdeck broke before the run ended. Check that flightdeck open is still running in your terminal, then run the command again.";
    case "shape":
      return "The server sent data that this page cannot read. Check that this page and flightdeck come from the same version.";
    case "status": {
      const said = failure.problem?.cause ?? failure.problem?.message ?? failure.problem?.title;
      if (failure.status === 401) {
        return "This page is no longer signed in. Run flightdeck open again in your terminal.";
      }
      if (failure.status === 413) {
        return "The request is larger than the server accepts. Send fewer or shorter values.";
      }
      if (said !== undefined) {
        return said;
      }
      return failure.status < 500
        ? `The server refused the request with status ${failure.status}.`
        : `The server failed with status ${failure.status}. Try again.`;
    }
  }
}

/** Returns true when the server refused the page's token. */
export function isSignedOut(error: unknown): boolean {
  return (
    error instanceof ApiError && error.failure.kind === "status" && error.failure.status === 401
  );
}

/** Returns true for crumbkit's refusal of a command line, such as `shelf::cli::usage`. */
function isUsageFailure(problem: FailureDocument | null): boolean {
  return problem?.code.endsWith("::cli::usage") === true;
}

/**
 * Returns the composer field that a refusal marks, and its words. Only a
 * failure document whose code is `flightdeck::run::refused`, the server's
 * refusal whatever the reply's status, or a tool's own `<tool>::cli::usage`
 * marks a field. The refusal's `argument` names the field. When it carries
 * `earlier`, the earlier argument that needs a value first is marked
 * instead, and when it carries `index`, the words name the refused value's
 * place in its list.
 */
export function fieldErrorOf(
  problem: FailureDocument | null,
  words: string,
): { name: string; words: string } | null {
  if (problem === null) {
    return null;
  }
  if (problem.code !== "flightdeck::run::refused" && !isUsageFailure(problem)) {
    return null;
  }
  const fields = problem.fields ?? {};
  const { argument, earlier, index } = fields;
  const name = typeof earlier === "string" ? earlier : argument;
  if (typeof name !== "string") {
    return null;
  }
  return { name, words: typeof index === "number" ? `Value ${index + 1}: ${words}` : words };
}

/** The runner's codes for a request it will not run as sent. */
const refusals = [
  "flightdeck::run::refused",
  "flightdeck::run::read_only",
  "flightdeck::run::needs_confirmation",
  "flightdeck::run::confirmation",
  "flightdeck::run::directory",
];

/**
 * Returns the card's word for a run that the server did not start, from the
 * failure document's code. The reply's status says less, because the server
 * may answer the same failure with another status.
 */
export function notStartedWord(problem: FailureDocument | null): string {
  const code = problem?.code;
  if (code !== undefined && refusals.includes(code)) {
    return "refused";
  }
  return code === "flightdeck::run::busy" ? "busy" : "not started";
}
