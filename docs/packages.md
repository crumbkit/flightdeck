# The compliance packages

Flightdeck's compliance pages are also two npm packages, so another front end
can render them in its own look:

- `@crumbkit/flightdeck-headless` parses a `flightdeck/compliance/v1`
  document, the `compliance.json` that `flightdeck compliance build --out`
  writes, and returns each page's headlines, tiles, rows, counts and words.
  It has no DOM, network or styling, and reads the document only through a
  `ComplianceSource` that the app passes in.
- `@crumbkit/flightdeck-react` holds the React hooks over it, such as
  `useComplianceDocument` and `useCompliance`. They keep each page's filters
  in the page address and return prop getters, and render nothing.

Both ship compiled JavaScript and declarations, depend on `@crumbkit/ui` by
exact version, and come from npmjs. The React package needs React 19.

## Install

```sh
pnpm add @crumbkit/flightdeck-headless@<version> @crumbkit/flightdeck-react@<version>
```

`<version>` is a Flightdeck release, such as `0.6.0`; both packages carry the
release's version. The install needs no build step, no override and no change
to pnpm's settings. A project that sets `minimumReleaseAge` installs a
release once it is that old.

The app supplies the source and the address:

```ts
import { readDocument, type ComplianceSource } from "@crumbkit/flightdeck-headless/source";
import { useCompliance } from "@crumbkit/flightdeck-react/useCompliance";

const source: ComplianceSource = {
  read: async () => (await fetch("/compliance.json")).json(),
};
```

`scripts/check-packages/consumer.ts` is a whole example: it reads the sample
document, renders the Compliance page's headline and counts under
`react-dom/server`, and is what the release checks.

## Verify a download

Each GitHub release attaches the two tarballs beside the binary archives,
lists them in `SHA256SUMS`, and carries a build provenance attestation for
every file. To check that a file was built by Flightdeck's release workflow
from this repository:

```sh
gh attestation verify crumbkit-flightdeck-headless-<version>.tgz \
  --repo crumbkit/flightdeck \
  --signer-workflow crumbkit/flightdeck/.github/workflows/release.yml
```

The same command checks a binary archive. npmjs shows the provenance of each
version it got from the workflow, and `npm audit signatures` checks it.

## How a release builds them

The committed manifests in `ui/headless` and `ui/react` stay at `0.0.0`,
point their exports at `src/`, and name `@crumbkit/ui` by its git tag, so the
page and the tests use the source with no build. The `packages` job of
`.github/workflows/release.yml` runs:

1. `scripts/pack-packages/pack.sh <version> <dir>`, which builds each
   package's `dist/` with `tsc -p tsconfig.build.json`, sets both versions,
   names `@crumbkit/ui` by the exact version its tag's `package.json` holds,
   and packs. For a release it stops unless npmjs holds that version with
   the same `dist/` as the tag.
2. `scripts/check-packages/check.sh <version> <dir>`, which installs both
   tarballs into a new project with pnpm's default settings, from a local
   registry that also serves the tag's `@crumbkit/ui`, checks that one copy
   of `@crumbkit/ui` is installed, type-checks `consumer.ts` and compares
   its render with `expected.html`.

The `release` job then attests every file, publishes the core and then the
hooks to npmjs with npm trusted publishing, which uses no stored token, and
attaches the files to the release. A run again skips a version npmjs already
holds.

Both scripts run locally after `pnpm install --frozen-lockfile` in `ui/`.
