# Changelog

This file records the notable changes to the published `crumbkit-flightdeck`
crate. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- flightdeck builds and runs on Windows, and a release attaches a Windows
  binary in a zip archive beside the macOS and Linux archives. On Windows a
  tool and every process it starts run in a job object, and a cancel, a
  dropped run or an output limit ends the job at once, without the SIGINT
  and SIGTERM steps that Unix gets. `flightdeck open` on Windows prints the
  opener page's path for you to open, because flightdeck has no Windows
  browser opener yet.
- A command's form shows what the command costs the agents that call it,
  from the tool's own `self figures list` and `self usage`: the runs and
  the days they cover, the 95th percentile of its wall time and of its
  output in bytes and about how many tokens, how many runs failed, the
  calls an agent could have saved, and a sentence for each slow command,
  help loop, most retried failure and heavy output that the tool found.
  Each figure shows only when the tool reports it, and the form grows to
  show the card, its fields and Run together. `GET /api/model` gives each
  command a `cost` field, whose findings each name their `kind`, and the
  tool a `figures_since` day. The loader runs `self figures list` with
  `--full`, and runs `self usage --json --full --no-input` at the same time
  when the tool mounts it. When `self usage` fails, the card shows the
  figures without findings.

### Changed

- `GET /api/model` gives each command a `class` field, which is `read_only`,
  `additive` or `destructive`. The server decides it from the command's
  effects: a command that declares itself both read-only and destructive is
  destructive, and a command that declares no effects is destructive. The
  runner and the page both read this field. Before, the page counted a command
  that declares both as read-only and offered Run, where the server asked for a
  confirmation or refused it under `--read-only`.
- The library has one `clock::Clock`, one `clock::SystemClock` and one
  `clock::Moment`, which the runner and the server share. They replace
  `runner::Clock`, `runner::SystemClock`, `server::Clock`,
  `server::SystemClock` and `server::Moment`. The runner reads
  `moment.instant()` from the clock it is given.
- flightdeck starts, reads and stops every child through crumbkit-process
  from crumbkit's repository, in place of its own process code. Runs and
  loads behave as before on macOS and Linux, with two exceptions. A load
  whose tool leaves running a process that left its process group and
  holds the tool's output open now ends one second after the tool exits,
  with the tool's own exit, where before it waited for the time limit and
  failed with `time_limit`. A run of a program named without a path that
  cannot start, such as a name that is not on `PATH`, now ends with
  `flightdeck::run::spawn` as its outcome, where before the run was refused
  with that code; a program given as a path that is missing or may not be
  run is still refused.

- The page has a new look. The dark appearance has cards a step lighter
  than the background, 3-pixel corners on cards and controls, and thin
  lines in place of shadows. One cyan marks links, focus and the filled
  buttons, which are Run and the chips that open a next or recovery
  command; the light appearance uses a deep blue in its place. The selected
  command in the rail has the accent's fill and a cyan edge. Words are set in IBM Plex Sans
  and commands, arguments and figures in JetBrains Mono, both served by
  flightdeck itself rather than a font service. The rail names each effect
  in a word without a coloured dot. The tool's version appears once, in the
  header. Numeric columns in a table, and their headings, are aligned to the
  right. On a 375-pixel phone the header wraps, nothing scrolls sideways,
  each example in the welcome takes its own line, and a table's column
  descriptions are hidden from sight but still read by a screen reader.
- With a form open, the command line and form take at most 40% of the
  window. The form scrolls inside that space while the Run button stays in
  view. After a run, the page scrolls the new card into view above the
  form, with the card's top showing. This includes a run the server
  refuses and a run that ends with no rows. When the card's rows run past
  the bottom of the view, the New rows button appears.

- flightdeck builds on crumbkit 0.2.0 from crates.io, including
  crumbkit-process, which it took from a commit of crumbkit's repository
  before. A run whose child exits while the page has not read the run's full
  queue of lines now ends within one second of the child's exit, where it
  waited until the page read or the run was stopped. The run keeps the
  child's last line, so a one-line document still ends the run. A page more
  than half a second behind at the child's exit loses lines from the middle
  of the output, and the run still ends as a result with no marker of the
  gap. An indented document longer than the queue, read by a page that has
  fallen behind, ends as a failure with no document.
- `commands list --json --full` gives the `open` row `prints_no_result: true`, and
  `schemas show` gains an optional `next` property and a shorter
  `cut.code` description, from crumbkit 0.2.0.

### Fixed

- The page opens at the top of the welcome. Before, the transcript
  scrolled to its end as soon as it appeared, so a phone opened the page at
  the end of the command list.
- A run of a crumbkit tool that sets `JsonLayout::Pretty` ends with its
  result, or with its failure and the failure's code, and the page draws
  its result as a table or a record as it does for a document on one line.
  The runner and the page now read a run's last document from the last
  line that starts with `{` to the end of the output. Before, they read
  only the last line, so such a run ended with
  `flightdeck::run::no_document`.
- A run of a command that does not offer `--full` reaches its result.
  crumbkit revisions before crumbkit/crumbkit#270, earlier than 0.1.0, refuse `--full` with
  `<tool>::cli::usage` and exit code 2 on a command whose JSON already
  holds every field, and the runner added `--full` to every run, so such a run failed before the tool
  did any work. The runner now adds `--full` only when the command's row in
  `commands list` lists it, as the command's own argument or a global one
  that it inherits, and the command does not print a headed answer or
  bytes, whose JSON already holds every field. It adds `--no-input` only when the
  command lists it too, because a tool that cannot ask a question need not
  declare it. It still adds `--json` to every run.
- A run of a command that prints no result reaches its result. Every
  crumbkit refuses `--full` on a command declared with `prints_no_result`,
  and `commands list` gives such a command no form. The runner added
  `--full` to it when its row listed `--full`. Since crumbkit/crumbkit#291
  the command's row has `prints_no_result: true`, and the tool's own
  `commands list` schema declares the field. The runner now leaves `--full`
  out for a marked command. On a tool whose schema does not declare the
  field, such as one on crumbkit 0.1.1, a command that prints no result and
  a command with no document have the same row, so the runner leaves
  `--full` out for every command without a form. A run of a command with
  no document whose handler prints enveloped rows that are not whole then
  shows only the rows' columns. On a tool that marks such commands, an
  unmarked command without a form keeps `--full`.

## [0.1.0] - 2026-11-04

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

[Unreleased]: https://github.com/crumbkit/flightdeck/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/crumbkit/flightdeck/releases/tag/v0.1.0
