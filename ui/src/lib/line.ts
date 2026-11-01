import type { Argument, Command, Tool } from "@/api/model";
import type { Value } from "@/api/runs";
import { type Draft, lineOfDraft, shows } from "./form";
import { type Token, tokenize } from "./tokens";

/** What a word of the line is, which chooses its colour. */
export type MarkKind = "path" | "flag" | "value" | "positional";

/** A word of the line, where it starts and ends, and whether it was quoted. */
export type Mark = { start: number; end: number; kind: MarkKind; quoted: boolean };

/** The text that replaces a part of the line to fix a problem. */
export type Fix = { start: number; end: number; text: string };

/** A part of the line that the model cannot place, crumbkit's words for it, and a fix, if one is known. */
export type Problem = { start: number; end: number; words: string; fix: Fix | null };

/**
 * What a line says: its command and a value for each argument by the
 * model's name, the words that were taken out, its marks, and whether the
 * person is still typing or the line has a problem. A pending or refused
 * line changes nothing in the form.
 */
export type Parsed = {
  command: Command | null;
  values: Record<string, Value>;
  dropped: string[];
  marks: Mark[];
  pending: boolean;
  problem: Problem | null;
};

/** crumbkit's flags that flightdeck adds to every run. */
const addedByFlightdeck = ["--json", "--full", "--no-input"];

/** crumbkit's flags that stand for the preview and the answer a person gives. */
const answers = ["--yes", "--dry-run"];

/**
 * Returns `text` without a leading `$ ` and the tool's name, which a line
 * copied from a terminal starts with. The name stays when one of the
 * tool's own commands starts with it.
 */
export function stripPrompt(tool: Tool, text: string): string {
  const unprompted = text.replace(/^\s*\$\s+/, "");
  const [first = ""] = unprompted.trimStart().split(/\s/, 1);
  const named = first === tool.name && !tool.commands.some((each) => each.path[0] === tool.name);
  return named ? unprompted.trimStart().slice(first.length).trimStart() : unprompted;
}

/** Returns the line's text for `draft`: its command line without the tool's name. */
export function textOf(tool: Tool, command: Command, draft: Draft): string {
  return lineOfDraft(tool.name, command, draft).slice(tool.name.length + 1);
}

/** Returns the notes that say why each dropped flag was taken out of the line. */
export function notesOf(dropped: string[]): string[] {
  const notes: string[] = [];
  const each = (flags: string[]) => flags.filter((flag) => dropped.includes(flag));
  if (each(addedByFlightdeck).length > 0) {
    notes.push(
      "flightdeck adds --json, --full and --no-input itself, so they were taken out of the line.",
    );
  }
  for (const flag of each(answers)) {
    notes.push(
      `flightdeck asks before it runs a command that changes data, so ${flag} was taken out of the line.`,
    );
  }
  if (dropped.includes("--fields")) {
    notes.push("The result's table chooses its columns, so --fields was taken out of the line.");
  }
  if (dropped.includes("--long")) {
    notes.push("This command's rows have no long columns, so --long was taken out of the line.");
  }
  return notes;
}

/**
 * Reads a line, such as `books show 2 5 --long`, against the tool's model,
 * the way the tool itself would read it, and in the tool's words when it
 * cannot place a word.
 *
 * `caret` is where the caret is while the line has focus, or null after
 * the person leaves it. The word under the caret is still being typed, so
 * reading stops before it and the line is pending. Reading also stops at
 * the first problem, as the tool does.
 */
export function parseLine(tool: Tool, text: string, caret: number | null): Parsed {
  const { tokens, open } = tokenize(text);
  const reader = new Reader(tool, tokens, open, caret);
  reader.read();
  return reader.parsed();
}

/** Returns how a refusal names an argument, such as `--limit <N>` or `<ID>...`. */
function display(argument: Argument): string {
  const { arity } = argument;
  if (argument.kind === "positional") {
    return arity.kind === "many" ? `${argument.name}...` : argument.name;
  }
  if (arity.kind === "one" || arity.kind === "many") {
    return `${argument.name} <${arity.value_names[0] ?? "VALUE"}>`;
  }
  return argument.name;
}

/**
 * Returns the word of `candidates` closest to `word`, or null when none is
 * close enough to be a typing mistake.
 *
 * RUST-10: this counts the single-character edits between two words, and allows one
 * edit for every three characters. The tool's own parser, clap, measures
 * similarity in another way, so for a word far from every candidate the two
 * may suggest different words or one may suggest none. A copy of clap's
 * measure would replace this if the difference shows.
 */
function similar(word: string, candidates: string[]): string | null {
  const most = Math.max(1, Math.floor(word.length / 3));
  let best: { candidate: string; distance: number } | null = null;
  for (const candidate of candidates) {
    const distance = editDistance(word, candidate);
    if (distance <= most && (best === null || distance < best.distance)) {
      best = { candidate, distance };
    }
  }
  return best?.candidate ?? null;
}

