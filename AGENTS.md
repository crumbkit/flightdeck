# Agent notes

- Run `scripts/install-test-tools.sh` once before the tests. It installs
  shelf and reading under `target/test-tools/bin`, and tests that need them
  fail without them.
- Move the crumbkit commit in that script and the schema copies in
  `tests/fixtures/crumbkit/` together, and say so in the pull request.
- `SNAPSHOTS=overwrite cargo test` rewrites the golden files under
  `tests/golden/` and `docs/commands.md`. Review the diff before you commit.
- Only `src/spawn.rs` and its child modules start processes. Add a new
  kind of run there as its own operation, with the five items that PROC-2
  lists. A thread starts there too, or inside a `std::thread::scope` that
  joins it, as the server's threads do.
- Test names are sentences, and no test repeats what another proves.
- A deliberate shortcut gets a `RUST-10:` comment that says what it handles,
  what it misses and what replaces it.
- The page lives in `ui/`. Before you push a change there, run `pnpm run
  lint`, `typecheck`, `knip`, `test`, `build`, `size` and `acceptance` in
  `ui/`, and commit the rebuilt `ui/dist`.
- `ui/tests/fixtures/shelf-model.json` mirrors the server's `/api/model`
  reply for shelf. Regenerate it from the server when the reply changes.
- The page's tests run against a stub of the server's API in
  `ui/tests/acceptance/stub.ts`, until the server serves the page.
