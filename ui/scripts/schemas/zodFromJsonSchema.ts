// Writes a zod module from a JSON Schema (draft 2020-12) that another program
// publishes, so the wire type keeps one home (BASE-21).
//
// BASE-32: no maintained generator reads flightdeck's schemas with the
// workspace's TypeScript 7. This one translates only the keywords those
// schemas use, listed in KEYWORDS, and refuses any other keyword with its
// JSON pointer, so a new schema fails loudly here instead of losing a check.
// It misses three things. `format` is skipped, so the integer ranges uint32
// and uint64 are not enforced. `oneOf` is read as `anyOf`, which is exact
// only while the branches are disjoint, as flightdeck's are by a `kind`
// constant. A `$ref` back into a schema it sits inside becomes `z.unknown()`,
// so a recursive value is checked one level deep. A maintained generator that
// runs under TypeScript 7 and reads `$anchor` replaces this file.

/** A keyword this generator does not translate, at `pointer` in the schema. */
export class UnsupportedSchema extends Error {
  readonly pointer: string;

  constructor(pointer: string, message: string) {
    super(`${pointer === "" ? "/" : pointer}: ${message}`);
    this.name = "UnsupportedSchema";
    this.pointer = pointer;
  }
}

/** What the generated file says about itself and where it imports zod from. */
export interface Target {
  /** The generator's path from the repository's root, which the header names. */
  readonly generator: string;
  /** The schema's path from the repository's root, which the header names. */
  readonly source: string;
  /** The name of the root schema's constant, without the `Schema` suffix. */
  readonly root: string;
  /** The module the generated file imports `z` from. */
  readonly zodImport: string;
}

type Node = Readonly<Record<string, unknown>>;

const ANNOTATIONS = new Set(["description", "title", "format", "$schema", "$anchor"]);
const KEYWORDS = new Set([
  "$defs",
  "$ref",
  "type",
  "properties",
  "required",
  "items",
  "additionalProperties",
  "propertyNames",
  "const",
  "enum",
  "oneOf",
  "anyOf",
  "minimum",
  "pattern",
]);