/** Returns the number of inserts, deletes and changes of one character that turn `a` into `b`. */
function editDistance(a: string, b: string): number {
  let previous = Array.from({ length: b.length + 1 }, (_, index) => index);
  for (let i = 1; i <= a.length; i += 1) {
    const current = [i];
    for (let j = 1; j <= b.length; j += 1) {
      const change = a[i - 1] === b[j - 1] ? 0 : 1;
      current.push(
        Math.min(
          (previous[j] ?? 0) + 1,
          (current[j - 1] ?? 0) + 1,
          (previous[j - 1] ?? 0) + change,
        ),
      );
    }
    previous = current;
  }
  return previous[b.length] ?? 0;
}

/** Reads the tokens of one line in order, as `parseLine` describes. */
class Reader {
  readonly #tool: Tool;
  readonly #tokens: Token[];
  readonly #open: number | null;
  readonly #caret: number | null;
  #index = 0;
  #command: Command | null = null;
  readonly #values: Record<string, Value> = {};
  /** The first token of each argument that has a value, for a refusal of a switch. */
  readonly #given = new Map<string, Token>();
  readonly #dropped: string[] = [];
  readonly #marks: Mark[] = [];
  #pending = false;
  #problem: Problem | null = null;

  constructor(tool: Tool, tokens: Token[], open: number | null, caret: number | null) {
    this.#tool = tool;
    this.#tokens = tokens;
    this.#open = open;
    this.#caret = caret;
  }

  parsed(): Parsed {
    // The words after the place where reading stopped are coloured by
    // their look alone, so the line keeps its colours while it is typed.
    for (const token of this.#tokens.slice(this.#index)) {
      this.#mark(token, token.value.startsWith("-") ? "flag" : "value");
    }
    return {
      command: this.#command,
      values: this.#values,
      dropped: this.#dropped,
      marks: this.#marks,
      pending: this.#pending,
      problem: this.#problem,
    };
  }

  read(): void {
    if (!this.#readPath()) {
      return;
    }
    const command = this.#command;
    if (command === null) {
      return;
    }
    const positionals = command.arguments.filter((each) => each.kind === "positional");
    let position = 0;
    let afterDashes = false;
    for (let token = this.#next(); token !== null; token = this.#next()) {
      if (!afterDashes && !token.quoted && token.value === "--") {
        afterDashes = true;
        this.#mark(token, "flag");
      } else if (!afterDashes && token.value.length > 1 && token.value.startsWith("-")) {
        if (!this.#readOption(command, token)) {
          return;
        }
      } else {
        const argument = positionals[position];
        if (argument === undefined) {
          this.#refuse(token, `unexpected argument '${token.value}' found`, null);
          return;
        }
        this.#mark(token, "positional");
        if (!this.#add(argument, token, token.value)) {
          return;
        }
        if (argument.arity.kind !== "many") {
          position += 1;
        }
      }
    }
  }

