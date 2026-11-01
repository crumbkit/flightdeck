import type { FailureDocument } from "@/api/client";
import { fieldErrorOf } from "@/api/failure";

/**
 * Returns the start and end, in characters, of the word in a run's line
 * that a refusal names, or null when the failure is no refusal or the line
 * has no such word. The server writes an option as one word, such as
 * `--limit=101`, so the word that starts with the refused argument is
 * marked. A positional argument has no word of its own, so the last word
 * that equals the refused value is marked instead.
 */
export function refusedSpan(
  line: string,
  problem: FailureDocument | null,
): [number, number] | null {
  const marked = fieldErrorOf(problem, "");
  if (marked === null) {
    return null;
  }
  const value = problem?.fields?.value;
  const words = [...line.matchAll(/\S+/g)].map((match) => ({
    text: match[0],
    start: match.index,
  }));
  const named = words.find(
    (word) => word.text === marked.name || word.text.startsWith(`${marked.name}=`),
  );
  const valued =
    typeof value === "string" || typeof value === "number"
      ? words.findLast((word) => word.text === String(value))
      : undefined;
  const word = named ?? valued;
  return word === undefined ? null : [word.start, word.start + word.text.length];
}
