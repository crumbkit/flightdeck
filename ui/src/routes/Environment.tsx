import { cn } from "cn";
import { ChevronDown, ChevronRight } from "lucide-react";
import { Fragment, type ReactNode, useEffect, useRef, useState } from "react";
import type { Variable } from "@/api/model";
import {
  defaultOf,
  environmentSummary,
  isWithheld,
  variablesCount,
  wrongOf,
  wrongWords,
} from "@/lib/environment";

type Props = {
  /** The tool's name, such as "shelf". */
  name: string;
  variables: Variable[];
  /** The variable whose row the view opens on, or null. */
  open: string | null;
};

/** What happens when the variable is unset, with a plain default in code type. */
function Unset({ variable }: { variable: Variable }) {
  const value = defaultOf(variable.unset);
  return value === null ? (
    variable.unset
  ) : (
    <>
      default <span className="font-mono">{value}</span>
    </>
  );
}

/**
 * The value as flightdeck started the tool. A withheld value is eight
 * dots, whatever its length, because the page never holds it.
 */
function Value({ variable }: { variable: Variable }) {
  if (isWithheld(variable)) {
    return (
      <span role="img" aria-label="withheld" title="Withheld" className="tracking-wider">
        ••••••••
      </span>
    );
  }
  const { text } = variable.value;
  return text === null ? (
    <span className="text-muted-foreground font-sans">not set</span>
  ) : (
    <span className="wrap-anywhere">{text}</span>
  );
}

/** The variable's state as its word, in red when it stops the tool. */
function State({ variable }: { variable: Variable }) {
  const wrong = wrongOf(variable) !== null;
  const { state } = variable.value;
  return (
    <span
      className={cn(
        wrong && "text-destructive font-semibold",
        !wrong && state !== "set" && "text-muted-foreground",
      )}
    >
      {state}
    </span>
  );
}

/** One label and its value in an open row. */
function Item({
  label,
  red = false,
  children,
}: {
  label: string;
  red?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-0.5 md:flex-row md:gap-3">
      <dt className="text-muted-foreground shrink-0 md:w-24">{label}</dt>
      <dd className={cn("m-0 max-w-prose min-w-0", red && "text-destructive")}>{children}</dd>
    </div>
  );
}

/** What a row shows when it is open: the variable's purpose, effect, values, check, group and bind. */
function Detail({ variable }: { variable: Variable }) {
  const wrong = wrongOf(variable);
  return (
    <dl className="m-0 flex flex-col gap-2 text-xs md:gap-1.5">
      {variable.purpose !== null && <Item label="Purpose">{variable.purpose}</Item>}
      {variable.effect !== null && (
        <Item label="When unset" red={wrong === "unset"}>
          {variable.effect}
        </Item>
      )}
      {variable.values !== null && variable.values.length > 0 && (
        <Item label="Values">
          <ul className="m-0 flex list-none flex-col gap-0.5 p-0">
            {variable.values.map((each) => (
              <li key={each.value} className="flex gap-3">
                <span className="min-w-12 shrink-0 font-mono">{each.value}</span>
                <span className="text-muted-foreground">{each.description}</span>
              </li>
            ))}
          </ul>
        </Item>
      )}
      {variable.check !== null && (
        <Item label="Check" red={wrong === "invalid"}>
          {variable.check}
        </Item>
      )}
      {variable.group !== null && <Item label="Group">{variable.group}</Item>}
      {variable.bind !== null && (
        <Item label="Bind">
          <span className="font-mono wrap-anywhere">{variable.bind}</span>
        </Item>
      )}
    </dl>
  );
}

/** The button that opens a row: a chevron and the variable's name. */
function NameButton({
  variable,
  open,
  onToggle,
}: {
  variable: Variable;
  open: boolean;
  onToggle: () => void;
}) {
  const Icon = open ? ChevronDown : ChevronRight;
  return (
    <button
      type="button"
      aria-expanded={open}
      onClick={onToggle}
      data-name={variable.name}
      className="hover:text-primary -ml-1 inline-flex min-h-6 items-center gap-1 rounded-xs text-left font-mono font-medium wrap-anywhere"
    >
      <Icon aria-hidden="true" className="text-muted-foreground size-3.5 shrink-0" />
      {variable.name}
    </button>
  );
}

const cell = "px-2.5 py-1.5 text-left align-middle";
const heading = "text-muted-foreground border-b px-2.5 py-1.75 text-left font-medium";
const columns = ["Name", "Type", "When unset", "Source", "Value", "State"];

/** The title of the rows of the variables that crumbkit reads. */
const crumbkitTitle = "Read by crumbkit in every tool";

/**
 * The environment view: every variable that the tool declares, from the
 * tool's `self environment list --full`, with its type, what happens when it
 * is unset, where its value comes from, its value as flightdeck started the
 * tool and its state. A row opens in place to show the variable's purpose,
 * effect, values, check, group and bind. The variables that crumbkit reads
 * in every tool follow the tool's own under their own heading. Below the md
 * width the table becomes one list row per variable. Opened on a variable,
 * the view opens its row, scrolls to it and focuses its name.
 */
