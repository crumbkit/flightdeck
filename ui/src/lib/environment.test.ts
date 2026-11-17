import { describe, expect, it } from "vitest";
import type { Variable } from "@/api/model";
import { environmentShelf } from "../../tests/fixtures/shelf";
import {
  environmentSummary,
  isWithheld,
  reasonOf,
  sourceWords,
  typeWords,
  unsetPhrase,
  wrongOf,
  wrongWords,
} from "./environment";
import { partsOf } from "./variableParts";

const variables = environmentShelf.environment ?? [];

/** Returns `variable` as a tool before crumbkit-starmap 0.4.2 prints it, with only the text fields. */
function textOnly(variable: Variable): Variable {
  return { ...variable, type_name: null, unset_rule: null, source_kind: null };
}

/** Returns the plain text of an unset phrase. */
function plain(variable: Variable): string {
  return unsetPhrase(variable)
    .map((part) => (typeof part === "string" ? part : part.code))
    .join("");
}

/** Returns the fixture's variable named `name`, or throws. */
function variable(name: string): Variable {
  const found = variables.find((each) => each.name === name);
  if (found === undefined) {
    throw new Error(`no variable ${name}`);
  }
  return found;
}

describe("wrongOf", () => {
  it("marks a required variable that is unset and a value that does not fit", () => {
    expect(wrongOf(variable("SHELF_CATALOGUE_URL"))).toBe("unset");
    expect(wrongOf(variable("SHELF_REGION"))).toBe("invalid");
  });

  it("leaves an optional unset variable, a default and a set value alone", () => {
    expect(wrongOf(variable("SHELF_MIRROR_ACCOUNT"))).toBeNull();
    expect(wrongOf(variable("SHELF_PORT"))).toBeNull();
    expect(wrongOf(variable("SHELF_DEBUG"))).toBeNull();
  });

  it("reads the local rule of the unset rule, whatever production's is", () => {
    const token = variable("SHELF_MIRROR_TOKEN");
    const unset = { ...token, value: { state: "unset" as const, text: null } };
    const requiredLocally = {
      ...unset,
      unset_rule: {
        local: { rule: "required" as const },
        production: { rule: "optional" as const },
      },
    };

    expect(wrongOf(unset)).toBeNull();
    expect(wrongOf(requiredLocally)).toBe("unset");
  });

  it("reads the local rule first from the text of a tool without the unset rule", () => {
    const localOnly = {
      ...textOnly(variable("SHELF_MIRROR_ACCOUNT")),
      unset: "required; production optional",
    };
    const productionOnly = { ...localOnly, unset: "optional; production required" };

    expect(wrongOf(localOnly)).toBe("unset");
    expect(wrongOf(productionOnly)).toBeNull();
  });
});

describe("wrongWords", () => {
  it("gives the effect of a required unset variable", () => {
    expect(wrongWords(variable("SHELF_CATALOGUE_URL"))).toBe(
      "Every books command fails, because shelf has no catalogue to read.",
    );
  });

  it("gives the check of an invalid value, and the type when the declaration states no check", () => {
    expect(wrongWords(variable("SHELF_REGION"))).toBe(
      "The region must have a mirror that answers.",
    );
    expect(wrongWords({ ...variable("SHELF_REGION"), check: null })).toBe(
      "moon does not fit the type region: one of eu, us, ap.",
    );
  });

  it("never names a withheld value that does not fit", () => {
    const token = variable("SHELF_MIRROR_TOKEN");
    const invalid = { ...token, value: { state: "invalid" as const, text: null } };

    expect(wrongWords(invalid)).toBe("The value does not fit the type string.");
  });
});

describe("environmentSummary", () => {
  it("counts every variable, crumbkit's included, and what stops the tool", () => {
    expect(environmentSummary(variables)).toBe("16 declared · 1 required unset · 1 invalid");
  });

  it("names only the count when nothing stops the tool", () => {
    expect(environmentSummary([variable("SHELF_PORT")])).toBe("1 declared");
  });
});

describe("isWithheld", () => {
  it("is true for a set withheld variable and false for an unset or shown one", () => {
    expect(isWithheld(variable("SHELF_MIRROR_TOKEN"))).toBe(true);
    expect(isWithheld(variable("SHELF_CATALOGUE_URL"))).toBe(false);
    expect(isWithheld(variable("SHELF_DEBUG"))).toBe(false);
  });

  it("is false for a shown variable that is invalid and has no text", () => {
    const odd = { ...variable("SHELF_DEBUG"), value: { state: "invalid" as const, text: null } };

    expect(isWithheld(odd)).toBe(false);
  });
});