  /** Reads the command path. Returns false when reading stops. */
  #readPath(): boolean {
    const path: string[] = [];
    let last: Token | null = null;
    while (this.#index < this.#tokens.length) {
      const token = this.#peek();
      if (token === null) {
        return false;
      }
      const longer = [...path, token.value];
      const starts = this.#tool.commands.some((each) =>
        longer.every((word, index) => each.path[index] === word),
      );
      if (!starts) {
        if (this.#command !== null) {
          return true;
        }
        const words = this.#nextWords(path);
        const like = similar(token.value, words);
        const said = `unrecognized subcommand '${token.value}'`;
        this.#refuse(
          token,
          like === null ? said : `${said}; a similar subcommand exists: '${like}'`,
          like === null || token.quoted ? null : { start: token.start, end: token.end, text: like },
        );
        return false;
      }
      this.#index += 1;
      this.#mark(token, "path");
      path.push(token.value);
      last = token;
      this.#command =
        this.#tool.commands.find((each) => each.path.join(" ") === path.join(" ")) ?? null;
    }
    if (this.#command === null && last !== null) {
      if (this.#caret !== null) {
        this.#pending = true;
      } else {
        const words = this.#nextWords(path).join(", ");
        this.#refuse(
          last,
          `the command \`${this.#tool.name} ${path.join(" ")}\` needs a subcommand: ${words}`,
          null,
        );
      }
      return false;
    }
    return true;
  }

  /** Returns the words that can follow `path` in a command path. */
  #nextWords(path: string[]): string[] {
    const words = this.#tool.commands
      .filter((each) => path.every((word, index) => each.path[index] === word))
      .flatMap((each) => each.path.slice(path.length, path.length + 1));
    return [...new Set(words)];
  }

  /** Reads one option and its value. Returns false when reading stops. */
  #readOption(command: Command, token: Token): boolean {
    const long = token.value.startsWith("--");
    // RUST-10: a short option is read as `-x` or `-x VALUE`. Short flags
    // run together, such as `-ab`, and a value joined to its option, such
    // as `-n5`, are refused as unknown. Splitting them as clap does
    // replaces this when a tool's examples use them.
    const equals = long ? token.value.indexOf("=") : -1;
    const name = equals === -1 ? token.value : token.value.slice(0, equals);
    const inline = equals === -1 ? null : token.value.slice(equals + 1);
    const argument = command.arguments.find((each) => each.name === name || each.short === name);
    if (argument === undefined) {
      const like = similar(
        name,
        command.arguments.filter((each) => each.kind === "named").map((each) => each.name),
      );
      const said = `unexpected argument '${name}' found`;
      const end = token.start + name.length;
      this.#refuse(
        token,
        like === null ? said : `${said}; a similar argument exists: '${like}'`,
        like === null || token.quoted ? null : { start: token.start, end, text: like },
      );
      return false;
    }
    this.#mark(token, "flag");
    const { kind } = argument.arity;
    if (kind === "count") {
      this.#refuse(token, `flightdeck cannot send the count '${name}' yet`, null);
      return false;
    }
    if (kind === "flag") {
      if (inline !== null) {
        this.#refuse(
          token,
          `unexpected value '${inline}' for '${name}' found; no more were expected`,
          null,
        );
        return false;
      }
      return this.#add(argument, token, true);
    }
    if (inline !== null) {
      return this.#add(argument, token, inline);
    }
    const value = this.#next();
    if (value === null) {
      if (this.#index >= this.#tokens.length) {
        if (this.#caret !== null) {
          this.#pending = true;
        } else {
          this.#refuse(
            token,
            `a value is required for '${display(argument)}' but none was supplied`,
            null,
          );
        }
      }
      return false;
    }
    this.#mark(value, "value");
    return this.#add(argument, token, value.value);
  }

  /**
   * Adds a value of `argument`, or drops a crumbkit flag that the form does
   * not show. Returns false when the value is refused.
   */
  #add(argument: Argument, token: Token, value: true | string): boolean {
    const command = this.#command;
    if (command === null) {
      return false;
    }
    if (!shows(command, argument)) {
      if (!this.#dropped.includes(argument.name)) {
        this.#dropped.push(argument.name);
      }
      return true;
    }
    if (typeof value === "string" && argument.possible_values.length > 0) {
      if (!argument.possible_values.includes(value)) {
        const possible = argument.possible_values.join(", ");
        this.#refuse(
          token,
          `invalid value '${value}' for '${display(argument)}'; possible values: ${possible}`,
          null,
        );
        return false;
      }
    }
    const current = this.#values[argument.name];
    if (argument.arity.kind === "many" && typeof value === "string") {
      this.#values[argument.name] = [...(Array.isArray(current) ? current : []), value];
    } else if (current !== undefined) {
      this.#refuse(
        token,
        `the argument '${display(argument)}' cannot be used multiple times`,
        null,
      );
      return false;
    } else {
      this.#values[argument.name] = value;
    }
    if (!this.#given.has(argument.name)) {
      this.#given.set(argument.name, token);
    }
    const other = command.arguments.find(
      (each) =>
        each !== argument &&
        this.#given.has(each.name) &&
        (argument.one_of.includes(each.name) || each.one_of.includes(argument.name)),
    );
    if (other !== undefined) {
      this.#refuse(
        token,
        `the argument '${display(other)}' cannot be used with '${display(argument)}'`,
        null,
      );
      return false;
    }
    return true;
  }

  /**
   * Returns the next token and moves past it, or null at the end of the
   * line, at the word under the caret, which makes the line pending, and at
   * a quote that is not closed, which is a problem once the person leaves
   * the line.
   */
  #next(): Token | null {
    const token = this.#peek();
    if (token !== null) {
      this.#index += 1;
    }
    return token;
  }

  /** Returns the next token without moving past it, as `#next` describes. */
  #peek(): Token | null {
    const token = this.#tokens[this.#index];
    if (token === undefined) {
      return null;
    }
    const caret = this.#caret;
    if (caret !== null && caret >= token.start && caret <= token.end) {
      this.#pending = true;
      return null;
    }
    if (this.#open !== null && this.#open >= token.start && this.#open < token.end) {
      const column = this.#open + 1;
      this.#refuse(
        { ...token, start: this.#open, end: token.end },
        `the quote at column ${column} is not closed`,
        null,
      );
      return null;
    }
    return token;
  }

  #mark(token: Token, kind: MarkKind): void {
    this.#marks.push({ start: token.start, end: token.end, kind, quoted: token.quoted });
  }

  #refuse(token: Token, words: string, fix: Fix | null): void {
    this.#problem = { start: token.start, end: token.end, words, fix };
  }
}
