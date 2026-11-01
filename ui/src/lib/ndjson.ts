/**
 * Reads a stream of newline-delimited JSON as its lines, yielding each
 * complete line as soon as its newline arrives. A chunk may end inside a
 * line or inside a character, so the reader keeps the unfinished part until
 * the next chunk completes it. Empty lines are skipped, and a last line
 * without a newline is yielded when the stream ends.
 */
export async function* readLines(stream: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  const decoder = new TextDecoder();
  const reader = stream.getReader();
  let pending = "";
  try {
    for (;;) {
      const { done, value } = await reader.read();
      pending += done ? decoder.decode() : decoder.decode(value, { stream: true });
      const lines = pending.split("\n");
      pending = done ? "" : (lines.pop() ?? "");
      for (const line of lines) {
        if (line.trim() !== "") {
          yield line;
        }
      }
      if (done) {
        return;
      }
    }
  } finally {
    reader.releaseLock();
  }
}
