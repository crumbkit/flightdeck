// The Compliance page of the site (signed artboard 54, the snapshot of
// artboard 46): the headline, the Show and Group filters, the count tiles
// that narrow the matrix, the matrix of rules and checks by repository, and
// the lists of failing checks, repositories without a current report and
// stale checks. The headless view supplies every word.

import {
  type CheckItem,
  complianceAddress,
  type MissingItem,
} from "@crumbkit/flightdeck-headless/compliance";
import type { ComplianceDocument } from "@crumbkit/flightdeck-headless/document";
import type { MatrixRow } from "@crumbkit/flightdeck-headless/matrix";
import { shortName } from "@crumbkit/flightdeck-headless/words";
import { useCompliance } from "@crumbkit/flightdeck-react/useCompliance";
import { counted } from "@crumbkit/ui/a11y";
import { Fragment, useMemo } from "react";
import { joinSearch, sharedSource } from "@/compliance-site/address";
import { type Filters, filtersAddress, groupsOf, shownColumns, shownRows } from "./filters.ts";
import { PhraseText, RepositoryLink, RunLink, repositoryHash, StateChip } from "./parts.tsx";

/** What every page of the site is given: the document and its page's query. */
export interface PageProps {
  readonly document: ComplianceDocument;
  readonly search: string;
  readonly navigate: (search: string) => void;
}

/** Returns the path of the document row a matrix row's cell in `column` comes from, which the cell's link selects. */
function cellRule(row: MatrixRow, name: string): string {
  if (row.rows.length === 1) return row.rows[0]?.id ?? row.id;
  const own = row.id.replace("<repo>", shortName(name));
  return row.rows.find((each) => each.id === own)?.id ?? row.rows[0]?.id ?? row.id;
}

/** Returns the last segment of a code, such as "invalid_report" for "flightdeck::compliance::invalid_report". */
function codeName(code: string): string {
  return code.split("::").at(-1) ?? code;
}

/**
 * Returns the code of a cell that its report refused, such as a checker's
 * failure, or null. A column with no report or an invalid one draws its
 * cells as words, so this reads only the cells of a readable report.
 */
function refusalCode(row: MatrixRow, index: number, name: string): string | null {
  const own = row.rows.find((each) => each.id === cellRule(row, name));
  const cell = own?.cells[index];
  if (cell === undefined || cell.state !== "not_readable") return null;
  return cell.code;
}

/** One row of the matrix, with a cell for each repository the filters keep. */
function MatrixLine({
  document,
  row,
  columns,
}: {
  readonly document: ComplianceDocument;
  readonly row: MatrixRow;
  readonly columns: readonly number[];
}) {
  const suite = row.section === "suite";
  return (
    <tr>
      <td className="cs-name">{row.name}</td>
      <td className="cs-kind">
        {suite ? (
          <span className="cs-mono" style={{ fontSize: 12 }}>
            {row.checker}
          </span>
        ) : (
          row.checker
        )}
      </td>
      {columns.map((index) => {
        const cell = row.cells[index];
        const column = document.repositories[index];
        if (cell === undefined || column === undefined) return <td key={index} className="cs-r" />;
        if (cell.state === "absent") {
          return (
            <td key={cell.repository} className="cs-r cs-dash">
              <span aria-hidden="true">{cell.word}</span>
              <span className="cs-visually-hidden">{cell.spoken}</span>
            </td>
          );
        }
        if (column.report === "no_report" || column.report === "invalid_report") {
          return (
            <td key={cell.repository} className="cs-r">
              <span className="cs-cap">{cell.word}</span>
            </td>
          );
        }
        const refused = refusalCode(row, index, column.name);
        return (
          <td key={cell.repository} className="cs-r">
            <a
              href={repositoryHash(column.name, cellRule(row, column.name))}
              aria-label={refused === null ? cell.spoken : `${cell.spoken}, ${refused}`}
            >
              <StateChip word={cell.word} />
            </a>
            {refused === null ? null : (
              <span className="cs-cap cs-mono" aria-hidden="true">
                {refused}
              </span>
            )}
            {cell.caption === null ? null : (
              <span className="cs-cap" aria-hidden="true">
                {cell.caption}
              </span>
            )}
          </td>
        );
      })}
    </tr>
  );
}

