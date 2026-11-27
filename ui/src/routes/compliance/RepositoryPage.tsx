// One repository's page of the site (signed artboard 55, the snapshot of
// artboard 47): its measured rules, zero rules, suites and written rules,
// the detail of the rule the address selects, how the states are read, and
// where the report came from.
import { ageWord, moment, shortCommit } from "@crumbkit/flightdeck-headless/words";
import { useRepositoryCompliance } from "@crumbkit/flightdeck-react/useRepositoryCompliance";
import { useMemo } from "react";
import { placeHash } from "@/compliance-site/address";
import type { PageProps } from "./CompliancePage.tsx";
import { RunLink, StateChip } from "./parts.tsx";

/** The repository page of `name` in `document`. */
export function RepositoryPage({
  document,
  name,
  search,
  navigate,
}: PageProps & { readonly name: string }) {
  const source = useMemo(() => ({ search, navigate }), [search, navigate]);
  const page = useRepositoryCompliance(document, name, source);
  const view = page.view;
  const column = document.repositories.find((each) => each.name === name);
  if (view === null || column === undefined) {
    return (
      <div>
        <div className="cs-sub">
          <a href={placeHash({ page: "compliance" })}>Compliance</a>
        </div>
        <h1>This snapshot has no repository {name}</h1>
      </div>
    );
  }
  const commit = column.commit === null ? null : shortCommit(column.commit);
  const run = column.source_run;
  const selected = view.selected;
  return (
    <>
      <div>
        <div className="cs-sub">
          <a href={placeHash({ page: "compliance" })}>Compliance</a> /{" "}
          <span className="cs-mono">{view.name}</span>
        </div>
        <h1>{view.headline}</h1>
        <div className="cs-row">
          {column.generated === null ? (
            <span className="cs-cap">{view.subtitle}</span>
          ) : (
            <span className="cs-cap">
              Report
              {run === null ? null : (
                <>
                  {" "}
                  from <RunLink column={column} label={`main run #${run.run_number}`} />
                </>
              )}
              {commit === null ? null : (
                <>
                  {" "}
                  at <span className="cs-mono">{commit}</span>
                </>
              )}
              , made at <time dateTime={column.generated}>{moment(column.generated)}</time>
              {column.age_s === null ? null : `, ${ageWord(column.age_s)} before the snapshot`}
            </span>
          )}
        </div>
      </div>

      <div className="cs-split">
        <div className="cs-colm">
          {view.measured.count === 0 ? null : (
            <section className="cs-card" aria-labelledby="measured-heading">
              <div className="cs-head">
                <h2 className="cs-lab" id="measured-heading">
                  Rules measured{commit === null ? "" : ` at ${commit}`} · {view.measured.count}
                </h2>
                <span className="cs-cap">From the repository's report</span>
              </div>
              <table>
                <thead>
                  <tr>
                    <th scope="col">Rule</th>
                    <th scope="col">Checker</th>
                    <th scope="col">Measured</th>
                    <th scope="col" style={{ width: 80 }}>
                      State
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {view.measured.rows.map((line) => (
                    <tr key={line.key}>
                      <td>
                        <button className="cs-plain" {...page.getRuleButtonProps(line.id)}>
                          {line.name}
                        </button>
                      </td>
                      <td className="cs-kind">{line.checker}</td>
                      <td className="cs-cap">{line.measured}</td>
                      <td>
                        <StateChip word={line.word} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
          )}

          {view.zero.count === 0 ? null : (
            <section className="cs-card" aria-labelledby="zero-heading">
              <div className="cs-head">
                <h2 className="cs-lab" id="zero-heading">
                  Zero rules · {view.zero.count}
                </h2>
                <span className="cs-cap">Read from each suite's newest run</span>
              </div>
              <table>
                <thead>
                  <tr>
                    <th scope="col">Rule</th>
                    <th scope="col">Suite</th>
                    <th scope="col">Measured</th>
                    <th scope="col" style={{ width: 80 }}>
                      State
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {view.zero.rows.map((line) => (
                    <tr key={line.key}>
                      <td>
                        <button className="cs-plain" {...page.getRuleButtonProps(line.id)}>
                          {line.name}
                        </button>
                      </td>
                      <td className="cs-mono" style={{ fontSize: 12 }}>
                        {line.suite}
                      </td>
                      <td className="cs-cap">{line.measured}</td>
                      <td>
                        <StateChip word={line.word} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
          )}

          {view.suites.count === 0 ? null : (
            <section className="cs-card" aria-labelledby="suites-heading">
              <div className="cs-head">
                <h2 className="cs-lab" id="suites-heading">
                  Suites · {view.suites.count}
                </h2>
                <span className="cs-cap">
                  Newest clean full run of each suite that names {view.name}
                </span>
              </div>
              <table>
                <thead>
                  <tr>
                    <th scope="col">Suite</th>
                    <th scope="col">Run</th>
                    <th scope="col">Commits since</th>
                    <th scope="col">Result</th>
                    <th scope="col" style={{ width: 80 }}>
                      State
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {view.suites.rows.map((line) => (
                    <tr key={line.key}>
                      <td>
                        <button
                          className="cs-plain cs-mono"
                          style={{ fontSize: 12 }}
                          {...page.getRuleButtonProps(line.id)}
                        >
                          {line.id}
                        </button>
                      </td>
                      <td className="cs-cap">{line.run}</td>
                      <td className="cs-num">{line.commitsSince ?? "–"}</td>
                      <td className="cs-cap">{line.result}</td>
                      <td>
                        <StateChip word={line.word} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
          )}

          {view.written.count === 0 ? null : (
            <section className="cs-card" aria-labelledby="written-heading">
              <div className="cs-head">
                <h2 className="cs-lab" id="written-heading">
                  Written rules · {view.written.count}
                </h2>
                <span className="cs-cap">Judged by an audit</span>
              </div>
              {view.written.rows.map((line) => (
                <div className="cs-item" key={line.key}>
                  <div>{line.name}.</div>
                  <span className="cs-kind">{line.word}</span>
                </div>
              ))}
            </section>
          )}
        </div>

        <div className="cs-colm">
          {selected === null ? null : (
            <section className="cs-card" {...page.getDetailProps()}>
              <h2 className="cs-lab">{selected.label}</h2>
              <dl className="cs-kv">
                {selected.fields.map((field) => (
                  <div key={field.term} style={{ display: "contents" }}>
                    <dt>{field.term}</dt>
                    <dd>
                      {field.term === "Rule" || field.term === "Suite" ? (
                        <span className="cs-ref">{field.value}</span>
                      ) : (
                        field.value
                      )}
                    </dd>
                  </div>
                ))}
              </dl>
              {selected.over.length === 0 ? null : (
                <table>
                  <thead>
                    <tr>
                      <th scope="col">File over the limit</th>
                      <th scope="col" style={{ textAlign: "right" }}>
                        Used
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {selected.over.map((over) => (
                      <tr key={over.file}>
                        <td className="cs-mono" style={{ fontSize: 12 }}>
                          {over.file.replace(/^\//, "")}
                        </td>
                        <td className="cs-num" style={{ textAlign: "right" }}>
                          {over.used.toLocaleString("en-US")}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
              {selected.behind.length === 0 ? null : (
                <table>
                  <thead>
                    <tr>
                      <th scope="col">Pin behind the minimum</th>
                      <th scope="col" style={{ textAlign: "right" }}>
                        Found
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {selected.behind.map((pin) => (
                      <tr key={`${pin.file}:${pin.line}`}>
                        <td className="cs-mono" style={{ fontSize: 12 }}>
                          {pin.file.replace(/^\//, "")} line {pin.line}
                        </td>
                        <td className="cs-num" style={{ textAlign: "right" }}>
                          {pin.found}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
              <h3 className="cs-lab" style={{ marginTop: 4 }}>
                To check again
              </h3>
              <pre className="cs-code">{selected.recheck.argv.join(" ")}</pre>
              <div className="cs-cap">{selected.recheck.hint}</div>
            </section>
          )}

          <section className="cs-card" aria-labelledby="legend-heading">
            <h2 className="cs-lab" id="legend-heading">
              How the states are read
            </h2>
            {view.legend.map((entry) => (
              <div className="cs-item" key={entry.word}>
                <div>
                  <StateChip word={entry.word} />
                </div>
                <div className="cs-cap" style={{ flex: 1 }}>
                  {entry.meaning}
                </div>
              </div>
            ))}
          </section>
        </div>
      </div>

      <div className="cs-cap">{view.readFrom}</div>
    </>
  );
}
