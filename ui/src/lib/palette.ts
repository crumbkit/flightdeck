import type { Command, Example, Tool } from "@/api/model";

/** How well a command matches the palette's query, best first. */
const ranks = ["exact path", "path", "about", "example"] as const;
type Rank = (typeof ranks)[number];

/** A command that matches the query, and the example it matched by. */
export type Match = { command: Command; rank: Rank; example: Example | null };

/**
 * Returns the commands that match `query`, best first. A command matches
 * when every word of the query appears in its path, its about text or one
 * of its examples' captions, ignoring case. An exact path ranks first, then
 * a path match, an about-text match and an example match. Within one rank,
 * the command with more runs comes first, and the guide's order breaks the
 * remaining ties. An empty query lists every command in the guide's order.
 */
export function search(tool: Tool, query: string): Match[] {
  const normal = query.trim().toLowerCase().replace(/\s+/g, " ");
  if (normal === "") {
    return tool.commands.map((command) => ({ command, rank: "path", example: null }));
  }
  const words = normal.split(" ");
  const has = (text: string | null) =>
    text !== null && words.every((word) => text.toLowerCase().includes(word));

  const matches: Match[] = tool.commands.flatMap((command): Match[] => {
    const path = command.path.join(" ");
    if (path === normal) {
      return [{ command, rank: "exact path", example: null }];
    }
    if (has(path)) {
      return [{ command, rank: "path", example: null }];
    }
    if (has(command.about)) {
      return [{ command, rank: "about", example: null }];
    }
    const example = command.examples.find((each) => has(each.about));
    return example === undefined ? [] : [{ command, rank: "example", example }];
  });
  // Array.prototype.sort is stable, so equal rows keep the guide's order.
  return matches.sort(
    (a, b) => ranks.indexOf(a.rank) - ranks.indexOf(b.rank) || b.command.runs - a.command.runs,
  );
}
