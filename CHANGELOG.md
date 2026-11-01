# Changelog

This file records the notable changes to the published `crumbkit-flightdeck`
crate. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `flightdeck commands list`, `flightdeck schemas list` and
  `flightdeck schemas show`, crumbkit's own commands, describe flightdeck.
- The `model` module loads a crumbkit tool's description of itself into a
  model of its commands, arguments, goal headings, documents and inferred
  links, and fails with `flightdeck::tool::not_found` or
  `flightdeck::tool::load` when it cannot.