/** An item of the Failing or Stale list. */
function CheckLine({
  document,
  item,
}: {
  readonly document: ComplianceDocument;
  readonly item: CheckItem;
}) {
  const column = document.repositories.find((each) => each.name === item.repositories[0]);
  return (
    <div className="cs-item">
      <div>
        {item.repositories.map((name, at) => (
          <Fragment key={name}>
            {at === 0 ? null : ", "}
            <RepositoryLink name={name} rule={item.rule} />
          </Fragment>
        ))}{" "}
        · {item.name}
        <div className="cs-cap">
          <PhraseText phrase={item.phrase} />
        </div>
      </div>
      {item.run === null ? null : <RunLink column={column} label={item.run} className="cs-cap" />}
    </div>
  );
}

/** An item of the list of repositories without a current report. */
function MissingLine({
  document,
  item,
}: {
  readonly document: ComplianceDocument;
  readonly item: MissingItem;
}) {
  const column = document.repositories.find((each) => each.name === item.repository);
  return (
    <div className="cs-item">
      <div>
        <RepositoryLink name={item.repository} /> · {item.word}
        <div className="cs-cap">{item.detail}</div>
        {column?.report === "invalid_report" && column.code !== null ? (
          <div className="cs-cap cs-mono">{column.code}</div>
        ) : null}
      </div>
      {item.run === null ? null : <RunLink column={column} label={item.run} className="cs-cap" />}
    </div>
  );
}

