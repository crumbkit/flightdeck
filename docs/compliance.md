# The Compliance site

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
    and State chips.
  - `#/repositories/<name>`: one repository's page. It shows its report,
    its measured rules, its rules with nothing to measure, its suites, its
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
