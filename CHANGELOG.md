# Changelog

This file records the notable changes to the published `crumbkit-flightdeck`
crate. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `flightdeck open <TOOL> [--read-only] [--no-browser]` starts a local
  server for the tool's page on 127.0.0.1, signs the browser in with a
  one-time launch code and a token, serves the tool's model at
  `GET /api/model`, and stops on Ctrl-C, SIGTERM or after 30 idle minutes.
  It fails with `flightdeck::tool::not_found`, `flightdeck::tool::load` or
  `flightdeck::server::failed` when it cannot start. Its first line names
  the page's address first.
- The page's server runs the tool's commands: `POST /api/runs`,
  `POST /api/previews` and `POST /api/confirmations/<id>` answer with a
  stream of JSON lines that ends with an `ended` line, or with
  flightdeck's failure document and 422, 503 or 500, and `POST /api/runs/<id>/cancel` cancels a
  run. A closed connection cancels its run, and a shutdown lets runs finish
  within a grace and cancels the rest. A body that does not arrive in time
  gets 408 with `flightdeck::request::timeout`.

- `flightdeck commands list`, `flightdeck schemas list` and
  `flightdeck schemas show`, crumbkit's own commands, describe flightdeck.
- The `model` module loads a crumbkit tool's description of itself into a
  model of its commands, arguments, goal headings, documents and inferred
  links, and fails with `flightdeck::tool::not_found` or
  `flightdeck::tool::load` when it cannot.
- The `runner` module runs a tool's command from a structured request,
  streams each line of standard output, and ends each run with an outcome.
  It refuses requests that the model does not allow with
  `flightdeck::run::refused`, runs a destructive command only with a
  confirmation id that a preview issues once its dry run succeeds, refuses
  every command that is not read-only in read-only mode, and fails a run
  with `flightdeck::run::no_document`, `flightdeck::run::spawn` or
  `flightdeck::run::output_limit` when the tool gives no document, will not
  start or prints too much. A refusal names the path as a list of words and
  the item, the arity or the earlier positional argument at fault. A run ends a second after its child exits even
  when a process in another session holds its output open, and dropping a
  run cancels it. A shutdown
  lets runs end within a grace, cancels the rest, and refuses new runs with
  `flightdeck::run::closed`.
- The model counts each command's runs from the tool's `self figures list`
  when the tool mounts it. A tool without it, or a failing call, loads with
  every count at 0. The commands that flightdeck runs to load the tool keep
  no count, because the figures include flightdeck's own runs of them.
- `flightdeck open` serves the web page, which the binary embeds from
  `ui/dist`. The page lists the tool's commands grouped by goal, finds one
  with ⌘K, builds a run from a form and shows each run as a card with its
  result as a table or a record.
- The page draws a stream's rows as they arrive, once per animation frame,
  with the time since the run started and a count of its rows. Cancel and
  ⌘. stop a run, which then reads "cancelled · exit 130 · N rows kept" and
  keeps its rows; Esc never cancels. A failure card shows the failure's
  message, cause, code, fields and labels, with its recovery as the
  primary chip and its alternatives as quiet chips, each of which fills
  the form and runs nothing. A batch's failures stand in their keys' places
  among the rows, headed such as "7 (index 1)". A refusal of the line
  underlines the word at fault and marks its field in the form with the
  tool's message. A result's next steps follow its footer as chips, three
  at first and the rest behind "+N more". A stream that breaks mid-run
  keeps its rows and says that the connection to flightdeck broke.
- The page's command line edits the same run as the form, in both
  directions. It colours flags and positional values, fills the form from
  each finished word, rewrites itself in the form's order when the person
  leaves it, marks a word the tool would refuse with crumbkit's words and a
  one-click fix, drops a pasted `$ ` and tool name, and takes out
  `--json`, `--full`, `--no-input`, `--yes` and `--dry-run` with a note.
- The page previews a destructive command, or one that declares no
  effects, and asks for confirmation in the preview's card, which names the
  verb and the count. Cancel has focus, ⌘↩ never confirms, and a change to
  the composer after the preview disables the confirm button. A run that
  fails because the tool asks for `--yes` opens the same card. Under
  `--read-only`, the page keeps the other commands in the rail, dimmed and
  locked, and says why it cannot run them.
- An `id` cell that an inferred link starts from has a dotted underline
  and "inferred" in its tooltip and accessible name, and fills the linked
  command's form without running it. The IDs chooser of a field that an
  inferred link fills lists the source's rows, each labelled by its other
  compact columns, and still takes a typed value.
- `POST /api/choices` runs the read-only source of a link that the model
  inferred and answers with at most 1,000 of its `id` values. It refuses
  any other source with `flightdeck::run::refused`, shares the run cap,
  and answers 502 with `flightdeck::choices::no_listing` or 504 with
  `flightdeck::choices::time_limit` when the source gives no listing within
  eight seconds.
