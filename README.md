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

`flightdeck open <TOOL>` starts the page's server, its sign-in and the
routes that run the tool's commands, and serves the page. The page's code is
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
- The browser signs in with a one-time launch code that the opener page
  holds in the URL's fragment, and exchanges it for a token that every API
  request carries. `GET /api/model` returns the tool's model as JSON.
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
- `flightdeck commands list`, `flightdeck schemas list` and
  `flightdeck schemas show`, crumbkit's own commands, describe flightdeck
  itself. [docs/commands.md](docs/commands.md) is the reference.
- The library's `model` module loads a crumbkit tool by name or path. It
  runs the tool's `--version`, `commands list`, `schemas list` and
  `schemas show` and joins them into a model of the tool's runnable
  commands, their arguments, the goal headings that group them, the
  documents they print with their JSON Schemas, and the links between
  them. It sets crumbkit's own flags, such as `--json` and `--limit`, apart
  from the tool's own arguments. It refuses a tool whose name, command
  words or argument names hold a control character. When the tool mounts
  `self figures list`, the model also counts how often the tool's usage log
  saw each command run. When that call fails, every count stays 0 and the
  tool still loads. `commands list`, `schemas list`, `schemas show` and
  `self figures list` keep no count, because the figures also count the
  runs that flightdeck makes to load the tool.
- The page in `ui/` is a React application. It lists the tool's commands
  grouped by goal, finds a command with ⌘K, builds a run from a form, and
  shows each run as a card with its result as a table or a record. A
  stream's rows appear as they arrive, under the time since the run
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
  because the tool asks for `--yes` opens the same card. Under
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
  runs a shell. It delivers each line of standard output as it arrives and
  ends with an outcome. A destructive command, or one that declares no
  effects, runs only with a confirmation id for the same vector, which a
  preview issues once its dry run succeeds. A cancel sends SIGINT, SIGTERM
  and SIGKILL to the child's process group in turn.

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

Commit `ui/dist` with the change. CI's `gate` job fails when the committed
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