describe("reasonOf", () => {
  it("keeps the reason on an open row whose detail does not show it", () => {
    const noEffect = { ...variable("SHELF_CATALOGUE_URL"), effect: null };
    const noCheck = { ...variable("SHELF_REGION"), check: null };

    expect(reasonOf(noEffect, true)).toBe("The tool needs a value for this variable.");
    expect(reasonOf(noCheck, true)).toBe("moon does not fit the type region: one of eu, us, ap.");
  });

  it("leaves the reason to the detail of an open row that shows the effect or the check", () => {
    expect(reasonOf(variable("SHELF_CATALOGUE_URL"), true)).toBeNull();
    expect(reasonOf(variable("SHELF_REGION"), true)).toBeNull();
    expect(reasonOf(variable("SHELF_REGION"), false)).toBe(
      "The region must have a mirror that answers.",
    );
  });
});

describe("typeWords", () => {
  it("gives the one word that names the type", () => {
    expect(typeWords(variable("SHELF_PORT"))).toBe("port");
    expect(typeWords(variable("SHELF_LOG_LEVEL"))).toBe("enumeration");
    expect(typeWords({ ...variable("SHELF_PORT"), type_name: "socket_address" })).toBe(
      "socket address",
    );
  });

  it("falls back to the type's text for a word a later starmap adds and for an older tool", () => {
    expect(typeWords({ ...variable("SHELF_PORT"), type_name: "other", type: "another type" })).toBe(
      "another type",
    );
    expect(typeWords(textOnly(variable("SHELF_LOG_LEVEL")))).toBe(
      "log-level: one of error, warn, info, debug",
    );
  });
});

describe("unsetPhrase", () => {
  it("gives a default with its value in code type, required and optional", () => {
    expect(unsetPhrase(variable("SHELF_PORT"))).toEqual(["default ", { code: "8080" }]);
    expect(plain(variable("SHELF_CATALOGUE_URL"))).toBe("required");
    expect(plain(variable("SHELF_MIRROR_ACCOUNT"))).toBe("optional");
  });

  it("gives both rules when production's differs", () => {
    expect(plain(variable("SHELF_MIRROR_TOKEN"))).toBe("optional locally, required in production");
    const port = {
      ...variable("SHELF_PORT"),
      unset_rule: {
        local: { rule: "default" as const, default: "8080" },
        production: { rule: "required" as const },
      },
    };
    expect(unsetPhrase(port)).toEqual([
      "default ",
      { code: "8080" },
      " locally, ",
      "required",
      " in production",
    ]);
  });

  it("falls back to the text of a tool without the unset rule", () => {
    expect(unsetPhrase(textOnly(variable("SHELF_PORT")))).toEqual(["default 8080"]);
    expect(plain(textOnly(variable("SHELF_MIRROR_TOKEN")))).toBe(
      textOnly(variable("SHELF_MIRROR_TOKEN")).unset,
    );
  });
});

describe("sourceWords", () => {
  it("gives the source's kind and the kind of its bind", () => {
    expect(sourceWords(variable("SHELF_PORT"))).toBe("setting");
    expect(sourceWords(variable("SHELF_MIRROR_TOKEN"))).toBe("secret · 1Password");
    expect(sourceWords(variable("TERM"))).toBe("inherited");
  });

  it("falls back to the source's text for a kind a later starmap adds and for an older tool", () => {
    const later = {
      ...variable("SHELF_PORT"),
      source: "derived",
      source_kind: { kind: "other" as const, bind: null },
    };

    expect(sourceWords(later)).toBe("derived");
    expect(sourceWords(textOnly(variable("SHELF_MIRROR_TOKEN")))).toBe(
      variable("SHELF_MIRROR_TOKEN").source,
    );
  });
});

describe("partsOf", () => {
  it("splits out each whole name and prefers the longer of two that start alike", () => {
    expect(
      partsOf("SHELF_PORT_RANGE and SHELF_PORT, not XSHELF_PORT", [
        "SHELF_PORT",
        "SHELF_PORT_RANGE",
      ]),
    ).toEqual([
      { text: "SHELF_PORT_RANGE", variable: true },
      { text: " and ", variable: false },
      { text: "SHELF_PORT", variable: true },
      { text: ", not XSHELF_PORT", variable: false },
    ]);
  });

  it("keeps a sentence whole when it names no variable", () => {
    expect(partsOf("the catalogue has no book 99", ["SHELF_PORT"])).toEqual([
      { text: "the catalogue has no book 99", variable: false },
    ]);
  });
});
