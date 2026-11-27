// The Rules page of the site (signed artboard 52, as a snapshot with no rail
// and no button that reads the reports again): the headline, the tiles, each
// repository's report, the Kind, Checker, Shown and State filters, the
// sortable table of rules and the counts by checker, by when an agent is
// shown a rule and by kind.
import type { RuleEntry } from "@crumbkit/flightdeck-headless/rules";
import { shortCommit } from "@crumbkit/flightdeck-headless/words";
import {
  CHECKER_CHIPS,
  KIND_CHIPS,
  SHOWN_CHIPS,
  STATE_CHIPS,
  useRulesOverview,
} from "@crumbkit/flightdeck-react/useRulesOverview";
import { counted } from "@crumbkit/ui/a11y";
import type { PressedButtonProps } from "@crumbkit/ui/react";
import { useId, useMemo, useRef } from "react";
import type { PageProps } from "./CompliancePage.tsx";
import { RunLink, StateChip } from "./parts.tsx";

/** One set of chips: its label and its chips. */
function Filter<T extends string>({
  label,
  chips,
  getProps,
}: {
  readonly label: string;
  readonly chips: readonly { readonly option: T; readonly label: string }[];
  readonly getProps: (option: T) => PressedButtonProps;
}) {
  const id = useId();
  return (
    <div className="cs-row cs-nowrap">
      <span className="cs-cap" id={id}>
        {label}
      </span>
      {/* biome-ignore lint/a11y/useSemanticElements: a group of toggle buttons. */}
      <div className="cs-seg" role="group" aria-labelledby={id}>
        {chips.map((chip) => (
          <button key={chip.option} {...getProps(chip.option)}>
            {chip.label}
          </button>
        ))}
      </div>
    </div>
  );
}

/** The repositories a rule reaches, as the Rules table names them. */
function Reach({ rule }: { readonly rule: RuleEntry }) {
  if (rule.repositories.startsWith("all ") || rule.reached.length === 0) {
    return <span className="cs-cap">{rule.repositories}</span>;
  }
  if (rule.reached.length > 2) {
    return (
      <>
        <span className="cs-mono" style={{ fontSize: 12 }}>
          {rule.reached[0]}
        </span>{" "}
        <span className="cs-cap">and {rule.reached.length - 1} more</span>
      </>
    );
  }
  return (
    <span className="cs-mono" style={{ fontSize: 12 }}>
      {rule.reached.join(", ")}
    </span>
  );
}

/** Returns the glyph of a sorted column's direction, or nothing for a column the table is not sorted by. */
function glyph(direction: "ascending" | "descending" | undefined): string {
  if (direction === undefined) return "";
  return direction === "ascending" ? " ▾" : " ▴";
}