export default function Environment({ name, variables, open }: Props) {
  const [opened, setOpened] = useState<Set<string>>(() => new Set(open === null ? [] : [open]));
  const view = useRef<HTMLDivElement>(null);
  const toggle = (variable: string) =>
    setOpened((before) => {
      const next = new Set(before);
      if (!next.delete(variable)) {
        next.add(variable);
      }
      return next;
    });

  useEffect(() => {
    if (open === null) {
      return;
    }
    const buttons = view.current?.querySelectorAll<HTMLElement>(
      `button[data-name="${CSS.escape(open)}"]`,
    );
    // The table and the list each hold the row, and only one is on screen.
    const shown = [...(buttons ?? [])].find((each) => each.offsetParent !== null);
    shown?.scrollIntoView({ block: "nearest" });
    shown?.focus({ preventScroll: true });
  }, [open]);

  const groups: { title: string | null; rows: Variable[] }[] = [
    { title: null, rows: variables.filter((each) => each.owner === null) },
    { title: crumbkitTitle, rows: variables.filter((each) => each.owner !== null) },
  ].filter((group) => group.rows.length > 0);

  return (
    <div ref={view} className="flex min-h-0 flex-1 flex-col overflow-auto">
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1 border-b px-4 py-2.5">
        <h1 className="m-0 text-sm font-semibold">Environment</h1>
        <span className="text-muted-foreground text-xs">{environmentSummary(variables)}</span>
        <span className="text-muted-foreground text-xs md:ml-auto">
          Values are as they were when flightdeck started.
        </span>
      </div>
      <div className="px-4 pt-3 pb-4">
        {variables.length === 0 ? (
          <p className="text-muted-foreground m-0 text-sm">
            {name} declares no environment variables.
          </p>
        ) : (
          <div className="bg-card overflow-hidden rounded-lg border">
            <table
              aria-label="Environment variables"
              className="hidden w-full border-collapse text-xs md:table"
            >
              <thead>
                <tr>
                  {columns.map((label) => (
                    <th key={label} scope="col" className={heading}>
                      {label}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {groups.map((group) => (
                  <Fragment key={group.title ?? "declared"}>
                    {group.title !== null && (
                      <tr>
                        <th
                          colSpan={columns.length}
                          scope="colgroup"
                          className="bg-muted text-muted-foreground border-b px-2.5 py-1.5 text-left font-medium"
                        >
                          {group.title}
                        </th>
                      </tr>
                    )}
                    {group.rows.map((variable) => {
                      const isOpen = opened.has(variable.name);
                      const wrong = isOpen ? null : wrongWords(variable);
                      const more = isOpen || wrong !== null;
                      return (
                        <Fragment key={variable.name}>
                          <tr className={cn(!more && "border-b last:border-b-0")}>
                            <th scope="row" className={cn(cell, "font-normal")}>
                              <NameButton
                                variable={variable}
                                open={isOpen}
                                onToggle={() => toggle(variable.name)}
                              />
                            </th>
                            <td className={cn(cell, "wrap-anywhere")}>{variable.type}</td>
                            <td className={cell}>
                              <Unset variable={variable} />
                            </td>
                            <td className={cell}>{variable.source}</td>
                            <td className={cn(cell, "font-mono")}>
                              <Value variable={variable} />
                            </td>
                            <td className={cell}>
                              <State variable={variable} />
                            </td>
                          </tr>
                          {wrong !== null && (
                            <tr className="border-b last:border-b-0">
                              <td
                                colSpan={columns.length}
                                className="text-destructive px-2.5 pt-0 pb-2 pl-7"
                              >
                                {wrong}
                              </td>
                            </tr>
                          )}
                          {isOpen && (
                            <tr className="border-b last:border-b-0">
                              <td colSpan={columns.length} className="px-2.5 pt-1 pb-3 pl-7">
                                <Detail variable={variable} />
                              </td>
                            </tr>
                          )}
                        </Fragment>
                      );
                    })}
                  </Fragment>
                ))}
              </tbody>
            </table>
            <div className="md:hidden">
              {groups.map((group) => (
                <Fragment key={group.title ?? "declared"}>
                  {group.title !== null && (
                    <h2 className="bg-muted text-muted-foreground m-0 border-y px-3 py-1.5 text-xs font-medium">
                      {group.title}
                    </h2>
                  )}
                  <ul
                    aria-label={group.title ?? `Variables that ${name} declares`}
                    className="m-0 list-none p-0"
                  >
                    {group.rows.map((variable) => {
                      const isOpen = opened.has(variable.name);
                      const wrong = isOpen ? null : wrongWords(variable);
                      return (
                        <li
                          key={variable.name}
                          className="border-b px-3 py-2.25 text-xs last:border-b-0"
                        >
                          <div className="flex items-center justify-between gap-2">
                            <NameButton
                              variable={variable}
                              open={isOpen}
                              onToggle={() => toggle(variable.name)}
                            />
                            <State variable={variable} />
                          </div>
                          <div className="mt-0.5 pl-3.5">
                            <p className="text-muted-foreground m-0">
                              {variable.type} · <Unset variable={variable} /> · {variable.source}
                            </p>
                            <p className="m-0 mt-0.5 font-mono">
                              <Value variable={variable} />
                            </p>
                            {wrong !== null && <p className="text-destructive m-0 mt-1">{wrong}</p>}
                            {isOpen && (
                              <div className="mt-2.5 mb-1">
                                <Detail variable={variable} />
                              </div>
                            )}
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                </Fragment>
              ))}
            </div>
          </div>
        )}
        <p className="text-muted-foreground m-0 mt-2.5 text-xs">
          {variablesCount(variables)} from{" "}
          <code className="font-mono">{name} self environment list --full</code>. Start flightdeck
          again to read the values again.
        </p>
      </div>
    </div>
  );
}
