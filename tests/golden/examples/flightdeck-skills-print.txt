---
name: flightdeck
description: Open a local web page that lists, runs and draws the commands of a crumbkit tool, and read which commands agents run badly. Use it when a person wants to browse or run a crumbkit tool's commands in a browser, or to see how agents use the tool.
---

# flightdeck

flightdeck starts a local web page for any command-line tool built on
crumbkit. The page lists the tool's commands grouped by goal, draws a form
for each one, runs the command and draws its JSON result. flightdeck reads
only the JSON documents that the tool prints about itself, so it works with
any crumbkit tool without being built for it.

## When to open a page

Open a tool's page when a person wants to see what a tool can do, fill in a
command from a form rather than type it, look at a result as a table, or
review how agents use the tool. Run:

```sh
flightdeck open TOOL
```

TOOL is the name of a program on PATH, such as `shelf`, or a path to one,
such as `./target/debug/shelf`. flightdeck runs the tool's `--version`,
`commands list`, `schemas list` and `schemas show` to load it, so the tool
must be built on crumbkit.

## Start it where the person can see it

`flightdeck open` does not return while the page is open. It serves the page
until the person presses Ctrl-C, until it receives SIGTERM, or until 30
minutes pass without a request from the page. Start it in a terminal that
the person can see and stop, not as a background job of your own session.

When it starts, it prints two lines on standard error: the page's address
with the tool's name, version and number of commands, and a line that says
it is opening the browser and that Ctrl-C stops it. It prints no result on
standard output. When it stops after the idle limit, it prints a line that
says so.

## A person at another machine: `--no-browser`

With `--no-browser`, flightdeck opens no browser. It prints the path of an
opener page, a local file that sends the browser to the page's address with
a one-time launch code in the URL's fragment, the part after `#`. The person
opens that file in a browser. If the browser does not open without the flag,
flightdeck prints the same path.

A bare `http://127.0.0.1:PORT` address will not sign in, because it carries
no launch code. The code works once and expires 60 seconds after flightdeck
starts, so the person opens the page soon after the command prints it. To
sign in again, stop flightdeck and run `flightdeck open` again.

The server listens only on 127.0.0.1 and refuses a request whose `Host`
header is not `127.0.0.1:PORT`. A person at another machine therefore needs
a port forward that keeps the same port number, such as
`ssh -L PORT:127.0.0.1:PORT`, and then opens the address with its fragment
that the opener page holds.

## `--read-only`

`flightdeck open TOOL --read-only` lets the page run only the commands that
the tool declares read-only. The page shows a lock in its top bar and dims
the other commands. Use it when the person only wants to look, or when the
tool's other commands change things that matter.

## Destructive commands

A command that the tool declares destructive, or one that declares no
effects, has Preview in place of Run. Preview runs the command's dry run and
shows what it would change. The person then confirms in the same card, and
the command runs once. A confirmation lasts 10 minutes, and a change to the
form after the preview disables it. flightdeck itself refuses such a run
without a confirmation, so nothing on the page skips this step.

The page's command line drops `--yes` and `--dry-run` from a pasted line,
with a note, so a pasted `--yes` never skips the confirmation. It also drops
`--json`, `--full` and `--no-input`, which flightdeck adds itself.

## Find commands that agents run badly

When the tool keeps a usage log, the page's top bar has a Usage view. Its
Figures tab lists each command's runs, failures, refused command lines, help
pages read before it, and its wall time and output size. A command with many
failures or refusals, or one that agents read help for before most runs, is
one they run badly. A command's cost card shows the tool's findings about it,
such as a retry after a failure.

The Weight tab shows the bytes of help a reader passes to reach each command,
and Both sets that weight beside the command's runs, so it picks out heavy
commands that run often. When the tool lists its runs, the Steps, Sessions
and Flags tabs show which command followed which, each session's runs, and
which flags each command's runs passed.

## When `flightdeck open` fails

Each failure prints a message and a code, and exits with 1.

- `flightdeck::tool::not_found`: flightdeck cannot start TOOL. The name is
  not a program on PATH, the path does not exist, or the program will not
  start. Give the name of a program on PATH, or its path.
- `flightdeck::tool::load`: TOOL started, but flightdeck cannot read its
  description. The failure names the query, such as
  `commands list --json --full --all --no-input`, and its `problem` field
  says what went wrong: `exit` (the query exited with a code other than 0),
  `start`, `time_limit` (it did not finish within 10 seconds),
  `stdout_limit` (it printed more than 16 MiB), `not_document` (it printed
  something other than the crumbkit document it names, as a tool not built
  on crumbkit does) or `read`. Run the named query yourself to see what it
  prints. The code also covers a temporary state directory that flightdeck
  cannot make; free space in the system's temporary directory and run
  flightdeck again.
- `flightdeck::tool::search_path`: TOOL is a bare name and an entry of PATH
  cannot be made absolute. Give the tool's path, or run flightdeck from
  another directory.
- `flightdeck::server::failed`: the page's server could not start or
  stopped answering. flightdeck could not listen on 127.0.0.1, read the
  system's random source, write the opener page or watch for Ctrl-C. Check
  that flightdeck may listen on 127.0.0.1 and that the temporary directory
  is writable, then run it again.

A run on the page can fail with a `flightdeck::run::` or
`flightdeck::choices::` code, which the page shows on the run's card.
`flightdeck::run::read_only` means the page was opened with `--read-only`,
and `flightdeck open TOOL` without it runs the command.
`flightdeck::run::busy` means eight runs are already running.
