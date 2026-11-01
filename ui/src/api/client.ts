import { z } from "@/api/zod";
import { forgetToken, readToken, writeToken } from "@/storage";

/**
 * A failure document, with the fields the page reads: crumbkit's failure
 * document, which a tool prints and the runner answers with, or the
 * server's problem document of a refused request, which carries a `title`
 * in place of a `message`. Each has a stable `code`.
 */
export const FailureDocument = z.object({
  code: z.string(),
  message: z.string().optional(),
  cause: z.string().optional(),
  recovery: z.unknown().optional(),
  fields: z.record(z.string(), z.unknown()).optional(),
  title: z.string().optional(),
});
export type FailureDocument = z.infer<typeof FailureDocument>;

/** Why a request to the server failed, in terms a screen can word (APP-14). */
export type ApiFailure =
  | { kind: "status"; status: number; problem: FailureDocument | null }
  | { kind: "shape"; issues: string }
  | { kind: "network" };

export class ApiError extends Error {
  readonly failure: ApiFailure;

  constructor(failure: ApiFailure, options?: ErrorOptions) {
    super(`request failed: ${failure.kind}`, options);
    this.name = "ApiError";
    this.failure = failure;
  }
}

/** How long one request other than a run may take (BASE-26). */
const deadlineMs = 10_000;

// The token of this tab's session. The page keeps it in memory and in
// sessionStorage, so a reload keeps the session and a browser that refuses
// storage still signs in once.
let token: string | null = readToken();

/** Returns true when the page holds a token. */
export function hasToken(): boolean {
  return token !== null;
}

/** Keeps the token that the server gave in exchange for the launch code. */
export function keepToken(value: string): void {
  token = value;
  writeToken(value);
}

/** Forgets the token, after the server refused it. */
export function dropToken(): void {
  token = null;
  forgetToken();
}

function headers(json: boolean): Headers {
  const result = new Headers({ Accept: "application/json" });
  if (json) {
    result.set("Content-Type", "application/json");
  }
  if (token !== null) {
    result.set("Authorization", `Bearer ${token}`);
  }
  return result;
}

async function send(path: string, init: RequestInit): Promise<Response> {
  let response: Response;
  try {
    response = await fetch(path, init);
  } catch (error) {
    throw new ApiError({ kind: "network" }, { cause: error });
  }
  if (!response.ok) {
    throw new ApiError({
      kind: "status",
      status: response.status,
      problem: await problemOf(response),
    });
  }
  return response;
}

async function problemOf(response: Response): Promise<FailureDocument | null> {
  try {
    const parsed = FailureDocument.safeParse(await response.json());
    return parsed.success ? parsed.data : null;
  } catch {
    return null;
  }
}

async function parseJson<T>(response: Response, schema: z.ZodType<T>): Promise<T> {
  let body: unknown;
  try {
    body = await response.json();
  } catch (error) {
    throw new ApiError({ kind: "shape", issues: "the body is not JSON" }, { cause: error });
  }
  const parsed = schema.safeParse(body);
  if (!parsed.success) {
    throw new ApiError({ kind: "shape", issues: parsed.error.message }, { cause: parsed.error });
  }
  return parsed.data;
}

/**
 * The one place that calls `fetch` (BASE-7). Each function sends the token
 * when the page holds one, parses the answer with `schema`, so every value
 * past this module has the schema's type (BASE-21), and throws ApiError for
 * a refused or malformed answer, because TanStack Query takes a thrown
 * error as the query's failure (BASE-23).
 */
export async function getJson<T>(
  path: string,
  schema: z.ZodType<T>,
  signal: AbortSignal,
): Promise<T> {
  const response = await send(path, {
    signal: AbortSignal.any([signal, AbortSignal.timeout(deadlineMs)]),
    headers: headers(false),
  });
  return parseJson(response, schema);
}

/** Sends `body` as JSON and parses the answer with `schema`. */
export async function postJson<T>(
  path: string,
  body: unknown,
  schema: z.ZodType<T>,
  signal: AbortSignal,
): Promise<T> {
  const response = await send(path, {
    method: "POST",
    signal: AbortSignal.any([signal, AbortSignal.timeout(deadlineMs)]),
    headers: headers(true),
    body: JSON.stringify(body),
  });
  return parseJson(response, schema);
}

/**
 * Sends `body` as JSON and returns the answer's body as a stream, for a
 * route that answers with one JSON document per line.
 *
 * BASE-26's deadline is left out here: a run lasts as long as the tool runs,
 * and the caller's signal ends it. A deadline per run replaces this when
 * the page offers one.
 */
export async function postStream(
  path: string,
  body: unknown,
  signal: AbortSignal,
): Promise<ReadableStream<Uint8Array>> {
  const response = await send(path, {
    method: "POST",
    signal,
    headers: headers(true),
    body: JSON.stringify(body),
  });
  if (response.body === null) {
    throw new ApiError({ kind: "shape", issues: "the answer has no body" });
  }
  return response.body;
}