function isNode(value: unknown): value is Node {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

/** Returns the zod module that `schema` describes, with the header `target` names. */
export function zodFromJsonSchema(schema: unknown, target: Target): string {
  if (!isNode(schema)) throw new UnsupportedSchema("", "the schema is not an object");
  const writer = new Writer(schema);
  const lines = [
    "// This file is generated. Do not edit it by hand.",
    `// ${target.generator} wrote it from ${target.source}.`,
    "// Change the source and run `pnpm run schemas` in ui/ to write it again.",
    `import { z } from ${JSON.stringify(target.zodImport)};`,
    "",
  ];
  for (const name of writer.definitionOrder()) {
    const definition = writer.definitions.get(name);
    if (definition === undefined) continue;
    lines.push(...declaration(name, definition, writer.expression(definition, `/$defs/${name}`)));
  }
  lines.push(...declaration(target.root, schema, writer.expression(schema, "")));
  return lines.join("\n");
}

function declaration(name: string, node: Node, expression: string): string[] {
  const lines: string[] = [];
  const description = node.description;
  if (isString(description)) {
    const text = description.replaceAll("*/", "*\\/").split("\n").join("\n * ");
    lines.push(`/** ${text} */`);
  }
  lines.push(`export const ${name}Schema = ${expression};`, "");
  return lines;
}

class Writer {
  readonly definitions = new Map<string, Node>();
  readonly #anchors = new Map<string, Node>();
  readonly #inside: Node[] = [];

  constructor(root: Node) {
    const defs = root.$defs;
    if (defs !== undefined) {
      if (!isNode(defs)) throw new UnsupportedSchema("/$defs", "$defs is not an object");
      for (const [name, definition] of Object.entries(defs)) {
        if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
          throw new UnsupportedSchema(`/$defs/${name}`, "the name is not an identifier");
        }
        if (!isNode(definition)) throw new UnsupportedSchema(`/$defs/${name}`, "not an object");
        this.definitions.set(name, definition);
      }
    }
    this.#collectAnchors(root);
  }

  #collectAnchors(value: unknown): void {
    if (Array.isArray(value)) {
      for (const item of value) this.#collectAnchors(item);
      return;
    }
    if (!isNode(value)) return;
    if (isString(value.$anchor)) this.#anchors.set(value.$anchor, value);
    for (const child of Object.values(value)) this.#collectAnchors(child);
  }

  /** Each definition's name, after the names of the definitions it references. */
  definitionOrder(): string[] {
    const order: string[] = [];
    const visiting = new Set<string>();
    const visit = (name: string): void => {
      if (order.includes(name)) return;
      if (visiting.has(name)) {
        throw new UnsupportedSchema(`/$defs/${name}`, "the definitions reference each other");
      }
      visiting.add(name);
      for (const reference of definitionReferences(this.definitions.get(name))) visit(reference);
      visiting.delete(name);
      order.push(name);
    };
    for (const name of this.definitions.keys()) visit(name);
    return order;
  }

  expression(node: Node, pointer: string, indent = ""): string {
    if (this.#inside.includes(node)) return "z.unknown()";
    this.#inside.push(node);
    try {
      return this.#translate(node, pointer, indent);
    } finally {
      this.#inside.pop();
    }
  }

  #child(value: unknown, pointer: string, indent: string): string {
    if (value === true) return "z.unknown()";
    if (!isNode(value)) throw new UnsupportedSchema(pointer, "the schema is not an object");
    return this.expression(value, pointer, indent);
  }

  #translate(node: Node, pointer: string, indent: string): string {
    for (const key of Object.keys(node)) {
      if (!KEYWORDS.has(key) && !ANNOTATIONS.has(key)) throw new UnsupportedSchema(pointer, key);
    }
    // Only the root's $defs are read, so definitions elsewhere would be lost.
    if (node.$defs !== undefined && pointer !== "") throw new UnsupportedSchema(pointer, "$defs");
    if (node.$ref !== undefined) return this.#reference(node, pointer, indent);
    if (node.const !== undefined) return literal(node.const, `${pointer}/const`);
    if (node.enum !== undefined) return stringEnum(node.enum, `${pointer}/enum`);
    const branches = node.oneOf ?? node.anyOf;
    if (branches !== undefined) {
      const keyword = node.oneOf === undefined ? "anyOf" : "oneOf";
      return this.#branches(branches, `${pointer}/${keyword}`, indent);
    }
    const type = node.type;
    if (type === undefined) return "z.unknown()";
    const types = Array.isArray(type) ? type : [type];
    const kinds = types.filter((kind) => kind !== "null");
    const nullable = kinds.length < types.length;
    const memberIndent = kinds.length > 1 ? `${indent}  ` : indent;
    const typed = kinds.map((kind) => this.#typed(node, kind, pointer, memberIndent));
    if (typed.length === 0) return "z.null()";
    const [only] = typed;
    const base = typed.length === 1 && only !== undefined ? only : union(typed, indent);
    return nullable ? `${base}.nullable()` : base;
  }

  #reference(node: Node, pointer: string, indent: string): string {
    const reference = node.$ref;
    if (!isString(reference)) throw new UnsupportedSchema(pointer, "$ref is not a string");
    const others = Object.keys(node).filter(
      (key) => key !== "$ref" && key !== "$defs" && !ANNOTATIONS.has(key),
    );
    if (others.length > 0) throw new UnsupportedSchema(pointer, `${others.join(", ")} beside $ref`);
    const definition = reference.match(/^#\/\$defs\/([A-Za-z_][A-Za-z0-9_]*)$/)?.[1];
    if (definition !== undefined && this.definitions.has(definition)) return `${definition}Schema`;
    const anchor = reference.match(/^#([A-Za-z_][A-Za-z0-9_.-]*)$/)?.[1];
    const anchored = anchor === undefined ? undefined : this.#anchors.get(anchor);
    if (anchored === undefined) throw new UnsupportedSchema(pointer, `$ref ${reference}`);
    return this.expression(anchored, pointer, indent);
  }

  #branches(value: unknown, pointer: string, indent: string): string {
    if (!Array.isArray(value)) throw new UnsupportedSchema(pointer, "not an array");
    const constants = value.map((branch) => (isNode(branch) ? branch.const : undefined));
    const strings = constants.filter(isString);
    if (strings.length === value.length) return stringEnum(strings, pointer);
    const isNull = (branch: unknown) => isNode(branch) && branch.type === "null";
    const rest = value.filter((branch) => !isNull(branch));
    const nullable = rest.length < value.length;
    const memberIndent = rest.length > 1 ? `${indent}  ` : indent;
    const translated = rest.map((branch) =>
      this.#child(branch, `${pointer}/${value.indexOf(branch)}`, memberIndent),
    );
    const [only] = translated;
    const base = translated.length === 1 && only !== undefined ? only : union(translated, indent);
    return nullable ? `${base}.nullable()` : base;
  }

  #typed(node: Node, kind: unknown, pointer: string, indent: string): string {
    switch (kind) {
      case "string":
        return isString(node.pattern) ? `z.string()${pattern(node.pattern)}` : "z.string()";
      case "integer":
        return `z.number().int()${minimum(node.minimum, pointer)}`;
      case "number":
        return `z.number()${minimum(node.minimum, pointer)}`;
      case "boolean":
        return "z.boolean()";
      case "array": {
        const items =
          node.items === undefined
            ? "z.unknown()"
            : this.#child(node.items, `${pointer}/items`, indent);
        return `z.array(${items})`;
      }
      case "object":
        return this.#object(node, pointer, indent);
      default:
        throw new UnsupportedSchema(`${pointer}/type`, `type ${String(kind)}`);
    }
  }

  #object(node: Node, pointer: string, indent: string): string {
    const { properties, additionalProperties } = node;
    if (properties !== undefined) {
      if (!isNode(properties))
        throw new UnsupportedSchema(`${pointer}/properties`, "not an object");
      if (additionalProperties !== undefined) {
        throw new UnsupportedSchema(pointer, "additionalProperties beside properties");
      }
      const required = Array.isArray(node.required) ? node.required.filter(isString) : [];
      const inner = `${indent}  `;
      const fields = Object.entries(properties).map(([key, value]) => {
        const field = this.#child(value, `${pointer}/properties/${key}`, inner);
        const optional = required.includes(key) ? "" : ".optional()";
        return `${inner}${propertyKey(key)}: ${field}${optional},`;
      });
      return fields.length === 0 ? "z.object({})" : `z.object({\n${fields.join("\n")}\n${indent}})`;
    }
    const keys =
      node.propertyNames === undefined
        ? "z.string()"
        : this.#propertyNames(node.propertyNames, `${pointer}/propertyNames`);
    const values =
      additionalProperties === undefined
        ? "z.unknown()"
        : this.#child(additionalProperties, `${pointer}/additionalProperties`, indent);
    return `z.record(${keys}, ${values})`;
  }

  #propertyNames(value: unknown, pointer: string): string {
    if (!isNode(value)) throw new UnsupportedSchema(pointer, "not an object");
    const others = Object.keys(value).filter((key) => key !== "pattern" && !ANNOTATIONS.has(key));
    if (others.length > 0) throw new UnsupportedSchema(pointer, others.join(", "));
    return isString(value.pattern) ? `z.string()${pattern(value.pattern)}` : "z.string()";
  }
}

