# Commands of `flightdeck`

This reference is generated from the help of `flightdeck`. Edit the help in the tool's code, and not this file.

## `flightdeck`

Start a local web page for a crumbkit tool

### Commands

- [`flightdeck open`](#flightdeck-open): Open a local web page that lists and runs a tool's commands
- [`flightdeck commands`](#flightdeck-commands): Explore flightdeck's commands
- [`flightdeck completion`](#flightdeck-completion): Set up flightdeck's completion in a shell
- [`flightdeck schemas`](#flightdeck-schemas): Explore the JSON documents that flightdeck prints

### Arguments

- `--json`: Print the result as a JSON document. The default prints text. Works after every subcommand.
- `--fields <FIELD>`: Print only these comma-separated fields, in this order. The default prints the compact columns. Takes more than one value. Works after every subcommand.
- `--long`: Add the long columns after the compact ones. The default prints only the compact columns. Works after every subcommand.
- `--full`: With --json, print every field of every row. The default prints only the fields of the columns. Works after every subcommand.

## `flightdeck open`

Open a local web page that lists and runs a tool's commands

### Arguments

- `<TOOL>`: The crumbkit tool, by a name on PATH or a path. Required.
- `--read-only`: Run only the tool's read-only commands from the page
- `--no-browser`: Print the path of the page to open instead of opening the browser

### Examples

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
- `reason`: Why install skipped or failed, or what it replaced under --force, such as `linked to /src/shelf/shelf.fish`, or null.

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
- `reason`: Why the file differs, cannot be read or is linked, such as `linked to /src/shelf/shelf.fish`, or null.

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
