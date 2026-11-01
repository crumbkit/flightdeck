# Agent notes

- Run `scripts/install-test-tools.sh` once before the tests. It installs
  shelf and reading under `target/test-tools/bin`, and tests that need them
  fail without them.
- Move the crumbkit commit in that script and the schema copies in
  `tests/fixtures/crumbkit/` together, and say so in the pull request.
- `SNAPSHOTS=overwrite cargo test` rewrites the golden files under
  `tests/golden/` and `docs/commands.md`. Review the diff before you commit.
- Only `src/spawn.rs` starts processes. Add a new kind of run there as its
  own operation, with the five items its module documentation lists.
- Test names are sentences, and no test repeats what another proves.
- A deliberate shortcut gets a `RUST-10:` comment that says what it handles,
  what it misses and what replaces it.