function definitionReferences(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap(definitionReferences);
  if (!isNode(value)) return [];
  const own = isString(value.$ref) ? value.$ref.match(/^#\/\$defs\/(.+)$/)?.[1] : undefined;
  const nested = Object.values(value).flatMap(definitionReferences);
  return own === undefined ? nested : [own, ...nested];
}

function literal(value: unknown, pointer: string): string {
  if (value === null) return "z.null()";
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    return `z.literal(${JSON.stringify(value)})`;
  }
  throw new UnsupportedSchema(pointer, "a constant that is not a string, number, boolean or null");
}

function stringEnum(value: unknown, pointer: string): string {
  if (!Array.isArray(value) || value.length === 0 || !value.every(isString)) {
    throw new UnsupportedSchema(pointer, "an enum that is not a list of strings");
  }
  return `z.enum([${value.map((item) => JSON.stringify(item)).join(", ")}])`;
}

function union(members: string[], indent: string): string {
  const inner = `${indent}  `;
  return `z.union([\n${members.map((member) => `${inner}${member},`).join("\n")}\n${indent}])`;
}

function pattern(source: string): string {
  return `.regex(new RegExp(${JSON.stringify(source)}))`;
}

function minimum(value: unknown, pointer: string): string {
  if (value === undefined) return "";
  if (typeof value !== "number") throw new UnsupportedSchema(`${pointer}/minimum`, "not a number");
  return value === 0 ? ".nonnegative()" : `.min(${value})`;
}

function propertyKey(key: string): string {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key) ? key : JSON.stringify(key);
}
