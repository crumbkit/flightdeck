# The Compliance site

## Fetching the reports

- `flightdeck compliance fetch --config <FILE> --reports <DIR>` downloads
  each repository's newest rules report before the build runs.
- For a repository whose source is `github-artifact`, it runs `gh`, found
  on `PATH`, with the token gh already holds. flightdeck never reads,
  prints or passes a token.
  - It lists the newest 10 finished runs of the source's `workflow` on its
    `branch`, and takes the report from the newest run that holds an
    unexpired artifact named `artifact`. The artifact must hold one file.
  - It passes over a run of a `pull_request` or `pull_request_target`
    event, and over a run whose code came from another repository, such
    as a fork's branch that is also named `main`. Such a run is neither
    the report's run nor the newest run.
  - It refuses a run whose address is not a page under
    `https://github.com/`, and the build reads a record holding such an
    address as `invalid_report`.
  - It writes the report to `<DIR>/<owner>/<repo>.json`, and a
    `flightdeck/compliance-fetch/v1` record to
    `<DIR>/<owner>/<repo>.fetch.json`. The record names the newest run,
    the run the report came from, and the commits on the branch since the
    report's commit, which `gh api .../compare` counts.
  - When no listed run holds the artifact, it writes the record and
    removes any older report, so the build reads no report for it.
  - A failed query of GitHub fails that repository's row and leaves its
    files as they were. When the record cannot be written, the fetch puts
    the old report back.
  - A fetch first removes the `.fetch-*` temporary files and directories
    that a killed fetch left in the owner's directory.
- For a `path` source, it copies the file at that path, relative to the
  config's directory, and removes any record beside the report. A `url`
  source fails its row.
- The config may not list both `<owner>/<repo>` and `<owner>/<repo>.fetch`,
  because the report of the second is the record of the first.
- It prints one row per repository, with the failed rows first. A failed
  row makes the command exit with 1. When any source is `github-artifact`
  and `gh` does not start, the command fails with
  `flightdeck::compliance::no_gh` and writes nothing.
- A source may set `every_hours`, how often its controls job runs. The
  pages use it in sentences such as "Its controls job runs daily".
- The build reads each record beside its report:
  - A record whose newest run uploaded no report makes the repository read
    `no_report`. Its cells read `no_report` where the earlier run's report
    held the rule, so the Rules page counts those rules as stale.
  - The document's columns gain `newest_run`, `report_run`,
    `commits_since`, `commits_since_code` and `every_hours`, and each row
    gains the `paths` its rule applies to.
  - `compliance build --rules-branch <NAME>` writes the imposed set's
    branch as `imposed.branch`.
  - A build with no fetch records writes no `newest_run`, `report_run`,
    `commits_since` or `commits_since_code`. It writes `every_hours`,
    `imposed.branch` and `paths` only when the config, the
    `--rules-branch` flag or a report gives them.

## The files the build writes

`flightdeck compliance build --out <DIR>` writes two files:

- `compliance.json`: the `flightdeck/compliance/v1` document.
- `compliance.html`: one page that draws that document with no server.

## The page

- The page holds its script, its styles and the document inside the one
  file. It requests no other file, font or image.
- It opens from disk, in a browser tab, and inside a frame whose policy
  allows inline script and style, and `data:` images, and nothing else.
- It has three pages, reached from the top bar and kept in the address
  after `#`:
  - `#/`: the Compliance page. It shows the headline, the Show and Group
    chips, the count tiles that narrow the matrix, the matrix of rules and
    checks by repository, and the lists of failing checks, repositories
    without a current report and stale checks.
  - `#/rules`: the Rules page. It shows each repository's report with its
    run, commit and age, and the rules table with its Kind, Checker, Shown
    and State chips. A stale report's run says how often its controls job
    runs and how old its newest run is. A repository whose newest run
    uploaded no report names that run and its commit.
  - `#/repositories/<name>`: one repository's page. It shows its report,
    the commits on the branch since the report's run, its measured rules, its rules with nothing to measure, its suites, its
    written rules, the detail of the selected rule and a legend.
- Filters, the sort and the selected rule live in the address, so a reload
  or a shared link opens the same view.
- The page follows the system's light or dark appearance.
- Links to the build run and to each report's GitHub Actions run open in
  a new tab. The page itself makes no request for them.
- When the document is missing or does not match the schema, the page draws
  one line: `flightdeck::compliance::<code>: <message>`. The codes are
  `no_document`, `invalid_document` and `source_failed`.

## Building it

- The page's source is `ui/compliance.html` and `ui/src/compliance-site/`.
- `pnpm run build` in `ui/` writes `ui/dist/compliance.html`, which the
  binary embeds. Its words and views come from
  `@crumbkit/flightdeck-headless` and `@crumbkit/flightdeck-react`
  ([packages.md](packages.md)).
- `compliance build --out` puts the document in the embedded page, in a
  `<script type="application/json" id="compliance-document">` element, with
  every `<` written as `\u003c`.
- When the embedded page has no marker for the document, the command fails
  with `flightdeck::compliance::no_page` after it writes `compliance.json`.
- With `--json`, the printed document's `site` names both files, the page's
  size and the codes of any warnings about the page.

## Size

- Flare, the host that shares one HTML file, takes a file up to 4 MiB.
- When `compliance.html` is over 4 MiB, the command reports the warning
  `flightdeck::compliance::over_flare_cap` in its text output's note and in
  `site.warnings`, and the warning leaves the exit code at 0.
- flightdeck prints at most 1 MiB on standard output. A document too large
  to print, which every document in a page over 4 MiB is in `--json`, fails
  with `flightdeck::output::over_budget` after both files are written, and
  no warning is printed.
  Publish that page as a workflow artifact only.
- The empty page is held under 110 kB brotli by the `compliance page`
  budget in `ui/package.json`.
