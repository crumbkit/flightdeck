# flightdeck

flightdeck is a program in development that starts a local web page for
any command-line tool built on
[crumbkit](https://github.com/crumbkit/crumbkit). The page will list the
tool's commands, draw a form for each one, run the command as a child
process and draw its JSON result.

flightdeck reads only the JSON documents that every crumbkit tool prints
about itself: `commands list`, `schemas list`, `schemas show`, the failure
document and the listing envelope. It links none of the tool's code.

## Status

`flightdeck open [TOOL]` starts the page's server, its sign-in and the
routes that run the tool's commands, and serves the page. Without a tool,
the page shows the mxv workspaces on this machine. The page's code is
in [ui/](ui/), and the binary embeds its built files from `ui/dist`.

What works today:

- `flightdeck open shelf` loads shelf's model, listens on 127.0.0.1 on a
  port the system picks, prints two lines to standard error and opens the
  browser:

  ```text
  http://127.0.0.1:52710 · shelf 0.1.1 · 9 commands
  Opening the browser. Press Ctrl-C to stop flightdeck.
  ```

  With `--no-browser`, the second line names the opener page to open in a
  browser, and flightdeck opens nothing. With `--read-only`, the page may
  run only the tool's read-only commands. The server stops on Ctrl-C, on
  SIGTERM, and after 30 minutes without a request from the page. When it
  stops, it lets each run finish within five seconds and cancels the rest.
  When `FLIGHTDECK_PORT` is set, flightdeck listens on that port in place of
  one the system picks, and it refuses a value that is not a port, an empty
  one included.
- `mxv up` runs flightdeck in a workspace, as [mxv.toml](mxv.toml) declares:
  it builds flightdeck and starts `flightdeck open` on shelf, which
  `scripts/install-test-tools.sh` installs, on the host port that mxv
  holds for `flightdeck`. mxv gives the port through `FLIGHTDECK_PORT`.
  The page signs in only through the opener file, so read the opener's
  path from `mxv logs flightdeck` and open that file in a browser within
  60 seconds of the process starting. After that, run `mxv restart` for a
  new launch code. The address that `mxv ports` reports serves the page
  without a launch code, and the proxy's route names are not addresses
  that flightdeck answers to.
- The browser signs in with a one-time launch code that the opener page
  holds in the URL's fragment, and exchanges it for a token that every API
  request carries. `GET /api/model` returns the tool's model as JSON, and
  `GET /api/weight` returns the bytes of the tool's help pages, which the
  server measures once and keeps. `GET /api/weight?again=1` measures a
  partial measurement again.
  [docs/security.md](docs/security.md) states the threat model, every guard
  of the server and the test that proves it.
- The page runs a command with `POST /api/runs`, previews a destructive
  one with `POST /api/previews`, runs it once with the preview's id with
  `POST /api/confirmations/<id>`, and cancels a run with
  `POST /api/runs/<id>/cancel`. Each run route answers with a stream of
  JSON lines: `started`, a `line` for each line that the tool prints,
  `alive` after 15 seconds without output, and `ended` with the outcome,
  the exit code and the duration. A request that the runner refuses gets
  422 with flightdeck's failure document, a full or stopping runner gets
  503, and a tool that will not start gets 500. A page that closes its
  connection cancels its run. [docs/security.md](docs/security.md) states
  the routes' shapes in full.
- The page lists the values of an argument that an inferred link fills
  with `POST /api/choices`. The request names a link of the model: its
  source command, its target command and the target's argument. The server
  runs the source only when the model inferred that link and the source is
  read-only, with `--all` when the source takes it, and answers with at
  most 1,000 choices, each the value of a row's `id` labelled by the row's
  other compact columns.
- `flightdeck open` without a tool serves the Workspaces page: one card
  per mxv workspace on this machine, with its branch, its checkouts, how
  many of its units run, its routes, or its host ports when this mxv has
  no `proxy status`, and whether it is pinned. A card opens the One
  workspace page, with each checkout's branch, commit and uncommitted
  changes, and each unit's state, ports and route links. A tool's page
  opens them from the top bar's Workspaces button. `flightdeck workspaces
  list` and `flightdeck workspaces show NAME` print the same facts, and
  `GET /api/workspaces` and `GET /api/workspaces/<name>` send them.
  flightdeck runs `mxv list`, `mxv status` and `mxv proxy status`, at
  most four at a time and within 10 seconds, with the program that
  `FLIGHTDECK_MXV` names or `mxv` on PATH. The pages leave out what mxv
  does not supply.
- When Board answers, the rail also shows Workflows, with the number of
  workflow graphs in the board. A workflow graph is a
  `flightdeck/workflow-graph/v1` record at `/workflow-graphs/<slug>`: an
  agent's reading of one workflow, with its lanes, groups, steps, checks
  and edges. The Workflows page draws one graph as a swimlane, one column
  per lane and one band per phase or loop, beside a panel that says who
  acts at the selected step, where it comes from and where it goes next.
  Each open change that the graph's bindings match shows as a dot on its
  step or phase, ringed in violet when it follows another version of the
  flow. On a phone the graph is a run sheet. `flightdeck workflows list`
  and `flightdeck workflows show SLUG` print the same facts, and
  `GET /api/workflows` and `GET /api/workflows/<slug>` send them.
  flightdeck runs `board query` and `board log`, at most two at a time and
  within 10 seconds, with the program that `FLIGHTDECK_BOARD_PROGRAM`
  names or `board` on PATH, in the board that `FLIGHTDECK_BOARD` names or
  the person's own board.
- `flightdeck plugin install` writes the `flightdeck` Board plugin, which
  ships the schema of `flightdeck/workflow-graph/v1` and the read-only
  capability `flightdeck.check`, into Board's plugin directory. Then
  `board plugin enable flightdeck` adds the schema to a board, and
  `board flightdeck check --path /workflow-graphs/<slug>` lists what the
  page would refuse in a graph, such as an edge to an id that names
  nothing.
- `flightdeck commands list`, `flightdeck schemas list` and
  `flightdeck schemas show`, crumbkit's own commands, describe flightdeck
  itself. [docs/commands.md](docs/commands.md) is the reference.
- `flightdeck compliance fetch --config FILE --reports DIR` downloads each
  repository's newest rules report from its GitHub runs through `gh`, and
  writes beside it a record of the newest run, the run the report came
  from and the commits since.
- `flightdeck compliance build --out DIR` writes `compliance.json`, the
  document that joins each repository's rules report, and
  `compliance.html`, one file that draws it with no server: the Compliance
  page, the Rules page and each repository's page. The file loads nothing
  outside itself, so it opens from disk or as a Flare share.
  [docs/compliance.md](docs/compliance.md) describes it.
- The library's `model` module loads a crumbkit tool by name or path. It
  runs the tool's `--version`, `commands list`, `schemas list` and
  `schemas show` and joins them into a model of the tool's runnable
  commands, their arguments, the goal headings that group them, the
  documents they print with their JSON Schemas, and the links between
  them. It reads each query's whole output as one document, so a tool that
  prints its JSON on one line and one that indents it load alike. It sets
  crumbkit's own flags, such as `--json` and `--limit`, apart
  from the tool's own arguments. It refuses a tool whose name, command
  words or argument names hold a control character, or whose command paths
  have a word that is empty or starts with `-`. When the tool mounts
  `self figures list`, the model also reads what each command costs the
  agents that call it: how often the tool's usage log saw it run, fail and
  have its command line refused, the help pages read before it, and the
  median and 95th percentile of its wall time and output. When the tool
  also mounts `self usage`, which runs at the same time, each command gets
  the tool's findings about it, such as a retry after a failure. When it
  mounts `self runs list`, which also runs at the same time, the model
  holds each run of the last 7 days in order within its session: its
  command, the names of the flags it passed, whether it printed help, its
  exit code and failure code, its wall time and its output in bytes. When
  the figures fail, every count stays 0 and the tool still loads; when the
  findings or the runs fail, the figures stay without them. When the tool
  mounts crumbkit-starmap's `self environment list`, which also runs at
  the same time, the model holds each environment variable that the tool
  declares and each that crumbkit reads, with its type, its rule when
  unset, its source, its value and its state. The query runs with the
  environment that the page's runs get, and with `<TOOL>_PROGRAM=flightdeck`
  as the reports of the usage log do. A withheld value never reaches the
  page, and when the query fails the tool loads without an environment.
  flightdeck asks
  `--version`, `commands list`, `schemas list` and `schemas show` with
  `XDG_STATE_HOME` set to a temporary directory, so those runs stay out of
  the person's usage log and the person's own runs of them are counted.
  `self figures list`, `self usage` and `self runs list` read that log, so
  the log records flightdeck's runs of them. On a tool built on crumbkit
  0.3.2 or later, flightdeck runs them with `<TOOL>_PROGRAM=flightdeck`,
  such as `SHELF_PROGRAM` for shelf, and with
  `--exclude-program flightdeck`, so the tool marks those runs as
  flightdeck's and leaves them out of its figures, findings and runs. Every
  command, these three included, then counts the person's and agents' runs,
  and the model reads which commands are crumbkit's own from the `built_in`
  field of `commands list`. On a tool built on an earlier crumbkit, the
  three keep no count and no cost, the runs leave them out, and a command
  counts as crumbkit's own when it sits under the `commands`, `schemas`,
  `self` or `aliases` group.
- The page in `ui/` is a React application. It lists the tool's commands
  grouped by goal, finds a command with ⌘K, builds a run from a form that
  starts with what the command costs an agent when the tool keeps figures
  for it, and shows each run as a card with its result as a table or a
  record. When the tool keeps figures or lists its runs, a Usage view in
  the top bar opens on Figures, a table of every command's runs, failures,
  refusals, help read first, wall time and output, and has a Weight tab of
  the help pages a reader passes to reach each command and its output
  against crumbkit's default budgets. A Both tab sets each command's path
  weight beside its runs, sorted by the tokens agents read, and picks out
  the heavy and light commands that run often or never. When the tool lists its
  runs, the view also shows which command followed which, each session's
  runs and the flags each command's runs passed. It loads in its own file
  the first time it opens.
  When the tool mounts `self environment list`, an Environment view in
  the top bar lists each declared variable with its value as flightdeck
  started the tool, crumbkit's own variables under their own heading, and
  a withheld value as eight dots. A row opens to the variable's purpose,
  effect when unset, values, check, group and bind. The name of a declared
  variable in a failure is a link that opens the view on its row.
  A stream's rows appear as they arrive, under the time since the run
  started and a count of its rows, and Cancel or ⌘. stops the run and
  keeps the rows that came. A failure shows its message, cause, code,
  fields and labels, and its recovery and alternatives as chips that fill
  the form and run nothing. The failures of a batch stand in their keys'
  places among the rows. A refusal of the line underlines the word at
  fault and marks its field in the form. A result's next steps appear as
  chips after its footer, three at first and the rest behind "+N more".
  An `id` cell that an inferred link starts from has a dotted underline,
  and pressing it fills the linked command's form without running it. A
  field that an inferred link fills lists the source's rows as choices,
  and still takes a typed value.
- The page's command line above the form edits the same run as the form.
  It splits the line into words as a POSIX shell does and fills the form
  from each finished word while the line has focus, and it rewrites the
  line in the form's order when the person leaves it or changes a field.
  It underlines a word the tool would refuse, says why in crumbkit's
  words and offers the similar argument or command as a one-click fix. It
  drops a leading `$ ` and the tool's name from a pasted line, and takes
  out `--json`, `--full`, `--no-input`, `--yes` and `--dry-run` with a
  note, so a pasted `--yes` never skips the confirmation. `--long` asks
  for the result's long columns. The page sends the run as structured
  values and never sends the line itself.
- A destructive command, or one that declares no effects, has Preview in
  place of Run. Preview appends a card with crumbkit's dry-run sentence and
  the rows the command would change, or "This command has no preview." for
  a command without `--dry-run`. The card then asks the question with the
  verb and the count, such as "Remove 1 book?", states the effects in
  words, shows the exact line that will run, and offers Cancel, which has
  focus, and a red confirm button, such as "Remove 1 book". ⌘↩ never
  confirms. When the composer changes after the preview, the card marks
  itself stale and disables its confirm button. Confirm runs the command
  once in the same card, which then shows its result. A run that fails
  because the tool asks for `--yes` opens the same card. A command that
  is not destructive and takes `--dry-run` keeps Run and has Preview
  beside it. Its card shows the dry run and asks nothing, and its
  "Preview again" previews again. Preview has no keyboard shortcut, and
  ⌘↩ runs. Under
  `--read-only`, the topbar shows a lock, the commands that are not
  read-only stay in the rail dimmed with a lock, and their composer says
  why in place of its button.

- The library's `runner` module runs one of the tool's commands from a
  structured request: the command's path and a value for each argument by
  its name in the model. It refuses a command that `commands list` does
  not list, an argument the command does not take, a value of the wrong
  kind or outside the argument's possible values, and every crumbkit flag
  except `--limit` and `--all`. It also refuses a value with a control
  character, and a positional value after a positional argument without
  one. Each refusal names the command's path as a list of words, the
  argument, the reason, and, where they apply, the item of a list value,
  what the argument takes and the earlier positional argument. It builds the argument vector itself, with each option as one
  `--name=value` word and the positional values after `--`, and it never
  runs a shell. It adds `--json` to every run, `--full` when the command
  lists it and declares a document or a stream, and `--no-input` when the
  command lists it. The runner leaves `--full` out for a command that
  prints a headed answer or bytes, because their JSON already holds every
  field, and crumbkit revisions before crumbkit/crumbkit#270, earlier than 0.1.0, refuse
  `--full` there. Every crumbkit refuses `--full` on a command that prints
  no result, and `commands list` gives that command no form. Since
  crumbkit/crumbkit#291 its row also has `prints_no_result: true`, and the
  tool's own `commands list` schema declares that field. The runner leaves
  `--full` out for a command marked that way, and adds it to an unmarked
  command without a form, which has no document. On a tool whose
  `commands list` schema does not declare the field, such as one on
  crumbkit 0.1.1, the two commands have the same row, so the runner leaves
  `--full` out for every command without a form, and a run of a command
  with no document whose handler prints rows that are not whole shows only
  the rows' columns. A command whose
  document is prose, has no envelope or holds whole rows keeps `--full`.
  Every released crumbkit accepts `--full` there with a note, and crumbkit
  revisions before crumbkit/crumbkit#270 refuse it.
  It delivers each line of standard output as it arrives and
  ends with an outcome, which it reads from the run's last document. That
  document starts at the last line that starts with `{` and runs to the end
  of the output, so the runner reads a document on one line and one that a
  crumbkit tool that sets `JsonLayout::Pretty` indents across many lines,
  and the page draws both the same way. A stream's items are one to a line
  in either layout. A destructive command, or one that declares no
  effects, runs only with a confirmation id for the same vector, which a
  preview issues once its dry run succeeds. A cancel sends SIGINT, SIGTERM
  and SIGKILL to the child's process group in turn. On Windows a cancel
  ends the job object that the child and its own children run in.

A link is inferred from crumbkit's command grammar. A command whose rows
have an `id` column links it to each other command of the same resource that
takes a positional `<ID>` argument, so shelf's `books list` links its `id`
column to `books show`. The page marks every inferred link with the word
"inferred".

## Install

flightdeck needs Rust 1.99 or later. Install it from GitHub:

```sh
cargo install --locked --git https://github.com/crumbkit/flightdeck crumbkit-flightdeck
```

Or install it from a checkout of this repository:

```sh
cargo install --locked --path .
```

Either way, the binary is named `flightdeck`.

## Development

The tests run shelf and reading, crumbkit's example tools, at a pinned
crumbkit commit. Install them once under `target/test-tools/bin` before you
run the tests:

```sh
scripts/install-test-tools.sh
cargo test
```

A test that needs them fails with the script's name when they are missing.

The page needs Node.js at the version in `ui/.node-version` and pnpm at the
version in `ui/package.json`. Building flightdeck never runs Node, because
the built page is committed in `ui/dist`.

`pnpm dev` cannot reach flightdeck. flightdeck listens on a port that the
system picks, and its server refuses a request whose `Host` or `Origin` is
not its own address, so a development server in front of it is refused.
Develop the page with its tests instead: Vitest for its logic and
components, and the Playwright journeys, which start the real
`flightdeck open` with shelf. To see a change in a browser, build the page,
then build and start flightdeck:

```sh
(cd ui && pnpm run build)
cargo build
target/debug/flightdeck open target/test-tools/bin/shelf
```

The binary embeds `ui/dist` when it is built, so build the page before
flightdeck. After you change the page, run its checks and rebuild it from
`ui/`:

```sh
cd ui
pnpm install --frozen-lockfile
pnpm run lint && pnpm run typecheck && pnpm run knip && pnpm test
pnpm run build && pnpm run size
cargo build && pnpm run acceptance
```

Commit `ui/dist` with the change. CI's `page` job fails when the committed
files differ from a fresh build.
[ui/typescript-standards.toml](ui/typescript-standards.toml) maps the
TypeScript standards that the page follows to the files that meet them.
[rust-standards.toml](rust-standards.toml) maps the Rust standards that
flightdeck follows to the files that meet them.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
