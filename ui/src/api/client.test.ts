import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, postStream } from "./client";
import { describeFailure, notStartedWord } from "./failure";

afterEach(() => {
  vi.unstubAllGlobals();
});

/** The failures that the run routes answer with, each with its status. */
const failures = [
  { status: 422, code: "flightdeck::run::refused", word: "refused" },
  { status: 422, code: "flightdeck::run::needs_confirmation", word: "refused" },
  { status: 422, code: "flightdeck::run::read_only", word: "refused" },
  { status: 422, code: "flightdeck::run::confirmation", word: "refused" },
  { status: 503, code: "flightdeck::run::busy", word: "busy" },
  { status: 503, code: "flightdeck::run::closed", word: "not started" },
  { status: 500, code: "flightdeck::run::spawn", word: "not started" },
  { status: 500, code: "flightdeck::run::random", word: "not started" },
  { status: 404, code: "flightdeck::run::not_running", word: "not started" },
];

describe("postStream", () => {
  it.each(failures)("reads the failure document of a $status reply with $code", async (failure) => {
    const document = {
      schema: "flightdeck/failure/v1",
      code: failure.code,
      cause: `the cause of ${failure.code}`,
      hint: "a hint",
      fields: { path: ["books", "show"] },
    };
    vi.stubGlobal("fetch", async () =>
      Response.json(document, {
        status: failure.status,
        headers: { "Content-Type": "application/problem+json" },
      }),
    );

    const error: unknown = await postStream("/api/runs", {}, new AbortController().signal).catch(
      (caught: unknown) => caught,
    );

    expect(error).toBeInstanceOf(ApiError);
    const problem =
      error instanceof ApiError && error.failure.kind === "status" ? error.failure.problem : null;
    expect(problem?.code).toBe(failure.code);
    expect(describeFailure(error)).toBe(`the cause of ${failure.code}`);
    expect(notStartedWord(problem)).toBe(failure.word);
  });
});
