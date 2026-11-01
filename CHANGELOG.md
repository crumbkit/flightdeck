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
  `flightdeck::server::failed` when it cannot start.

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
  start or prints too much. A run ends a second after its child exits even
  when a process in another session holds its output open. A shutdown
  lets runs end within a grace, cancels the rest, and refuses new runs with
  `flightdeck::run::closed`.
