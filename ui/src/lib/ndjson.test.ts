import { describe, expect, it } from "vitest";
import { readLines } from "./ndjson";

function streamOf(chunks: Uint8Array[]): ReadableStream<Uint8Array> {
  return new ReadableStream({
    start(controller) {
      for (const chunk of chunks) {
        controller.enqueue(chunk);
      }
      controller.close();
    },
  });
}

async function collect(stream: ReadableStream<Uint8Array>): Promise<string[]> {
  const lines: string[] = [];
  for await (const line of readLines(stream)) {
    lines.push(line);
  }
  return lines;
}

describe("readLines", () => {
  it("joins a line that arrives in two chunks, even inside a character", async () => {
    const bytes = new TextEncoder().encode('{"text":"café"}\n{"n":2}\n');
    // The cut falls between the two bytes of "é".
    const cut = bytes.indexOf(0xc3) + 1;

    const lines = await collect(streamOf([bytes.slice(0, cut), bytes.slice(cut)]));

    expect(lines).toEqual(['{"text":"café"}', '{"n":2}']);
  });

  it("yields a last line without a newline and skips empty lines", async () => {
    const bytes = new TextEncoder().encode('{"a":1}\n\n{"b":2}');

    expect(await collect(streamOf([bytes]))).toEqual(['{"a":1}', '{"b":2}']);
  });
});
