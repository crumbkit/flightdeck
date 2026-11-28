import type { Actor, EdgeKind, NodeKind } from "@/api/workflows";

// The words and glyphs of the Workflows page, as the signed mockup has them.

/** What a lane's actor is called under its label. */
export const actorWord: Record<Actor, string> = {
  owner: "you",
  agent: "agent",
  program: "program",
  schedule: "schedule",
};

/** What a node's kind is called in its chip and its accessible name. */
export const kindWord: Record<NodeKind, string> = {
  start: "Start",
  step: "Step",
  gate: "Check",
  choice: "Choice",
  wait: "Waits",
  end: "End",
};

/** What an edge's kind is called beside the node it leads to. */
export const edgeWord: Record<EdgeKind, string> = {
  next: "then",
  pass: "passes",
  fail: "fails",
  retry: "back to fix",
  loop: "loops",
  skip: "skipped",
};

/** The run sheet's words before the step that an edge back leads to. */
export const backWord: Record<EdgeKind, string> = {
  fail: "If it fails, back to",
  retry: "Back to",
  loop: "Then again from",
  next: "Back to",
  pass: "Back to",
  skip: "Back to",
};

/** Returns the first 8 characters of a commit or a version. */
export function short(text: string): string {
  return text.slice(0, 8);
}

/** Returns the last segment of a board path, such as a flow's version. */
export function lastSegment(path: string): string {
  return path.split("/").pop() ?? path;
}

/** One glyph of a node's kind, drawn in the current colour. */
export function Glyph({ kind }: { kind: NodeKind }) {
  return (
    <svg className="wf-glyph" viewBox="0 0 14 14" aria-hidden="true">
      {kind === "start" && <circle cx="7" cy="7" r="4.5" />}
      {kind === "end" && (
        <>
          <circle cx="7" cy="7" r="4.5" />
          <circle className="wf-glyph-fill" cx="7" cy="7" r="1.6" />
        </>
      )}
      {kind === "step" && <rect x="2.5" y="2.5" width="9" height="9" rx="1.5" />}
      {kind === "gate" && (
        <>
          <path d="M7 1.8 12.2 7 7 12.2 1.8 7Z" />
          <path d="M4.8 7.1 6.4 8.6 9.2 5.6" />
        </>
      )}
      {kind === "choice" && <path d="M7 1.8 12.2 7 7 12.2 1.8 7Z" />}
      {kind === "wait" && (
        <>
          <circle cx="7" cy="7" r="5" />
          <path d="M7 4v3.2l2.1 1.4" />
        </>
      )}
    </svg>
  );
}

/** Returns `count` as a word up to ten, as a sentence starts it, or as digits. */
export function countWord(count: number): string {
  const words = [
    "No",
    "One",
    "Two",
    "Three",
    "Four",
    "Five",
    "Six",
    "Seven",
    "Eight",
    "Nine",
    "Ten",
  ];
  return words[count] ?? String(count);
}
