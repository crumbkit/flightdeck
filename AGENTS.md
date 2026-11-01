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
- `pnpm run acceptance` starts `target/debug/flightdeck` with the shelf
  under `target/test-tools/bin`, so build flightdeck and install the test
  tools first.
- `ui/tests/fixtures/shelf-model.json` is the server's `/api/model` reply
  for shelf, and an acceptance test fails when they differ. Rewrite it with
  `FIXTURES=overwrite pnpm run acceptance`, then `pnpm run fix`.
- A journey picks its tool with `test.use({ tool })`: shelf by default,
  reading, or "slow reading", whose `entries stream` prints each line only
  when the test calls `served.release()`, so a test can see rows arrive
  and cancel a run while it is still going.
- Every journey runs against the real server. A test of a tool without
  goal headings, such as one built on crumbkit 0.1.0, reads a model that
  has one heading named after the tool, as `shelfWithoutGoals` in
  `ui/tests/fixtures/shelf.ts` does.
