# A shelf that indents its JSON

`shelf` here is a shell script that stands in for a crumbkit tool that sets
`JsonLayout::Pretty`. It runs the shelf that `scripts/install-test-tools.sh`
installs and lays out each line that starts with `{` as
`serde_json::to_writer_pretty` writes it, with each field and item on a line
of its own, indented by two spaces for each level. Every other line, such as
`--version`, passes through, and the script exits with shelf's exit code. It
needs `perl`.

shelf has no stream command, so the script indents every document that shelf
prints. A crumbkit tool that sets `JsonLayout::Pretty` still prints a
stream's items one to a line.

`tests/runner.rs`, `tests/model.rs` and the page's journey "pretty shelf"
run it.