/** The Rules page of `document`, with its filters and sort in `search`. */
export function RulesPage({ document, search, navigate }: PageProps) {
  const source = useMemo(() => ({ search, navigate }), [search, navigate]);
  const page = useRulesOverview(document, source);
  const view = page.view;
  const table = useRef<HTMLElement | null>(null);
  const imposed = document.imposed;
  const days = counted(document.stale_after_days, "day");
  const stateHeader = page.getHeaderProps("state");
  const ruleHeader = page.getHeaderProps("rule");
  const sorted =
    view.sort.key === "state"
      ? `Sorted by state, ${view.sort.direction === "ascending" ? "failing" : "stated"} first`
      : `Sorted by rule, ${view.sort.direction === "ascending" ? "A to Z" : "Z to A"}`;
  return (
    <>
      <div>
        <div className="cs-sub">Rules</div>
        <h1>{view.headline}</h1>
        <div className="cs-row">
          <span className="cs-cap">
            Against {imposed.name} in <span className="cs-mono">{imposed.from}</span>
            {imposed.commit === null ? null : (
              <>
                {" "}
                at <span className="cs-mono">{shortCommit(imposed.commit)}</span>
              </>
            )}
            . Each repository's own main run publishes its report, and this page shows the newest
            one at the snapshot.
          </span>
        </div>
      </div>

      <div className="cs-tiles">
        {view.tiles.map((tile) => (
          <button
            key={tile.key}
            type="button"
            className="cs-tile"
            onClick={() => {
              table.current?.scrollIntoView();
              table.current?.focus();
            }}
          >
            <span>{tile.label}</span>
            {tile.key === "stated" ? (
              <span className="cs-big">{tile.count}</span>
            ) : (
              <span className="cs-row">
                <span className="cs-big">{tile.count}</span>
                <StateChip word={tile.key} />
              </span>
            )}
          </button>
        ))}
      </div>

      <section className="cs-card" aria-labelledby="reports-heading">
        <div className="cs-head">
          <h2 className="cs-lab" id="reports-heading">
            Reports by repository · {document.repositories.length} configured
          </h2>
          <span className="cs-cap">
            A report older than {days} is stale. A repository with no report gives its rules no
            state.
          </span>
        </div>
        <table>
          <thead>
            <tr>
              <th scope="col">Repository</th>
              <th scope="col">Source run</th>
              <th scope="col">Commit</th>
              <th scope="col">Report age</th>
              <th scope="col">Rules</th>
              <th scope="col" style={{ width: 96 }}>
                Report
              </th>
            </tr>
          </thead>
          <tbody>
            {view.reports.map((report) => {
              const column = document.repositories.find((each) => each.name === report.name);
              return (
                <tr key={report.name}>
                  <td className="cs-mono" style={{ fontSize: 13 }}>
                    {report.name}
                  </td>
                  <td>
                    {report.run === null ? (
                      <span className="cs-dash">–</span>
                    ) : (
                      <RunLink column={column} label={`main ${report.run}`} />
                    )}
                  </td>
                  <td>
                    {report.commit === null ? (
                      <span className="cs-dash">–</span>
                    ) : (
                      <span className="cs-mono" style={{ fontSize: 12 }}>
                        {report.commit}
                      </span>
                    )}
                  </td>
                  <td>
                    {report.age === null || column?.generated == null ? (
                      <span className="cs-dash">–</span>
                    ) : (
                      <time className="cs-num" dateTime={column.generated} title={column.generated}>
                        {report.age}
                      </time>
                    )}
                  </td>
                  <td className="cs-num">{report.rules ?? "–"}</td>
                  <td>
                    <StateChip word={report.word} />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </section>

      <div className="cs-segs">
        <Filter label="Kind" chips={KIND_CHIPS} getProps={page.getKindProps} />
        <Filter label="Checker" chips={CHECKER_CHIPS} getProps={page.getCheckerProps} />
        <Filter label="Shown" chips={SHOWN_CHIPS} getProps={page.getShownProps} />
        <Filter label="State" chips={STATE_CHIPS} getProps={page.getStateProps} />
      </div>

      <section
        className="cs-card"
        aria-labelledby="rules-heading"
        tabIndex={-1}
        ref={(element) => {
          table.current = element;
        }}
      >
        <div className="cs-head">
          <h2 className="cs-lab" id="rules-heading">
            Rules · {view.rules.length}
          </h2>
          <span className="cs-cap">{sorted}</span>
        </div>
        <table className="cs-rt">
          <thead>
            <tr>
              <th {...ruleHeader}>
                <button {...page.getSortButtonProps("rule")}>
                  Rule{glyph(ruleHeader["aria-sort"])}
                </button>
              </th>
              <th scope="col" style={{ width: 92 }}>
                Kind
              </th>
              <th scope="col" style={{ width: 210 }}>
                Checker
              </th>
              <th scope="col" style={{ width: 70 }}>
                Shown
              </th>
              <th scope="col" style={{ width: 190 }}>
                Repositories
              </th>
              <th {...stateHeader} style={{ width: 200 }}>
                <button {...page.getSortButtonProps("state")}>
                  State{glyph(stateHeader["aria-sort"])}
                </button>
              </th>
            </tr>
          </thead>
          <tbody>
            {page.rules.map((rule, index) => (
              <tr key={rule.key} {...page.getRuleRowProps(index)}>
                <td>
                  <span>{rule.name}</span>
                  <span className="cs-sub2">{rule.id}</span>
                </td>
                <td className="cs-kind">{rule.kind}</td>
                <td className="cs-kind">{rule.checker}</td>
                <td>
                  <span className="cs-mono" style={{ fontSize: 12 }}>
                    {rule.shown}
                  </span>
                </td>
                <td>
                  <Reach rule={rule} />
                </td>
                <td>
                  <StateChip word={rule.word} />
                  {rule.caption === null ? null : (
                    <span className="cs-cap" style={{ display: "block", marginTop: 2 }}>
                      {rule.caption}
                    </span>
                  )}
                </td>
              </tr>
            ))}
            {page.hidden === 0 ? null : (
              <tr>
                <td colSpan={6}>
                  <button className="cs-ghost" {...page.getMoreButtonProps()}>
                    {page.moreLabel} ▸
                  </button>
                </td>
              </tr>
            )}
          </tbody>
        </table>
        <div className="cs-row">
          <span className="cs-cap">{view.note}</span>
        </div>
      </section>

      <div className="cs-cols3">
        <section className="cs-card cs-list" aria-labelledby="checkers-heading">
          <h2 className="cs-lab" id="checkers-heading">
            Checkers · what each one checks
          </h2>
          {view.checkers.map((entry) => (
            <div className="cs-item" key={entry.key}>
              <div>
                {entry.label}
                <div className="cs-cap">{entry.meaning}</div>
              </div>
              <span className="cs-num">{entry.count}</span>
            </div>
          ))}
        </section>
        <section className="cs-card cs-list" aria-labelledby="shown-heading">
          <h2 className="cs-lab" id="shown-heading">
            When an agent is shown a rule
          </h2>
          {view.shown.map((entry) => (
            <div className="cs-item" key={entry.key}>
              <div>
                <span className="cs-mono">{entry.label}</span>
                <div className="cs-cap">{entry.meaning}</div>
              </div>
              <span className="cs-num">{entry.count}</span>
            </div>
          ))}
        </section>
        <section className="cs-card cs-list" aria-labelledby="kinds-heading">
          <h2 className="cs-lab" id="kinds-heading">
            Kinds of rule
          </h2>
          {view.kinds.map((entry) => (
            <div className="cs-item" key={entry.key}>
              <div>
                {entry.label}
                <div className="cs-cap">{entry.meaning}</div>
              </div>
              <span className="cs-num">{entry.count}</span>
            </div>
          ))}
        </section>
      </div>

      <div className="cs-cap">
        Read from the newest report of each repository that the build's config names. Each report is
        the output of <span className="cs-mono">board rules report</span> in that repository's main
        run. Flightdeck stores nothing.
      </div>
    </>
  );
}
