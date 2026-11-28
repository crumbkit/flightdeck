# Agent notes

- Run `scripts/install-test-tools.sh` once before the tests. It installs
  shelf and reading under `target/test-tools/bin`, and tests that need them
  fail without them.
- A plain `cargo nextest run` leaves out the test binaries that need shelf
  and reading, because `default-filter` in `.config/nextest.toml` names
  them for the shared lane. Run `cargo nextest run --ignore-default-filter`
  to run every test.
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
  schemas`, `lint`, `typecheck`, `knip`, `test`, `build`, `size` and
  `acceptance` in `ui/`, and commit the rebuilt `ui/dist`.
- `ui/headless/src/api/generated/` is written by `pnpm run schemas` from
  the JSON Schemas under `tests/golden/schemas/`. After a change to those
  golden files, run it and commit the result; never edit the files by hand.
- `pnpm run acceptance` starts `target/debug/flightdeck` with the shelf
  under `target/test-tools/bin`, so build flightdeck and install the test
  tools first. It reads workflow graphs from the fake board in
  `tests/fixtures/board/fake`.
- `plugin/` holds the `flightdeck` Board plugin's manifest and the
  generated schema of `flightdeck/workflow-graph/v1`, which the binary
  embeds. `SNAPSHOTS=overwrite cargo test` rewrites the schema from the
  structs in `src/workflow_graph.rs`. `tests/fixtures/board/fake/make.py`
  rewrites the fake board's documents from the graphs beside it.
- The Workflows page's layout lives in `ui/src/lib/workflowLayout.ts`, a
  port of the signed mockup's script. A new built file under `ui/dist`
  needs its entry in `src/server/assets.rs`.
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
