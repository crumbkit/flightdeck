# Commands of `flightdeck`

This reference is generated from the help of `flightdeck`. Edit the help in the tool's code, and not this file.

## `flightdeck`

Start a local web page for a crumbkit tool

### Commands

- [`flightdeck open`](#flightdeck-open): Open a local web page of the mxv workspaces and a tool's commands
- [`flightdeck workspaces`](#flightdeck-workspaces): Read the local mxv workspaces
- [`flightdeck compliance`](#flightdeck-compliance): Build the compliance document from rules reports
- [`flightdeck commands`](#flightdeck-commands): Explore flightdeck's commands
- [`flightdeck skills`](#flightdeck-skills): Give coding agents flightdeck's skill
- [`flightdeck completion`](#flightdeck-completion): Set up flightdeck's completion in a shell
- [`flightdeck schemas`](#flightdeck-schemas): Explore the JSON documents that flightdeck prints
- [`flightdeck self`](#flightdeck-self): Review flightdeck itself

### Arguments

- `--json`: Print the result as a JSON document. The default prints text. Works after every subcommand.
- `--fields <FIELD>`: Print only these comma-separated fields, in this order. The default prints the compact columns. Takes more than one value. Works after every subcommand.
- `--long`: Add the long columns after the compact ones. The default prints only the compact columns. Works after every subcommand.
- `--full`: With --json, print every field of every row. The default prints only the fields of the columns. Works after every subcommand.
- `--no-input`: Fail instead of asking a question. Works after every subcommand.

## `flightdeck open`

Open a local web page of the mxv workspaces and a tool's commands

### Arguments

- `<TOOL>`: The crumbkit tool, by a name on PATH or a path; without one, the page shows the workspaces
- `--read-only`: Run only the tool's read-only commands from the page
- `--no-browser`: Print the path of the page to open instead of opening the browser

### Examples

Open the Workspaces page in the browser.

```sh
flightdeck open
```

Open the page for shelf in the browser.

```sh
flightdeck open shelf
```

Serve a built shelf's read-only commands and print the page to open.

```sh
flightdeck open ./target/debug/shelf --read-only --no-browser
```

Effects: read-only, open-world.

### Exit codes

- `130`: A second Ctrl-C while flightdeck stopped ended it at once and left the opener page behind.
- `143`: A second SIGTERM while flightdeck stopped ended it at once and left the opener page behind.

## `flightdeck workspaces`

Read the local mxv workspaces

### Commands

- [`flightdeck workspaces list`](#flightdeck-workspaces-list): List the local mxv workspaces with their units, routes and ports
- [`flightdeck workspaces show`](#flightdeck-workspaces-show): Show one local mxv workspace with its checkouts and units

## `flightdeck workspaces list`

List the local mxv workspaces with their units, routes and ports

### Examples

List the local mxv workspaces.

```sh
flightdeck workspaces list
```

Print the document that the Workspaces page draws.

```sh
flightdeck workspaces list --json --full
```

### Columns

- `name`: The workspace's name.
- `branch`: The branch that the workspace's checkouts were created on, or null when mxv cannot read the workspace's record.
- `running`: The number of units that are running, or null when `mxv status` failed.
- `declared`: The number of units that the workspace's manifest declares, or null when `mxv status` failed.
- `pinned`: Whether `mxv reclaim` leaves the workspace running, or null when `mxv status` failed.

### Columns that `--long` adds

- `checkouts`: The folder of each checkout, which is its repository's name.
- `problem`: Why mxv cannot read the workspace's record, or null when it can.

Effects: read-only, closed-world.

## `flightdeck workspaces show`

Show one local mxv workspace with its checkouts and units

### Arguments

- `<NAME>`: The workspace's name, as `flightdeck workspaces list` prints it. Required.

### Examples

Print the document that the page of the workspace reading-list draws.

```sh
flightdeck workspaces show reading-list --json
```

Effects: read-only, closed-world.

## `flightdeck compliance`

Build the compliance document from rules reports

### Commands

- [`flightdeck compliance build`](#flightdeck-compliance-build): Join each repository's rules report into one compliance document

## `flightdeck compliance build`

Join each repository's rules report into one compliance document

### Arguments

- `--config <FILE>`: The flightdeck/compliance-config/v1 file. Required.
- `--reports <DIR>`: The directory of the reports, one at \<owner\>/\<repo\>.json each. Required.
- `--out <DIR>`: Also write \<DIR\>/compliance.json
- `--build-run <LABEL>`: The label of the run that builds the pages
- `--build-url <URL>`: The address of that run
- `--rules-commit <SHA>`: The imposed set's current commit
- `--built-at <TIME>`: The build's time, such as 2026-11-25T06:00:00Z; now when left out

### Examples

Print one line per repository with its report's time and its failing, stale and drift counts.

```sh
flightdeck compliance build --config compliance.toml --reports reports
```

Write site/compliance.json for the pages and print the same document.

```sh
flightdeck compliance build --config compliance.toml --reports reports --out site --build-run 'Build run #412' --json
```

Effects: additive, idempotent, closed-world.

## `flightdeck commands`

Explore flightdeck's commands

### Commands

- [`flightdeck commands list`](#flightdeck-commands-list): List a branch of flightdeck's commands with their arguments, examples and columns

## `flightdeck commands list`

List a branch of flightdeck's commands with their arguments, examples and columns

### Arguments

- `<PATH>`: The names of the commands below the root that lead to the branch, such as `books list`. Takes more than one value.
- `--limit <N>`: Print at most this many rows. Default: `25`.
- `--all`: Print every row

### Examples

List every command.

```sh
flightdeck commands list
```

Print commands list with its arguments, examples and columns as JSON.

```sh
flightdeck commands list commands list --json --full
```

### Columns

- `path`: The command line that runs the command, such as `shelf books list`.
- `about`: The command's one-line description, or its full description when it has no one-line description.

Effects: read-only, closed-world.

## `flightdeck skills`

Give coding agents flightdeck's skill

### Commands

- [`flightdeck skills install`](#flightdeck-skills-install): Install flightdeck's skills into each agent skills directory that exists
- [`flightdeck skills check`](#flightdeck-skills-check): Check that each installed copy of flightdeck's skills matches this flightdeck
- [`flightdeck skills print`](#flightdeck-skills-print): Print flightdeck's skill

## `flightdeck skills install`

Install flightdeck's skills into each agent skills directory that exists

### Arguments

- `--dir <DIR>`: Install into this skills directory in place of the agent skills directories
- `--force`: Replace a file or a link that install did not write with the skill's own folder

### Examples

Install flightdeck's skills into each agent skills directory that exists.

```sh
flightdeck skills install
```

Install flightdeck's skills, replacing a file or a link that install did not write.

```sh
flightdeck skills install --force
```

### Columns

- `state`: What install did: failed, skipped, written or unchanged.
- `skill`: The skill's name, which names its folder in each skills directory, such as `shelf`.
- `dirs`: The skills directories that share this copy, as named. Directories that resolve to one place through a symbolic link share one copy.
- `reason`: Why install skipped or failed, or what it replaced under --force, such as `linked to /src/shelf/skill`, or null.

### Columns that `--long` adds

- `path`: The SKILL.md file that install wrote or left, or the skill's other file whose write failed, at its real path.

Effects: additive, idempotent, closed-world.

## `flightdeck skills check`

Check that each installed copy of flightdeck's skills matches this flightdeck

### Arguments

- `--dir <DIR>`: Check this skills directory in place of the agent skills directories

### Examples

Report each agent skills directory where flightdeck's skills are missing or differ.

```sh
flightdeck skills check
```

### Columns

- `state`: Whether the copy is the binary's: missing, differs, unreadable, linked or current.
- `skill`: The skill's name, which names its folder in each skills directory, such as `shelf`.
- `dirs`: The skills directories that share this copy, as named. Directories that resolve to one place through a symbolic link share one copy.
- `reason`: Why the copy is missing a file, differs, cannot be read or is linked, such as `written by shelf 0.2.0`, `REFERENCE.md is missing` or `linked to /src/shelf/skill`, or null.

### Columns that `--long` adds

- `path`: The SKILL.md file that was checked, at its real path.

Effects: read-only, closed-world.

## `flightdeck skills print`

Print flightdeck's skill

### Examples

Print flightdeck's skill.

```sh
flightdeck skills print
```

Effects: read-only, closed-world.

## `flightdeck completion`

Set up flightdeck's completion in a shell

### Commands

- [`flightdeck completion install`](#flightdeck-completion-install): Install flightdeck's completion where the shell loads it
- [`flightdeck completion check`](#flightdeck-completion-check): Check that flightdeck's installed completion file matches this flightdeck
- [`flightdeck completion print`](#flightdeck-completion-print): Print flightdeck's completion registration script for a shell

## `flightdeck completion install`

Install flightdeck's completion where the shell loads it

### Arguments

- `--shell <SHELL>`: Install for this shell in place of the one that SHELL names. One of `bash`, `elvish`, `fish`, `powershell`, `zsh`.
- `--force`: Replace a file or a link that install did not write; a link's target stays as it is

### Examples

Install flightdeck's completion for fish.

```sh
flightdeck completion install --shell fish
```

Install it for zsh, replacing a file or a link that install did not write.

```sh
flightdeck completion install --shell zsh --force
```

### Columns

- `state`: What install did: failed, skipped, written or unchanged.
- `shell`: The shell: bash, elvish, fish, powershell or zsh.
- `path`: The completion file that install wrote or left. When the file is a link to a file that install wrote, the path of that file.
- `reason`: Why install skipped or failed, or what it replaced under --force, such as `linked to /path/to/target.fish`, or null.

Effects: additive, idempotent, closed-world.

## `flightdeck completion check`

Check that flightdeck's installed completion file matches this flightdeck

### Arguments

- `--shell <SHELL>`: Check the file of this shell in place of the one that SHELL names. One of `bash`, `elvish`, `fish`, `powershell`, `zsh`.

### Examples

Report zsh's completion file when it is missing or differs.

```sh
flightdeck completion check --shell zsh
```

### Columns

- `state`: Whether the file holds the script that install writes: missing, differs, unreadable, linked or current.
- `shell`: The shell: bash, elvish, fish, powershell or zsh.
- `path`: The completion file that was checked. When the file is a link to a file that install wrote, the path of that file.
- `reason`: Why the file differs, cannot be read or is linked, such as `linked to /path/to/target.fish`, or null.

Effects: read-only, closed-world.

## `flightdeck completion print`

Print flightdeck's completion registration script for a shell

### Arguments

- `--shell <SHELL>`: Print the script of this shell in place of the one that SHELL names. One of `bash`, `elvish`, `fish`, `powershell`, `zsh`.

### Examples

Print the registration script for fish.

```sh
flightdeck completion print --shell fish
```

Effects: read-only, closed-world.

## `flightdeck schemas`

Explore the JSON documents that flightdeck prints

### Commands

- [`flightdeck schemas list`](#flightdeck-schemas-list): List the JSON documents that flightdeck prints, with a description of each
- [`flightdeck schemas show`](#flightdeck-schemas-show): Print the JSON Schema of each document

## `flightdeck schemas list`

List the JSON documents that flightdeck prints, with a description of each

### Arguments

- `--limit <N>`: Print at most this many rows. Default: `25`.
- `--all`: Print every row

### Examples

List every document that flightdeck prints.

```sh
flightdeck schemas list
```

### Columns

- `id`: The document's id, such as `shelf/books-list/v1`, which `schemas show` takes.
- `kind`: output when a command prints the document with its schema id, unwrapped when a command prints it without the envelope that holds its schema id, and file when it is a file format that users write and no command prints.
- `form`: stream when a command prints the document as a stream, one document on each line as each row arrives, and document when a command prints it once or it is a file format.
- `description`: What the document holds, in one line.

### Columns that `--long` adds

- `commands`: The command lines that print the document. It is empty for the failure document, which every command prints when it fails with --json, and for a file format.

Effects: read-only, closed-world.

## `flightdeck schemas show`

Print the JSON Schema of each document

### Arguments

- `<ID>`: The ids of the documents that `schemas list` lists, separated by spaces. Required. Takes more than one value.

### Examples

Print the JSON Schema of the document that schemas list prints.

```sh
flightdeck schemas show flightdeck/schemas-list/v1
```

### Columns

- `id`: The document's id, such as `shelf/books-list/v1`.
- `schema`: The document's 2020-12 JSON Schema.

Effects: read-only, closed-world.

## `flightdeck self`

Review flightdeck itself

### Commands

- [`flightdeck self environment`](#flightdeck-self-environment): Explore the environment variables that flightdeck reads

## `flightdeck self environment`

Explore the environment variables that flightdeck reads

### Commands

- [`flightdeck self environment list`](#flightdeck-self-environment-list): List the variables that flightdeck and crumbkit read, with their rules and values

## `flightdeck self environment list`

List the variables that flightdeck and crumbkit read, with their rules and values

### Examples

List the variables the tool declares and their values.

```sh
flightdeck self environment list
```

Print each variable as JSON, where a withheld value has no text.

```sh
flightdeck self environment list --json
```

### Columns

- `name`: The variable's name.
- `type`: The type of the variable's value, such as port or integer 1..=100.
- `unset`: What happens when the variable has no value, in local mode and, when it differs, in production.
- `source`: Where the variable's value comes from, with the kind of its bind.
- `value`: The variable's value in this shell, or its default; a withheld value prints as eight dots and has no text in JSON.
- `owner`: The library that reads the variable in every tool, such as `crumbkit`; empty, and left out of `--json`, for a variable that the tool's own declaration holds.

### Columns that `--long` adds

- `show`: Whether the value may be shown or is withheld.
- `bind`: The variable's bind, such as a 1Password reference, when it has one.
- `cache`: `never` when the bind keeps the value out of any cache of resolved values; empty, and left out of `--json`, when a cache may keep it.
- `purpose`: The sentence that says what the variable is for.
- `effect`: The sentence that says what the application does when the variable is unset; empty, and left out of `--json`, when the spec states none.
- `values`: Each value of a named enumeration with its description; empty, and left out of `--json`, for any other type.
- `check`: The sentence that states the check the application's own code makes of the value, which starmap never runs; empty, and left out of `--json`, when the spec states none.
- `group`: The name of the group of variables that are set together or not at all, which the variable belongs to; empty, and left out of `--json`, when it belongs to none.
- `state`: Where the value comes from: `set` when this shell sets the variable, `default` when it leaves the variable unset and the default applies, `unset` when the variable has no value, and `invalid` when this shell sets a value that does not fit the type. The same state as `value.state`, at the top level.

Effects: read-only, closed-world.