/** The Compliance page of `document`, with its filters in `search`. */
export function CompliancePage({ document, search, navigate }: PageProps) {
  const filters = filtersAddress.parse(search);
  const filtersSearch = filtersAddress.format(filters);
  const source = useMemo(
    () => sharedSource(search, filtersSearch, navigate),
    [search, filtersSearch, navigate],
  );
  const page = useCompliance(document, source);
  const view = page.view;
  const change = (next: Partial<Filters>) =>
    navigate(
      joinSearch(
        complianceAddress.format(complianceAddress.parse(search)),
        filtersAddress.format({ ...filters, ...next }),
      ),
    );
  const columns = shownColumns(document, filters);
  const shownNames = new Set(columns.map((index) => document.repositories[index]?.name));
  const keeps = (names: readonly string[]) => names.some((name) => shownNames.has(name));
  const groups = groupsOf(document);
  const span = columns.length + 2;
  const written = shownRows(view.written.rows, filters, columns);
  const failing = view.failing.items.filter((item) => keeps(item.repositories));
  const stale = view.stale.items.filter((item) => keeps(item.repositories));
  const missing = view.missing.items.filter((item) => shownNames.has(item.repository));
  const imposed = document.imposed;
  return (
    <>
      <div>
        <div className="cs-sub">Compliance</div>
        <h1>{view.headline}</h1>
      </div>

      <div className="cs-row cs-between cs-nowrap">
        <div className="cs-row cs-nowrap">
          <span className="cs-cap" id="show-label">
            Show
          </span>
          {/* biome-ignore lint/a11y/useSemanticElements: a group of toggle buttons, as the Rules page's filters are. */}
          <div className="cs-seg" role="group" aria-labelledby="show-label">
            {page.showOptions.map(({ option, label }) => (
              <button key={option} {...page.getShowProps(option)}>
                {label}
              </button>
            ))}
          </div>
          <span className="cs-cap">Filters run in the page, and they change nothing.</span>
        </div>
        {groups.length === 0 ? null : (
          <div className="cs-row cs-nowrap">
            <span className="cs-cap" id="group-label">
              Group
            </span>
            {/* biome-ignore lint/a11y/useSemanticElements: a group of toggle buttons. */}
            <div className="cs-seg" role="group" aria-labelledby="group-label">
              {["", ...groups].map((group) => (
                <button
                  key={group}
                  type="button"
                  aria-pressed={filters.group === group}
                  onClick={() => change({ group })}
                >
                  {group === "" ? "All" : group}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* biome-ignore lint/a11y/useSemanticElements: the tiles are toggle buttons that narrow the matrix. */}
      <div className="cs-tiles" role="group" aria-label="Filter the matrix by state">
        {view.tiles.map((tile) => {
          const on = filters.tile === tile.key;
          return (
            <button
              key={tile.key}
              type="button"
              className="cs-tile"
              aria-pressed={on}
              onClick={() => change({ tile: on ? "all" : tile.key })}
            >
              <span>{tile.label}</span>
              <span className="cs-row">
                <span className="cs-big">{tile.count}</span>
                <StateChip word={tile.word} />
              </span>
            </button>
          );
        })}
      </div>

      <section className="cs-card" aria-labelledby="matrix-heading">
        <div className="cs-head">
          <h2 className="cs-lab" id="matrix-heading">
            Rules and checks by repository · {counted(columns.length, "repository", "repositories")}
          </h2>
          <span className="cs-cap">
            Under each name: the age of its newest report at the snapshot. Under a suite's state:
            run age · commits since.
          </span>
        </div>
        <div className="cs-scroll">
          <table className="cs-mx">
            <thead>
              <tr>
                <th className="cs-name" scope="col">
                  Rule or check
                </th>
                <th scope="col" style={{ width: 196 }}>
                  Checker
                </th>
                {columns.map((index) => {
                  const head = view.columns[index];
                  const column = document.repositories[index];
                  if (head === undefined || column === undefined) return null;
                  const missingReport =
                    column.report === "no_report" || column.report === "invalid_report";
                  return (
                    <th key={head.name} className="cs-r" scope="col">
                      <a href={repositoryHash(head.name)} aria-label={head.spoken}>
                        {head.name}
                      </a>
                      {missingReport ? (
                        <>
                          <StateChip word={head.caption} />
                          {column.report === "invalid_report" && column.code !== null ? (
                            <span className="cs-cap cs-mono" title={column.code}>
                              {codeName(column.code)}
                            </span>
                          ) : null}
                        </>
                      ) : (
                        <span className="cs-cap">
                          <time
                            dateTime={column.generated ?? undefined}
                            title={column.generated ?? undefined}
                          >
                            {head.caption}
                          </time>
                        </span>
                      )}
                      {head.stale ? <StateChip word="stale" /> : null}
                    </th>
                  );
                })}
              </tr>
            </thead>
            <tbody>
              {view.sections.map((section) => {
                const rows = shownRows(section.rows, filters, columns);
                if (rows.length === 0) return null;
                return (
                  <Fragment key={section.key}>
                    <tr className="cs-grp">
                      <td colSpan={span}>{section.heading}</td>
                    </tr>
                    {rows.map((row) => (
                      <MatrixLine key={row.key} document={document} row={row} columns={columns} />
                    ))}
                  </Fragment>
                );
              })}
              {written.length === 0 ? null : (
                <>
                  <tr className="cs-grp">
                    <td colSpan={span}>{view.written.heading}</td>
                  </tr>
                  <tr>
                    <td className="cs-name" colSpan={2}>
                      <button className="cs-ghost" {...page.getWrittenToggleProps()}>
                        {page.writtenOpen
                          ? `Hide the ${counted(written.length, "written rule")} ▾`
                          : `Show ${counted(written.length, "written rule")} ▸`}
                      </button>
                    </td>
                    <td colSpan={columns.length} className="cs-cap">
                      {view.written.note}
                    </td>
                  </tr>
                </>
              )}
            </tbody>
            {written.length === 0 ? null : (
              <tbody {...page.getWrittenRegionProps()}>
                {written.map((row) => (
                  <MatrixLine key={row.key} document={document} row={row} columns={columns} />
                ))}
              </tbody>
            )}
          </table>
        </div>
        <div className="cs-row">
          <span className="cs-cap">{view.staleNote}</span>
        </div>
      </section>

      <div className="cs-cols2">
        <section className="cs-card cs-list" aria-labelledby="failing-heading">
          <div className="cs-head">
            <h2 className="cs-lab" id="failing-heading">
              Failing · {view.failing.count}
            </h2>
            <span className="cs-cap">Each links to the run that reported it</span>
          </div>
          {failing.map((item) => (
            <CheckLine key={item.key} document={document} item={item} />
          ))}
        </section>
        <div className="cs-colm">
          <section className="cs-card cs-list" aria-labelledby="missing-heading">
            <div className="cs-head">
              <h2 className="cs-lab" id="missing-heading">
                Repositories without a current report · {view.missing.count}
              </h2>
            </div>
            {missing.map((item) => (
              <MissingLine key={item.key} document={document} item={item} />
            ))}
          </section>
          <section className="cs-card cs-list" aria-labelledby="stale-heading">
            <div className="cs-head">
              <h2 className="cs-lab" id="stale-heading">
                Stale checks · {view.stale.count}
              </h2>
              <span className="cs-cap">Oldest first</span>
            </div>
            {stale.map((item) => (
              <CheckLine key={item.key} document={document} item={{ ...item, run: null }} />
            ))}
          </section>
        </div>
      </div>

      <div className="cs-cap">
        Built against {imposed.name} in <span className="cs-mono">{imposed.from}</span>
        {imposed.commit === null ? null : (
          <>
            {" "}
            at <span className="cs-mono">{imposed.commit.slice(0, 7)}</span>
          </>
        )}
        . The snapshot is one file, <span className="cs-mono">compliance.html</span>, with its
        document inside it; whoever hosts it decides who can open it.
      </div>
    </>
  );
}
