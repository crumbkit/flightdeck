import { describe, expect, it } from "vitest";
import type { Variable } from "@/api/model";
import { environmentShelf } from "../../tests/fixtures/shelf";
import { defaultOf, environmentSummary, isWithheld, wrongOf, wrongWords } from "./environment";
import { partsOf } from "./variableParts";

const variables = environmentShelf.environment ?? [];

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

  it("reads the local rule first when production differs", () => {
    const localOnly = {
      ...variable("SHELF_MIRROR_ACCOUNT"),
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
  it("is true for a set value without text and false for an unset or shown one", () => {
    expect(isWithheld(variable("SHELF_MIRROR_TOKEN"))).toBe(true);
    expect(isWithheld(variable("SHELF_CATALOGUE_URL"))).toBe(false);
    expect(isWithheld(variable("SHELF_DEBUG"))).toBe(false);
  });
});

describe("defaultOf", () => {
  it("gives the default of a plain default rule only", () => {
    expect(defaultOf("default 8080")).toBe("8080");
    expect(defaultOf("optional")).toBeNull();
    expect(defaultOf("default dev; production required")).toBeNull();
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
