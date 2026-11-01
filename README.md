# flightdeck

flightdeck is a program in development that will start a local web page
for any command-line tool built on
[crumbkit](https://github.com/crumbkit/crumbkit). The page will list the
tool's commands, draw a form for each one, run the command as a child
process and draw its JSON result.

flightdeck reads only the JSON documents that every crumbkit tool prints
about itself: `commands list`, `schemas list`, `schemas show`, the failure
document and the listing envelope. It links none of the tool's code.

## Status

The web page does not exist yet, and neither does `flightdeck open <TOOL>`,
the command that will start it.

What works today:

- `flightdeck commands list`, `flightdeck schemas list` and
  `flightdeck schemas show`, crumbkit's own commands, describe flightdeck
  itself. [docs/commands.md](docs/commands.md) is the reference.

## Install

flightdeck needs Rust 1.99 or later. Install it from GitHub:

```sh
cargo install --git https://github.com/crumbkit/flightdeck crumbkit-flightdeck
```

Or install it from a checkout of this repository:

```sh
cargo install --path .
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
[rust-standards.toml](rust-standards.toml) maps the Rust standards that
flightdeck follows to the files that meet them.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
