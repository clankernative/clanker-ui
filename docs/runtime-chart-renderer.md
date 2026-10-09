# Experimental request-time charts (ABI 1)

Current authorized query data → independently approved ECharts worker → checked
scene → ordinary Native SVG/table response. No Node service, browser chart
engine, request-time assembly compiler or build-time snapshot is involved.

`chart` remains `draft` / `adapter-required`. Ready-only discovery excludes it;
`clanker-ui capabilities` identifies its manifest and this guide. This feature
is additional to the fifty-four complete contracts, not a readiness promotion.

## Ownership and adoption

UI owns the closed chart input, ECharts 6.0.0 adaptation, templates/styles and
optional events. Platform owns authorization, queries, typed admission,
independent execution approval, confinement, supervision and HTTP/live updates.
The app owns data, sampling, domains, routes, accepted ranges, commands and dialogs.

1. Use matching UI/Platform source with generic presentation ABI 1. Older hosts
   do not admit `ui_scene`; no published compatibility is implied.
2. Restore locked dependencies during explicit setup, then build reviewed tools:

   ```sh
   cargo build --locked --offline --bin clanker-ui --bin clanker-chart-worker
   ./target/debug/clanker-chart-worker --contracts
   ./target/debug/clanker-chart-worker --licenses
   ```

   Worker contracts must match `components/chart/renderer-contract.json`.
   Review executable bytes, the embedded engine hash and upstream notices before
   execution approval. Native compilation never downloads dependencies.
3. Provision the package outside app `ui/` and explicitly author/update the
   canonical `ui/ui.lock.json`. Declare a checked query record:

   ```html
   <cui-chart id="sample-chart" data="{{ chart.chart }}"
              label="Current samples · values · UTC" enhance="false" />
   ```

   Scalars are integer `width,height,start,end,y_min,y_max` and string
   `title,kind`; `samples : CollectionPage(Sample)` holds integer
   `time,value`, boolean `missing` and string `key`. Expansion projects
   only checked scalars and `samples.items`, never cursors or unrelated fields.
4. Independently approve the build CLI via `DAY2_UI_PROVIDER_PIN_JSON` and
   runtime worker via `DAY2_PRESENTATION_PIN_JSON`. The latter is the JSON value,
   not a filename: schema version 1, ABI 1, canonical executable path, exact
   `sha256:` digest and matching `renderers` catalog. Native's
   `docs/RUNTIME-PRESENTATIONS.md` defines that generic boundary. Locks and
   declarations do not approve code.
5. Build/run normally. Assembly stages locked, non-browser
   `ui/presentation.json` metadata; startup privately captures the approved
   worker. Existing Native bundles/releases do **not** provision this worker.
   A local source override is not qualified cold provisioning.

Set `enhance="true"` for inspection/range controls and explicitly import the
staged `./clanker-ui.js` from the app module. Keep ordinary GET links/forms and
mutation drafts outside live display regions. App token overrides belong in
`ui/clanker-theme.css`; see the [component contract](../packages/vanilla/components/chart/README.md)
for event payloads, ownership and cleanup.

## Closed scope

- One `line`/`bar` UTC series; explicit integer domains and dimensions.
- Width 320–2048, height 120–1024; nonblank title ≤160 UTF-8 bytes.
- ≤128 chronological, unique-timestamp samples in `[start,end)`; unique keys
  ≤80 bytes. Native API collections retain their 100-item page bound.
- Safe integer values; measured values inside the y-domain. Missing is a gap,
  distinct from zero; its safe-integer placeholder is ignored as data.
- Fixed English/UTC axis formatting, no callbacks or arbitrary ECharts options.
- Fresh engine per job, ≤128 MiB heap, ≤2 seconds engine work. Host frames remain
  ≤1 MiB, exchange timeout 3 seconds, separate admission wait ≤2 seconds.
- Typed paths, labels, plot rectangle and points, not markup. SVG paint order,
  clipping, transforms, stroke metrics and text alignment/fonts are preserved;
  unsupported SVG features or malformed/nonfinite coordinates fail closed.
  Templates retain ordinary escaping/admission without a `safe` bypass.
- Reply `inputDigest` hashes the exact request bytes excluding the JSONL LF.
  `--contracts` and the locked schema are the wire-field authority.

## Deterministic simulation and conformance

`src/lib.rs` owns deterministic validation, protocol and scene adaptation.
It contains no clock, filesystem, environment lookup or randomized collection.
`Engine` is replaceable: simulated outcomes and actual ECharts use the same
input/output boundary. `engine.rs` owns QuickJS, bounded monotonic interruption
and fixed clock/random/locale behavior; `main.rs` owns framed process I/O.

Seeded simulations independently check success/rejection, invalid input,
timeout/engine failures, identities and coordinate failures, with seed/step
failure traces. Real subprocess tests replay recorded vectors, verify packaged
schema parity and compare host timezone/locale variants. These do not substitute
for Native confinement or browser acceptance. Component lifecycle schedules use
a DOM adapter; the app navigation adapter injects network and virtual timers.

```sh
cargo fmt --all --check
cargo test --locked --offline
cargo run --locked --offline -- verify --lock examples/button-app/ui.lock.json
node --test tests/components/*.test.mjs tests/browser/chart-interaction.test.mjs
```

Platform `examples/chart-live/VERIFICATION.md` separates current evidence from
remaining Mac/browser/full application DST/cold-restore/release gates.
