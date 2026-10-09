# Clanker Native UI

**Status:** early access: CLI 0.1.0 / vanilla 0.7.0, with fifty-four component-complete contracts, forty-five Native-supported within the documented scope and nine adapter-required. Consult the [exact-version release page](https://github.com/clankernative/clanker-ui/releases/tag/v0.1.0) for available assets, qualified targets and reviewed hashes/provenance; source documentation alone does not establish hosted availability. First-install guidance: [`docs/INSTALL.md`](docs/INSTALL.md). The old per-instance `compose` output is deprecated. Local package paths are not yet a portable CI dependency; this is not production-ready. The standalone local gallery has passed separate Native host admission and desktop/mobile browser checks for all fourteen layouts; CLI fixture verification alone does not prove host integration.

Clanker Native UI is an optional, agent-first component discovery and assembly system. It helps an app-building agent find a component that fits a user's intent, read its contract, compose it for the app's target, and verify the result. The vanilla package provides Button, Icon, Badge, Divider, Status Indicator, Tag, Alert, Progress, Form Field, Avatar, Empty State, Metric, Skeleton, Page Header, Card, Cluster, Container, Grid, Split, Stack, Cover, Layer, Pane, Reel, Sidebar, and Switch with target-specific HTML and CSS. Native controls integrate with the host's existing Datastar form and patch transport. Four ready components have bounded browser-local enhancements for literal static pages; they do not own Datastar signals or app transport. Other authors could publish their own packages and target adapters, including React catalogs, without making React a dependency of Clanker Native.

Clanker Native remains usable without Clanker Native UI. Apps keep ownership of their domain models, queries, commands, routes, presentation, and design flavor. Clanker Native UI describes and supplies UI components; it does not become a second app-operation catalog or an arbitrary callback SDK.

## Experimental request-time charts

An opt-in `<cui-chart>` line/bar contract and separate ECharts 6.0.0 worker
transform current authorized query data into checked SVG and exact values.
Optional inspection/range events leave domain state and requests with the app.

This **experimental v1** contract remains `draft` / `adapter-required`,
additional to the fifty-four complete contracts and excluded from ready
discovery. `capabilities` identifies its manifest. Native presentation ABI 1
and independent worker approval are required; existing UI releases do not
provision that worker. See [setup, bounds and test boundaries](docs/runtime-chart-renderer.md).

## Try the local proof

The Rust workspace has a pure `catalog-core` crate, a `clanker-ui` discovery/assembly CLI, and private `clanker-ui-runtime` preview guards. Native does not link that crate. Clanker UI owns expansion; the Native build invokes the pinned CLI and independently admits its output in a private snapshot. The forty-five Native-supported vanilla components are `button`, `icon`, `badge`, `divider`, `status-indicator`, `tag`, `alert`, `progress`, `form-field`, `avatar`, `empty-state`, `metric`, `skeleton`, `page-header`, `card`, `cluster`, `container`, `grid`, `split`, `stack`, `cover`, `layer`, `pane`, `reel`, `sidebar`, `switch`, `select-field`, `filter-bar`, `data-table`, `breadcrumbs`, `pagination`, `activity-feed`, `button-group`, `definition-list`, `disclosure`, `progress-steps`, `segmented-control`, `tabs`, `checkbox-group`, `radio-group`, `toggle`, `copy-field`, `theme-switcher`, `tooltip`, and `toast`. A component directory without `component.json` is ignored; malformed declared metadata fails. The lock at `examples/button-app/ui.lock.json` pins declared package inputs, not every file in the checkout. The app owns `examples/button-app/theme.css`; it does not edit package defaults.

From this directory:

```sh
cargo run --locked --offline -- find button --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- describe button --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- properties --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- graph button --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- verify --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- find icon --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- find-icon processor --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- describe icon --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- list --lock examples/button-app/ui.lock.json
cargo run --locked --offline -- graph badge --lock examples/button-app/ui.lock.json
```

`lock --lock <file> --package <path>` and `native-lock` author the same canonical Native JSON lock: `schemaVersion: 1`, `provider: "clanker-ui.native"`, and `package: {name, version, path, digest, inputs}`. All discovery, validation, expansion, and render commands consume this schema; legacy flat and Markdown locks are rejected. The package path is relative to the lock directory, with at most two leading parent segments. Inputs cover the complete declared package closure, including draft contracts and assets, sorted by path with byte counts and per-file SHA-256 digests. The package digest hashes each `path + NUL + decimal bytes + NUL + digest + LF` manifest record. Changing declared bytes requires explicit `--update` for either authoring command. The CLI emits versioned JSON and nonzero status on failure. Its `verify` checks manifest shape, declared inputs, token references, and self-authored fixtures; it is not a substitute for Native's independent admission. Word matching is literal, not embedding-based search. The old `build` and `compose` commands still produce isolated generated files for the earlier proof, but **do not use them to integrate an app**.

### Build-time expansion and script-free scenes

```sh
cargo run --locked --offline -- expand --lock examples/button-app/ui.lock.json --ui /path/to/app/ui
cargo run --locked --offline -- render --lock examples/button-app/ui.lock.json --ui /path/to/app/ui --scene /tmp/scene.json --fragment components/example.html
```

Standalone `expand` returns symbolic templates, typed binding metadata, a resource manifest, locked input digests, consumed source paths, and selected module entrypoints for agent diagnostics. Native `assemble` is protocol 2 and returns only ordinary templates/resources plus locked inputs; any needed `ui/ui-package.js` bootstrap is a generated module resource. Binding ABI remains 2. The producer-owned protocol fixture, default CLI golden test, and separately approved real-host smoke are documented in [`crates/clanker-ui/tests/fixtures/platform-assembly/README.md`](crates/clanker-ui/tests/fixtures/platform-assembly/README.md). Optional `expand --out <directory>` writes a separate proof bundle, never the app source. Component expansion, composition, binding syntax, resource selection, and CSS mappings live here; Native independently implements generic value and HTML safety capabilities.

`render` expands the same locked package and evaluates a bounded fake scene: `{"page":"pages/example.html","data":{"title":"Sample"},"routes":{"index":"/"},"width":1280}`. Without `--fragment`, `page` selects the template. Includes stay inside the captured UI. Text is escaped, numeric/image guards run, and fonts are embedded from locked bytes. The result is an inert, script-free HTML document: forms cannot submit and app modules do not run. Fake route maps are illustrative links, not Roc route checking. Studio must set the iframe viewport to the returned width; this command proves static presentation only.

`expand` is a read-only local proof. It emits ABI-2 expressions and uses the same canonical Native locked-input manifest digest as the other commands; this does not establish host admission. App integration uses `assemble`, not `expand`.

Local Platform execution uses the provider-neutral `DAY2_UI_PROVIDER_PIN_JSON` operator override. Verified installed bytes do not grant execution approval, and no hosted release is published by these setup commands. Never substitute a placeholder checksum or treat an app lock as executable authority.

### Provider-neutral Native assembly

Assembly protocol 2 uses MiniJinja 2.12.0 and binding ABI 2. Native captures private package/UI inputs, invokes `assemble --request FILE`, and independently admits the resulting bundle. The provider does not run while serving pages. ABI-1 bundles and component-specific `cui_*` callbacks are not accepted by this protocol.

`native-bundle` prepares a relocatable directory containing the exact current executable, the complete declared package input closure, mandatory `legal/LICENSE` and `legal/NOTICES.txt`, `manifest.json`, and a relative `provider-pin.json`. Legal bytes are embedded from reviewed source, independently hash/size-identified in a closed manifest shape and verified on restore; they do not alter vanilla 0.7.0 catalog inputs or app locks. Project code is MIT; dependency/font terms remain separate. See [`docs/RELEASE-NOTICES.md`](docs/RELEASE-NOTICES.md). App authors redistributing generated package CSS/JS or catalog assets must retain project MIT and applicable third-party notices (including icon terms and font OFL), without changing app-owned licenses or domain rights. It does not include generated app HTML or CSS. Prepare from a clean checkout of the exact source revision and use a fresh output path; offline mode requires the pinned Rust dependencies to be cached. The revision argument is an unsigned provenance claim, not a check that the executable was built from that commit:

```sh
cargo build --locked --offline --release --bin clanker-ui
./target/release/clanker-ui native-bundle --package packages/vanilla \\
  --output /tmp/clanker-ui-native-bundle --source-revision "$(git rev-parse HEAD)"
./target/release/clanker-ui verify-native-bundle --bundle /tmp/clanker-ui-native-bundle
```

Verification is local and never invokes the bundled executable. The unsigned manifest and pin identify bytes; they do not authenticate a producer or grant trust. An operator must approve the source revision, executable, and package tree **separately from installation and before compiler execution**. The 64 MiB executable guard remains unchanged; dev/test profiles omit debug information so default CLI tests fit the same guard.

#### Versioned release assets and explicit install/restore

`native-release` creates exactly seven files in a fresh local directory: `clanker-ui-native-<tool-version>-<target>.tar.gz`, standalone `clanker-ui-<tool-version>-<target>`, adjacent `clanker-ui-<tool-version>-<target>.NOTICES.txt`, their three `.sha256` checksums, and target-specific `clanker-ui-release-<tool-version>-<target>.json`. There is no `release.json` alias; Linux/macOS metadata can coexist on one exact-version release. The archive captures only bundle files, preserves executable permission, and normalizes ordering, timestamp, uid/gid, and modes. It validates its captured archive through the same safe extractor, then takes bootstrap and notices from that **exact validated snapshot**, never a new read of the original bundle. Bootstrap hash/bytes match the archive manifest and installed CLI; notices include project MIT and separate third-party terms and must accompany standalone redistribution. All seven files publish atomically/no-clobber; standalone mode is 0755 and other asset modes are 0644. Determinism applies to an identical bundle, not necessarily to independent compiler builds. This command **never publishes**. Existing CI uploads Linux review artifacts, not releases. Candidate target names are exactly `macos-aarch64` (primary Native qualification target, requiring separate native build/test and consumer proof) and `linux-x86_64` (CLI only, subject to producer tests; not a full Linux Native builder claim). Windows is unsupported. No isolated-builder port or new CI vendor is selected.

```sh
./target/release/clanker-ui native-release --bundle /tmp/clanker-ui-native-bundle --output /tmp/clanker-ui-release
# Existing directory bundles remain usable offline:
./target/release/clanker-ui install-native-bundle --bundle /tmp/clanker-ui-native-bundle --output /operator/providers/local-reviewed
# EXPECTED_SHA256 must be an explicitly reviewed sha256:<64 lowercase hex digits> value:
./target/release/clanker-ui install-native-bundle --archive /reviewed/clanker-ui-native-0.1.0-linux-x86_64.tar.gz --expected-sha256 "$EXPECTED_SHA256" --output /operator/providers/0.1.0
# Hosted acquisition is an explicit operator action, never an app build/server action:
./target/release/clanker-ui restore-native-bundle --github-repository clankernative/clanker-ui --version 0.1.0 --expected-sha256 "$EXPECTED_SHA256" --output /operator/providers/restored-0.1.0
```

The GitHub release-asset URL convention is an initial, replaceable acquisition adapter, not a registry. It requires an explicit repository and exact tool version (`v<version>` tag); there is no `latest`, arbitrary URL, downloaded install script, or automatic app networking. Hosted setup requires operator-installed `curl`; its configuration file is disabled, HTTPS redirects are bounded/HTTPS-only, and transfer size and time are bounded. Local directory/archive installation needs no network. Consult the exact release page for actually available/qualified assets; these producer commands do not establish hosted availability. Do not fetch a checksum alongside an untrusted archive and treat that as producer authentication: the expected SHA must come from the operator's reviewed identity decision. Hashes alone do not authenticate provenance; revision and manifest identity remain unsigned claims.

Archive installation checks the explicit expected SHA **before decompression/extraction**, then rejects non-regular entries (including symlinks, hardlinks, special files and path extensions), absolute/traversal/nonportable paths, duplicate/file-directory/case-folded prefix collisions, and excessive path depth, file count, file bytes, total bytes, or expanded bytes. Output modes are normalized, not trusted from the archive. The output parent must already exist. It verifies the resulting complete bundle without launching the compiler, and atomically publishes only to a fresh destination. Failed installs remove private staging; existing outputs, including dangling symlinks, are never replaced. `restore-native-bundle` is an alias for this same verified installer into a **new** destination, not an in-place upgrade, host activation, or rollback mutation. Bundles and pins remain relative/relocatable. No app lock or host configuration is rewritten, and the explicit local pin override remains available.

Agent/operator guidance: keep acquisition at setup/restore boundaries; never grant arbitrary app script authority or conflate checksum validation with compiler execution approval. Keep reviewed installs outside app source, select/activate them through separately approved host configuration, and retain old versions until the operator chooses to remove them. For an app such as GoLinks, keep `ui/ui.lock.json` explicit and ensure its package path resolves to the host's private builder-staged `../../packages/clanker-vanilla`; do not add a sibling-worktree dependency. This producer verification is not a full Platform/Native host gate. The app lock and Native's private assembly staging remain distinct from this producer bundle.

Generate an app-side lock for one explicitly selected local package, then generate a separate local executable pin:

```sh
cargo run --locked --offline -- native-lock --lock /path/to/app/ui/ui.lock.json --package ../../package
cargo run --locked --offline -- native-pin --output /path/to/local/provider-pin.json
```

`native-lock` hashes the complete declared package input closure, including contracts and assets. It writes only the named lock file; refreshing a lock remains an explicit operator action. `native-pin` writes an unsigned pin for the current CLI executable, with Native's 64 MiB size limit and a SHA-256 digest. It does not install, download, select, or configure an executable, and refuses to overwrite an existing file or a UI lock. Operators must review and verify executable provenance and digest before configuring `DAY2_UI_PROVIDER_PIN_JSON`. This pin is separate from the app lock and is not a portable release signature.

Golden tests cover the gallery, GoLinks, Studio's isolated gallery, and seven additional static contracts: all 38 historical supported roots. Expected symbolic expressions were explicitly rebased for ABI 2; this is no longer byte-for-byte compatibility with the old host callback ABI. The parity corpus uses a frozen package independent of current defaults. The seven static ports have separate real Native and browser evidence below; golden parity alone is not host-integration evidence for the nine adapter-required contracts.

### Component completeness and Native support

`status: "ready"` describes the component, not a backend or host adapter. A complete contract has real rendering and interaction, accessibility, cleanup, no-JS behavior, and conformance tests against its declared ports. Simulated adapters must be explicitly labeled and exercise failure, cancellation, recovery, and replacement; they are not production upload or query implementations.

`integration.native.status` separately records `supported` or `adapter-required`, with the exact proven scope or missing seam. An omitted integration record makes no new support claim. Native's actual declaration, resource, form, and route admission remains authoritative. Never weaken those checks to make a component appear integrated.

App-facing ports are discoverable under `integration.ports`; `typeSource` and `export` point to a component-owned TypeScript declaration listed in `assets.contracts`. These declarations are locked inputs, not executable adapters or server transports. The app owns file bytes, commands, data queries, routes, authorization, validation, persistence, and authoritative states. Optional adapter absence preserves the documented native fallback; a supplied adapter must obey cancellation and lifetime boundaries.

### Contract-driven Explorer metadata

`catalog-core::properties` describes editable fields, typed enum choices/defaults, numeric bounds, conditional fields, and admitted child slots for all 54 component-complete contracts. Its validators use the existing Rust contracts. Tests compare descriptor field names with serialized contract fields; `describe` includes the selected descriptor, and `properties` exports the full catalog with typed sample values.

The package declares `property-catalog.json` as locked data. `verify` rejects it if it differs from the Rust export. After changing descriptors, export the command's `data` object to that file, then explicitly refresh consumer locks. The Native host reads only the declared, digest-checked resource and stages `clanker-properties.js` in its private snapshot. It does not fetch package source at runtime or render HTML from the metadata.

The sibling gallery uses one shared property form and app-owned DOM bindings on already-admitted specimens. Browser drafts are previews, not host admission: the Native build remains authoritative for declarations, routes, resources, and accessible states. Images still require host-owned provenance; child controls choose admitted samples rather than accepting HTML. Local gallery Node tests live in `tests/gallery/` because Native admits only Roc outside the app's `ui/` tree; these tests require the sibling gallery checkout.

A Native app pins the package in `ui/ui.lock.json` and declares a button directly in its template, inside an existing app-owned command form when submitting:

```html
<cui-button kind="submit" variant="primary" icon="plus" label="Create" />
```

The Native build checks the generic input manifest and operator-pinned executable, invokes `assemble`, stages the returned templates and merged package/app CSS, then applies normal template, form, and resource admission. Local dev invokes the same build; neither the package nor the CLI runs at request time. The app contains no generated component HTML or CSS. The existing gallery, Studio, and GoLinks proofs use the older integration. They have not been migrated to this protocol; their locks and machine-local pins remain untouched. Their historical browser evidence does not prove ABI-2 integration. A pinned package checkout and executable must be provisioned explicitly before CI can adopt it.

Button attributes are `kind` (`action`, `submit`, `link`), `label`, `variant` (`primary`, `secondary` [default], `danger`, `quiet`), `size` (`compact`, `standard`), optional `icon`, `edge-aligned`, `disabled`, `busy`, and `busy-label`. A link uses a safe `href` or app-owned `route`. Labels may be app-authored literals or complete typed page-field interpolations. The Clanker adapter accepts whole checked Integer/Unsigned fields for `variant` (0 primary, 1 secondary, 2 quiet, 3 danger) and `size` (0 compact, 1 standard); generic host integer checks reject other values, while provider-authored template branches select literal CSS classes. These ordinal bindings belong to the Native target adapter, not the portable JSON contract or an arbitrary enum expression. Dynamic states select interactive or noninteractive markup from checked Bool fields; busy requires a replacement label. Disabled or busy links have no href. The standalone `<cui-icon name="search" size="medium" label="Search" />` contract uses the same closed 100-glyph catalog. `icon-catalog.json` supplies validated icon names, labels, and categories; `find-icon` searches all three case-insensitively and `describe icon` returns the metadata without changing the component contract. Icons are decorative/hidden from assistive technology by default; standalone meaningful icons require a nonblank `label` and render as `role="img"`. Icon fragment HTML and CSS are package assets; the target adapter supplies SVG attributes and geometry through the exact `[[attributes]]` and `[[geometry]]` slots. The button does not create a command, form fields, or route. The additional browser-local contracts are component-complete but require separate Native adapters; multi-package assembly and production package distribution remain future work.

The static declarations accept literal text or whole checked page-field bindings for their text values:

```html
<cui-badge tone="neutral" label="Active links" show-icon="false" />
<cui-divider orientation="horizontal" label="Details" alignment="start" />
<cui-status-indicator tone="neutral" label="No links match your filters." />
```

Badge requires visible `label` text, accepts tones `neutral`, `info`, `success`, `warning`, `danger`, and `running`, and can suppress or override its decorative icon with `show-icon` or a closed-catalog `icon` name. Divider defaults to a horizontal, unlabelled separator; it accepts a `start`, `center`, or `end` label alignment and rejects labels on vertical separators. Status Indicator requires visible `label` text, accepts tones `neutral`, `info`, `success`, `warning`, and `danger`, plus optional `detail`, `size` (`small` or `large`), and `pulse`; it is static, not an ARIA live region. Each component is noninteractive, checks unknown attributes, and uses app-overridable `--cui-*` tokens. A real GoLinks build in `../golinks-clanker-ui-button` exercises all three. This historical local proof used an empty sample list, not production data.

## Continue on a fresh machine

The development source is in Git; temporary bundles, build caches, and private
session cookies are not handoff inputs. Review branches are not a substitute for
exact-version reviewed release assets.
The Native development/isolated-build host is currently Apple Silicon macOS.
Install Xcode Command Line Tools and rustup, and have read access to the private
GoLinks repository.

From a new workspace directory:

```console
git clone --branch feat/native-ui-assembly-contract https://github.com/clankernative/clanker-ui.git clanker-ui
git clone --branch feat/provider-neutral-ui https://github.com/clankernative/platform.git platform
git clone --branch feat/clanker-ui-first-consumer https://git.wonderly.info/internal-tools/golinks.git golinks
(cd platform && cargo fetch --locked && cargo run --locked -p xtask -- bootstrap)
(cd clanker-ui && cargo +1.98.1 build --locked --release --bin clanker-ui)
mkdir -p packages .cache
test ! -e packages/clanker-vanilla && cp -R clanker-ui/packages/vanilla packages/clanker-vanilla
./clanker-ui/target/release/clanker-ui native-pin --output "$PWD/.cache/ui-provider-pin.json"
(cd platform && DAY2_UI_PROVIDER_PIN_JSON="$PWD/../.cache/ui-provider-pin.json" cargo run --locked -p xtask -- build "$PWD/../golinks")
```

The copy provisions the source package at the lock's conventional build-input
path; it is not generated HTML/CSS and does not modify GoLinks' lock. The builder
checks its declared bytes against that lock. The pin is machine-local approval
of the executable you just built, not permission supplied by the app. Record the
three checkout SHAs before comparing results. Fetch dependencies during setup;
`--offline` is appropriate only after the dependency cache is populated.

This exercises the direct Native build, **not** the full isolated control-plane
recipe. For an isolated builder, its private operator configuration additionally
sets `ui_assembly.provider_pin`, `ui_assembly.package_root`, and
`ui_assembly.package_key` (`clanker-vanilla`); see Platform's
`docs/CONTROL-PLANE.md`. Full local development also has the OpenTofu/Temporal
prerequisites documented in Platform's README.

No registry resolver or published package/tool release is assumed here. This is
a reproducible source-development setup, not the final Tailwind-like installation
experience. Do not copy old absolute pins or private instance state to a new
machine; regenerate approvals and create local test state there.

## GoLinks consumer proof

The actual GoLinks app's create/edit forms adopt Button and Form Field with an
app-owned theme and explicit `ui/ui.lock.json`. Disposable local checks cover
creation, editing, duplicate rejection with retained drafts, corrected retry,
escaping, keyboard focus, and desktop/mobile rendering. The native HTTP form
test is `tests/consumers/golinks-native-forms.test.mjs`; it requires an explicit
loopback origin and private development-session cookie file, and skips otherwise.
It does not execute browser JavaScript. Screenshots in
`docs/screenshots/golinks-first-consumer-*.png` show synthetic local data, not a
production deployment. These checks do not establish full isolated-build CI
qualification or published distribution.

## Form Field, Tag, Alert, and Progress

The `0.3.0` package adds four Native controls. The local GoLinks proof has a **UI Demo** navigation entry at `/ui-demo`, covering all twenty components, long copy, field errors and readonly values, tag counts and links, alert recovery, and measured/unknown progress. Sample task and error states are labeled illustrative; its create form uses the real app-owned command in the local instance. The dashboard and detail forms also use Form Field, including checked `link.url` and `link.description` value bindings. No generated component markup is pasted into the app. The historical `0.3.0` browser proof used a disposable local instance with two test links, not production data.

```html
<cui-form-field id="edit-url" name="url" label="Destination URL" input-type="url" value="{{ link.url }}" required="true" />
<cui-form-field id="edit-description" name="description" label="Description" kind="textarea" value="{{ link.description }}" />
<cui-tag label="Engineering" tone="brand" count="12" count-label="12 links" />
<cui-alert tone="warning" title="Review your links" body="Some destinations may be out of date." recovery-label="Open dashboard" recovery-route="links" />
<cui-progress label="Importing links" state="determinate" value="42" maximum="100" suffix="42 of 100 links" />
```

- **Form Field:** required stable `id`, `name`, and visible `label`; native input types `text`, `email`, `url`, `number`, `password`, `search`, `date`, and `tel`, or `kind="textarea"`. Optional `hint`, `error`, `placeholder`, `autocomplete`, `required`, `readonly`, `disabled`, bounded `maxlength`, and textarea `rows`. Hint/error IDs derive from the field ID; an error sets `aria-invalid`. Disabled inputs/textareas are supported outside command forms. The host still rejects disabled named command controls because browsers omit them from submission; use readonly when the command must receive the value. Hidden controls remain app-owned. The host still checks the control against the command's input carrier; a supported visual input type is not permission to bind it to every command field. Do not preload a password with a stored secret.
- **Tag:** tones `neutral`, `brand`, `info`, `success`, `warning`, `danger`; sizes `small`, `medium`, `large`; optional closed `icon`. `count` requires `count-label`. Use an admitted `href` or app-owned `route` for navigation. An otherwise static tag can have `remove-href`/`remove-route` paired with `remove-label`; this is navigation, not client-side dismissal. Nested links are rejected.
- **Alert:** required `title`, `body`, and `tone` (`info`, `success`, `warning`, `danger`). Optional `appearance` is `soft` (default), `outlined`, or `accent`; `heading-level` is `h2` (default), `h3`, or `h4`. A recovery destination (`recovery-href` or `recovery-route`) requires `recovery-label`. Announcement defaults to none; explicit `announcement="polite"` or `"assertive"` opts into live semantics. No dismissal or command lifecycle is claimed.
- **Progress:** required task `label` and explicit `state`. Determinate progress requires numeric literals or whole checked Integer/Unsigned page fields for `value` and `maximum`, with `maximum > 0` and `0 <= value <= maximum`; invalid values fail rather than clamp. Indeterminate progress omits both numbers and any `suffix`. Optional `detail`, semantic `tone`, and `size="regular"` or `"compact"`. Bound values pass the host's runtime range guard, preserving unsigned integer precision and checking completion against the original values.

## Avatar, Empty State, Metric, Skeleton, and Page Header

The `0.4.0` package adds five dependency-light presentation components. They extend the same GoLinks `/ui-demo`, not a new visual system. Page Header renders the page's only `h1`; Avatar, Empty State, Metric, and Skeleton have dedicated review sections. Real list data also exercises checked Metric label/value bindings and an Empty State when no results match. The sample metric totals, trends, people, and loading layouts are explicitly illustrative. Historical desktop/mobile checks covered these additions and all fourteen components using three disposable local test links.

```html
<cui-avatar initials="AL" label="Avery Lane" size="small" tone="brand" />
<cui-empty-state title="No results" body="Try another search." heading-level="h3" action-label="Open directory" action-route="links" />
<cui-metric label="Failed requests" value="2" trend="down" trend-tone="positive" trend-label="3 fewer than yesterday" />
<cui-skeleton shape="text" width="medium" animated="false" />
<cui-page-header title="Go Links" description="Find and manage your team's destinations." />
```

- **Avatar:** required `initials` (one to three Unicode grapheme clusters) and meaningful `label`, each literal or a whole checked page field. Sizes `extra-small`, `small`, `medium`, `large`, `extra-large`; tones `neutral`, `brand`, `success`. Native declarations accept optional `src` (a credential-free HTTPS literal or checked String field) or literal `image-asset` (an admitted app asset key), never both. The portable JSON contract calls this `imageSource`. Runtime guards validate bound initials and URLs. The browser loads the photo with no referrer; the server neither fetches nor proxies it. Only actually rendered HTTPS origins enter CSP, up to 32 per response; remote photo bytes are not package-pinned. CSP is fixed for the document: a new origin in a live patch preserves initials and shows an explicit refresh notice without discarding drafts automatically. Initials remain beneath the image as a no-JS fallback.
- **Empty State:** required `title`/`body`; `alignment="start"` or `"center"`; `heading-level="h2"`, `"h3"`, or `"h4"`. Optional `action-label` pairs with an admitted `action-href` or app-owned `action-route`. This is ordinary navigation, not a command or dismissal. Choose the heading level for the surrounding document.
- **Metric:** required `label` and already formatted `value`; optional `detail`, closed-catalog `icon`, and `appearance="plain"` or `"contained"`. `trend`, `trend-tone`, and `trend-label` must appear together. Direction (`up`, `down`, `flat`) is independent of favorability (`positive`, `negative`, `neutral`). Optional `trend-announcement` replaces the screen-reader description, not an ARIA live policy. The component does not calculate, parse, poll, or infer business meaning.
- **Skeleton:** required literal `shape="text"`, `"rectangle"`, or `"circle"`; optional `size="small"`, `"medium"`, or `"large"`, width presets `short`, `medium`, `full`, and `animated` (default true). Always `aria-hidden`; the app owns meaningful loading text and the parent's `aria-busy`. Reduced motion disables its animation. No arbitrary dimensions or data expressions are exposed.
- **Page Header:** required `title`, optional nonblank `description`, fixed `h1`. Optional named `actions` child is ordinary app-authored markup admitted by Native, not an HTML-valued attribute. Use `<cui-slot name="actions">…</cui-slot>`; do not pass a raw HTML option or create multiple page-level headings.

Empty State, Metric, and Page Header text accepts literals or complete checked page-field interpolation, subject to ordinary Native template admission and runtime escaping. The adapter emits generic Native runtime value checks for bound values; ordinary CLI fixtures do not verify a real app's runtime-bound inputs. Structural choices, ordinary navigation destinations, icon names, and Skeleton geometry remain checked literals or declared route names. Avatar image URLs and initials have dedicated checked runtime bindings. Query authors remain responsible for meaningful nonblank bound labels. No browser-local state, component-owned Datastar signal, or arbitrary expression passthrough is introduced.

## Layout and admitted child slots

The 0.5.0 package adds six static layouts. Their child markup is a host-admitted slot, never a plain-text attribute or fixture-style `children` input in an app declaration. Named slots use `<cui-slot name="…">…</cui-slot>` and are admitted by the Native host before composition:

```html
<cui-card padding="standard" title="Recent activity">
  <cui-slot name="body"><p>Three deployments completed.</p></cui-slot>
  <cui-slot name="actions"><a href="/deployments">View deployments</a></cui-slot>
</cui-card>
<cui-split ratio="start-wide">
  <cui-slot name="start"><section>Primary region</section></cui-slot>
  <cui-slot name="end"><aside>Supporting region</aside></cui-slot>
</cui-split>
<cui-grid columns="responsive"><cui-slot name="body">…</cui-slot></cui-grid>
```

Card requires `body`, optionally accepts `actions`, and has optional title/subtitle with `h2` default heading level. Split requires `start` and `end` and renders them in that order. Container, Stack, Cluster, and Grid accept implicit body children or an explicit `body` slot. Container options are `width` (`reading`, `compact`, `standard`, `wide`, `full`) and `gutter` (`none`, `compact`, `standard`, `generous`). Stack options are `gap` (`none`, `compact`, `standard`, `spacious`, `generous`) and `alignment` (`stretch`, `start`, `center`, `end`, `baseline`). Cluster adds `justification` (`start`, `center`, `end`, `between`) and `wrapping` (`wrap`, `nowrap`); defaults are compact gap, centered alignment, start justification, and wrapping. Grid options are `columns` (`responsive`, `two`, `three`, `four`), `minimum` (`narrow`, `standard`, `wide`), gap, alignment, and `collapse-at` (`compact`, `standard`, `wide`, `never`). Split options are `ratio` (`equal`, `start-wide`, `end-wide`, `start-dominant`, `end-dominant`), gap, alignment, and `collapse-at`; regions remain stacked until that container-query breakpoint. Gap values are `none`, `compact`, `standard`, `spacious`, `generous`; layout defaults and allowed typed values are defined in each manifest and `catalog-core::layout`. Grid intentionally has no masonry mode. Package fixtures verify static rendering; host admission, responsive behavior, and browser output remain separate checks. `collapse-at="never"` means never collapse: fixed Grid, Split, Sidebar, and Switch layouts expand immediately, including on narrow screens.

The 0.6.0 package adds the remaining six Toolframe layout-category components. Together with Divider and Page Header, the catalog now covers all fourteen Toolframe layouts; this is not parity with all fifty-four Toolframe components.

| Layout | Child slots | Typed options and behavior |
| --- | --- | --- |
| Cover | Required `primary`; optional `top`, `bottom` | `height`: compact, standard (default), fill, viewport; `gap`. Centers primary between optional chrome without changing DOM order. |
| Layer | Required `base`, `foreground` | `placement`: center (default), top-start, top-end, bottom-start, bottom-end, stretch; `inset`: the gap presets. Grid overlap, not a modal or focus trap. Base controls remain reachable outside foreground content. |
| Pane | Required `body`; optional `header`, `footer` | `height`: content (default), compact, standard, fill, viewport; optional `body-scrolling`. Bounded heights scroll the body by default; content height does not. An explicit boolean overrides scrolling independently of height. |
| Reel | Implicit children or explicit `body` | Required nonblank `label`; `item-width`: narrow, standard (default), wide, content; `snap`: free (default), start, center; `gap`. Labelled, keyboard-focusable native horizontal scrolling, not a carousel lifecycle. |
| Sidebar | Required `main`, `aside` | `side`: start, end (default); `width`: narrow, standard (default), wide; `gap`, `alignment` (default start), `collapse-at`. DOM order follows the selected rail side, including when stacked. |
| Switch | Implicit children or explicit `body` | `sizing`: natural (default), equal; `gap`, `alignment`, `justification`, `collapse-at`. Switches the complete group from a column to one row; it is not a boolean input. |

```html
<cui-sidebar side="start" width="narrow" collapse-at="compact">
  <cui-slot name="aside"><nav aria-label="Sections"><a href="#activity">Activity</a></nav></cui-slot>
  <cui-slot name="main"><section id="activity"><h2>Activity</h2><p>Illustrative workspace content.</p></section></cui-slot>
</cui-sidebar>
<cui-pane height="compact">
  <cui-slot name="header"><h2>Recent activity</h2></cui-slot>
  <cui-slot name="body"><p>Illustrative activity entries.</p></cui-slot>
  <cui-slot name="footer"><a href="#activity">View activity</a></cui-slot>
</cui-pane>
```

`fill` requires an app-owned parent with a definite block size. Viewport variants subtract the app-overridable viewport-offset token. A bounded Pane with `body-scrolling="false"` can clip oversized content; the app must ensure it fits. None of these layouts introduces app routes, commands, authorization, JavaScript, or arbitrary CSS-valued options.

## Read-only result and form components

The 0.7.0 package adds five presentation contracts: Select Field, Filter Bar, Data Table, Breadcrumbs, and Pagination. All 54 Toolframe contracts are component-complete. Native supports the previous 45 within their documented scope. Modal, Drawer, Popover, Command Menu, Confirm Dialog, Date Calendar, Date Picker, File Upload, and Data Viewport retain `integration.native.status: adapter-required`; they are not silently admitted to Native templates.

The nine have closed Rust validation, locked variant goldens, browser lifecycle/keyboard coverage, and desktop/mobile/no-JavaScript conformance tests. Confirm Dialog, File Upload, and Data Viewport expose explicit app-owned adapter ports; date components emit typed browser-draft events. Their `.d.ts` contracts are locked inputs, not transport implementations. Test adapters exercise rejection, cancellation, and recovery without uploading bytes, querying a backend, or claiming command success. `../clanker-ui-gallery/` owns visual usage examples; the package repository keeps reusable fixtures and conformance tests.

- **Select Field** emits a labelled native single-select with 1–100 unique choices, optional hint/error, required/disabled state, and checked selection. Use paired `<cui-option value="…" label="…" />` declarations for checked app data, or a closed static `choices` JSON array. The app owns the surrounding form and validation.
- **Filter Bar** is a labelled `div role="group"`, not a form. Its required `controls` and optional `actions`/`applied` slots accept only host-admitted children. An optional whole-field `summary` is announced as status text.
- **Data Table** has a required caption and a closed JSON column-label array. Native admits paired `cui-table-row`/`cui-table-cell` declarations, checks cell counts, and creates row IDs from a static `id-prefix` plus a checked `key`. Narrow layouts preserve every cell and column label. Filtering, ordering, paging, and row actions belong to the app.
- **Breadcrumbs** uses linked `cui-crumb` ancestors followed by one current, nonlinked item. **Pagination** uses `cui-page` items with closed kinds and standard/outlined/compact presentation. Destinations are admitted app route calls or safe literals—not arbitrary bound URL strings. Runtime guards require exactly one current item.

Native additionally admits read-only `<form data-page="registered_name">` controls for fixed-path page routes. It derives the GET action, checks declared scalar query fields and defaults, and revalidates dynamic select options after rendering. This is host support, not component-owned transport; command forms retain their existing ticket, authorization, and revision checks. The Button configurator uses provider-authored ordinal-to-class branches and generic bounded integer checks, not arbitrary bound CSS or variant strings. Native also checks whole boolean fields before materializing selected/checked controls.

## Seven static Native ports

Activity Feed, Button Group, Definition List, Disclosure, Progress Steps, Segmented Control, and Tabs are ready for static/native HTML. Activity Feed uses ordered timestamped entries; Definition List uses native `dl` semantics and admitted rich values; Button Group composes admitted Buttons; Disclosure uses native `details`/`summary`. Tabs and Segmented Control are ordinary page navigation, not client-side tab panels. Progress Steps is app-supplied workflow presentation, not a workflow engine.

Native JSON `href` values in Activity Feed, Tabs, Segmented Control, and Progress Steps may contain a **whole named route call**, such as `"href":"{{ routes.tasks() }}"`. They reuse the Breadcrumbs/Pagination grammar. The host checks route existence, argument types, reference codecs, and emission provenance; arbitrary field-based URLs and mixed expressions remain rejected. Literal internal paths still require host route admission. Serialized portable contracts remain safe-URL data, not template programs.

`tests/fixtures/static-native-proof.html` and its app-owned CSS were admitted in a disposable gallery copy using a separate platform snapshot and a locally digest-pinned CLI. The original gallery, its data, and the shared platform's selected artifact were not changed. Browser checks cover native semantics, disabled/busy Buttons, keyboard links and disclosure toggling, no page-wide mobile overflow, and JavaScript-disabled navigation/disclosure. A negative Native build rejected an unregistered route. These checks used illustrative content, not production data. The copied gallery's sidebar still describes its older 31-entry explorer.

To rerun `tests/browser/static-native-proof.mjs`, first build and authenticate the disposable Native app, then prepend `globalThis.staticNativeProof = {origin, space, screenshots}` and pass the resulting script to `ego-browser nodejs`. Use its authenticated TaskSpace ID and an absolute screenshot directory. This is an integration check, not a `file://` rendering proof; hosted CLI acquisition and target qualification are separate release gates. Only the local example lock was refreshed; existing sibling consumer locks require an explicit refresh before their next rebuild.

## Native choice controls

Checkbox Group, Radio Group, and Toggle use ordinary browser drafts and the app's existing command form. Native admits repeated `List(Str)` checkbox values, scalar radio values, and Boolean toggles; it excludes disabled choices from editable authority and keeps fully disabled groups host-bound. Empty selections, forged choices, keyboard operation, rejection recovery, and JavaScript-disabled submission passed in a disposable gallery. The app-owned proof command validates and echoes sample preferences; it changes no task or project. See `tests/browser/choice-native-proof.mjs` and the command fixtures under `tests/fixtures/native-choice-command/` for the desktop/mobile proof.

## Browser-local enhancements

Copy Field, Theme Switcher, Tooltip, and Toast are ready for **literal static-page declarations**. Clanker selects closed module entrypoints; Native independently admits, stages, and serves the complete import graph. CLI `verify` checks declared inputs and fixtures, not browser execution or host import admission. Legacy `build`/`compose` still cannot stage these enhancements and are not the Native app integration path.

- Copy Field retains a selectable readonly input. Clipboard feedback cannot claim a changed/replaced value or mutate after teardown; events contain no clipboard content.
- Theme Switcher writes only the configured app-owned `data-cui-theme` target. System preferences resolve to concrete palettes; teardown preserves the app-owned target's current theme rather than undoing it. The component owns no account/server preference or theme CSS.
- Tooltip keeps help visible in normal flow without JavaScript, hiding its inert trigger. Enhancement adds contextual positioning, hover/focus behavior, and Escape dismissal that preserves trigger focus.
- Toast dismisses presentation only, pauses timers during hover/focus/hidden-document periods, and releases timers on removal/teardown. Its explicit history policy controls local restoration; the app owns message content and meaning.

The disposable Native build and `tests/browser/enhancement-native-proof.mjs` passed desktop/mobile bounds, semantics, clipboard success/denial, scoped/system themes, keyboard dismissal, timer pause/cancellation, and no-JS fallbacks. Clipboard writes were stubbed; no OS clipboard content was read or overwritten. These checks used static illustrative data. Supplemental DOM replacement, teardown/reinstall, and synthetic persisted-event probes passed, but **actual Native live-region patching and real BFCache restoration remain unproven**. Do not infer those capabilities or runtime page-field bindings from this readiness scope. Host proof used an explicit local pin and isolated sources; it does not establish hosted CLI acquisition or release-target qualification.

## Standalone Native gallery

`../clanker-ui-gallery` is the current real consumer and Clanker Studio target. It is independent of GoLinks: the app owns two project/task models, typed create/edit/seed commands, queries, routes, and an orange-accented neutral theme. `/` has live project summaries and recent tasks beside an independent create form. `/tasks` supplies app-owned search, project/status filters, ordering, and six-row pagination. `/components` exposes 45 Native-supported component entries, named presets, and a checked Button configurator. The nine adapter-required contracts use a separate, explicitly labeled browser fixture lab. Theme/density selections use read-only page forms. `/tasks/{task_id}` retains a revision-checked editor; `/layouts` remains the detailed layout atlas.

From the sibling `platform` checkout:

```sh
./cli/day2 platform local-dev ../clanker-ui-gallery --example demo --directory /tmp/cui-gallery-demo --detach
```

The seed creates two illustrative projects and eight tasks only when both tables are empty. Current browser checks used separate temporary instances: all 31 default specimen routes rendered, task search/pagination and checked Button controls worked, and mobile tables preserved every field without page-wide overflow. Create/edit and seed initialization/no-op passed; a stale edit retained its draft without overwriting revision 2. No-JS GET filtering and empty-gallery initialization were also exercised.

Keep review screenshots outside the source checkout and published package. PR evidence may use review attachments or immutable historical links; local captures are not portable integration tests or production-data evidence.

The gallery's README documents the current Native rejection limitation: a failed submission preserves its original signed ticket, so corrected retries require copying the draft and reloading. Neither the gallery nor Studio bypasses revision protection. Out-of-range page-query choices are rejected, but Native currently presents declared query failures as a generic HTTP 500; the gallery does not alter that global host policy. Studio's local `instance/native-page.json` targets the gallery; its existing desktop-proof mode remains read-only and does not load the component catalog. The sibling package lock and machine-local preview origin still need portable provisioning before CI or distribution.

## Ownership and patch contracts

A component contract distinguishes configuration, app-owned presentation data, browser-local state, native outputs, composition, and lifecycle in its manifest invariants. Typed Rust instances and the CLI enforce component/declaration contracts; Native separately enforces app-context admission. Fixture verification does not prove that arbitrary app data meets a nonblank-label requirement: app queries must supply meaningful accessible text.

| Component | State owner | Output and lifecycle |
| --- | --- | --- |
| Button | App supplies busy/disabled data; no component signal | Native activation, submission, or admitted navigation; no mount hook |
| Icon, Badge, Divider, Status Indicator | App supplies presentation; no internal state | Ordinary markup; no events or announcement lifecycle |
| Tag | App supplies metadata/filter state | Optional navigation only; removal does not silently dismiss |
| Alert | App supplies message/recovery state | Optional navigation and explicit announcement policy |
| Progress | App owns measured task state | Native progress semantics; no polling or invented numeric state |
| Form Field, Select Field, Checkbox Group, Radio Group, Toggle | Browser owns its current draft; app owns initial value, choices, and validation | Native controls and surrounding admitted form submission; no separate command transport |
| Filter Bar, Data Table | App owns filters, source rows, ordering, paging, and actions | Admitted presentation/composition; no query or command ownership |
| Breadcrumbs, Pagination, Tabs, Segmented Control | App supplies current position and admitted destinations | Ordinary navigation; no route, panel, query or pagination state ownership |
| Activity Feed, Definition List, Progress Steps | App supplies history, labelled values or workflow status | Native/read-only presentation; no query, time formatting or workflow ownership |
| Button Group | App supplies admitted Buttons and group label | Composition only; each Button retains its ordinary semantics |
| Disclosure | App supplies initial open state and admitted body | Browser-native toggle; no JavaScript or recreated ARIA state |
| Copy Field, Theme Switcher, Tooltip, Toast | App supplies literal configuration/content; component owns bounded local presentation only | Host-staged installers and exact-root teardown; no commands, routes, or server-state ownership |
| Avatar, Metric, Page Header | App supplies identity, formatted measurements, and page context | Named identity or ordinary text; no events, calculation, or mount hook |
| Empty State | App owns result/empty state and recovery destination | Optional admitted navigation; no command or dismissal |
| Skeleton | App owns loading lifecycle and busy/text semantics | Decorative hidden markup; optional reduced-motion-aware CSS animation |
| Container, Stack, Cluster, Grid, Split, Card, Cover, Layer, Pane, Reel, Sidebar, Switch | App supplies admitted child content and layout choices | Build-time composition, responsive CSS, and native scrolling; no signals or business operations |

Keep editable controls outside app-owned `data-live` query regions. The host patches live data independently, replaces the submitted form on success, and preserves a rejected draft. Stable IDs and non-overlapping region ownership matter; components do not add a second Datastar state catalog. All fragments use single-pass substitution so slot-looking text remains text. Browser-local components require explicit ownership, reset/retention rules, outputs, no-JS behavior, and cleanup. The four admitted enhancements have bounded static-page proof; they do not establish Native live-region replacement or controlled runtime bindings.

## Locked agent context and diagnostics

`cargo run --locked --offline -- capabilities` reports the installed CLI's supported commands and explicit limitations without opening an app. `context <name> --lock <path>` returns the locked manifest, typed property descriptions, dependency-first component selection, and one declared fixture input. It lists every other fixture path and the host/browser checks still required. It does not copy component source into an application.

Components may declare a structured `templateAuthoring` contract in their manifest. `describe`, `context`, and `properties` forward it generically: paired child grammar, attribute value forms, identity/content constraints, and inline Native template examples. It is guidance, not editable properties, raw HTML input, or an executable renderer schema. Data Table includes a customer loop with composed badge/indicator cells and an empty row alongside its existing text-row fixture; Filter Bar, Select Field, Breadcrumbs, and Pagination describe their helper children. Read the example's app-field and route requirements before adapting it. Native Studio forwards the manifest object; an installed CLI and app lock must be explicitly refreshed together to see changed package bytes.

`verify` expands each declared authoring example through the existing Clanker renderer, without app I/O. This catches declaration drift, not route registration, page-field types, command authority, or browser behavior; Native still admits the app's final template. The manifests remain the source of truth; `property-catalog.json` is a freshness-checked projection.

`doctor --lock <path>` checks local package identity and bytes, dependency closure, typed fixtures, declared theme tokens, generated property-catalog freshness, and authoring example expansion. Its scope is the package: a passing result is not a Native app build or browser acceptance. Neither command mutates files or needs the network. App initialization and upgrades remain outside this CLI rather than pretending the F# project workflow applies to Native.

## Discover and check theming

`clanker-ui tokens --lock <lock> [--component <name>]` reports described token ownership, role, purpose, semantic grouping, CSS baseline defaults, resolution traces, and direct/transitive component readers. `describe` and `context` expose the same information in `component.tokenDetails`; the legacy `component.tokens` name array remains unchanged. Definitions live in component `tokenDescriptions` and the theme metadata path declared by the package's `tokenMetadata` field. CSS remains the default-value authority. Draft descriptors do not establish component readiness; unresolved or contextual defaults are reported rather than guessed.

`clanker-ui check-css --ui <dir> --lock <lock>` reads authored CSS/HTML without changing files or running app code. It reports unknown `--cui-*` sets/reads as errors; internal selectors, unscoped component element styling, misplaced component-token consumption, competing raw color rules, and suspicious `!important` as warnings. Diagnostics use the usual JSON envelope and `CUI001`, with a distinct `rule` and source location. Errors exit nonzero; warnings alone do not. `capabilities` advertises both commands. Check the report's limitations: template branches are possible static structure, not computed browser cascade; JavaScript-created styles and external CSS are not executed or fetched.

Use shared tokens for coordinated changes and component tokens for deliberate exceptions:

```css
:root { --cui-heading-text: green; }
.customer-page { --cui-surface-subtle: #f0f5f1; }
```

Page Header titles, Card titles, Empty State titles, and Data Table captions/column headers fall back to the shared heading color. Individual heading tokens remain overridable. These defaults use component-local fallbacks so a shared token override on an app subtree still works; they do not add global `h1`/`th` rules or recolor semantic status badges/alerts. `--cui-accent` remains the shared accent. Changing declared metadata or CSS bytes requires an explicit lock refresh; existing app and Studio installation pins are not updated by either command.

## What an agent should do

Given a request such as “add a searchable table with row actions and a details drawer,” an agent should:

1. Ask the local component catalog for candidates, filtered by target, purpose, and constraints.
2. Inspect the short category index, relevant component contracts, examples, and *use when / avoid when* guidance.
3. Select compatible components and bind them to the app's existing typed queries, commands, and routes.
4. Configure the app's tokens and declare selected components in app templates without inventing alternate markup or bypassing their interaction contracts.
5. Run package and target-specific checks; inspect the rendered result and its keyboard, no-JS, and responsive behavior.

An agent may search by words or intent. Search ranks candidates; it never defines component behavior. A generated, browsable category tree provides a predictable fallback when semantic search misses. Both views derive from the same manifests rather than handwritten parallel indexes. A small CLI should expose structured `search`, `describe`, and dependency-graph results as well as readable Markdown for agents and people. The CLI reports the exact package version and source of each result.

## Package and component contracts

A package has a scoped identity such as `@organization/controls`, a version, supported targets, dependencies, provenance, and a set of components. A component has a stable, package-qualified identity such as `@organization/controls/button`. Package scanning treats a recognized, colocated metadata file as an explicit opt-in: other source files are not catalog entries. Missing metadata means the component is not ready and is omitted from indexes; metadata present but malformed is a build error with a file path, not a silently missing component. Its manifest describes:

- Purpose, categories, search terms, *use when / avoid when*, and composition guidance.
- Semantic role; typed props, slots, variants, and states; required labels; and action/navigation behavior.
- Supported targets and the target-specific renderer or source assets.
- Semantic and component token requirements, defaults, responsive behavior, and visual states.
- Owned HTML, CSS, JavaScript, events, and dependencies, including any dynamic import graph.
- Accessibility and no-JS expectations, fixtures, tests, and verification commands.

Use variants for different appearances of the same semantic control. Give controls with different semantics—such as links, submit buttons, and menu triggers—different contracts when their obligations differ. Keep component-authored styles and tests near their source; generated bundles need not be pleasant for an agent to read.

Authors can add components in their own packages. An app may choose a replacement explicitly in its lock or configuration, but a later package must not silently shadow an existing component ID. Contract-compatible replacement and a new component identity are distinct choices. Third-party manifests are claims to validate, not proof that third-party code is safe.

## Local dependency, built artifact

Authors publish packages in their own repositories. A consumer obtains a package locally before building; a build must not silently fetch an unpinned branch or resolve a new version from the network. The app records exact package identities, versions, source revisions, and content digests in a lock. The agent reads that installed, locked package rather than copying snippets from an unverified search result.

The **package source and discovery CLI are development/build dependencies**. The HTML templates, CSS, and JavaScript selected from them are **runtime assets**: the build stages their complete dependency closure into the app artifact, pins their bytes, and serves them through the host's existing admission rules. No browser request or production process should need the source checkout, package registry, or live GitHub repository. Clanker Native apps do not gain per-app npm installs, scripts, or CDN imports.

For each app, the build starts from the selected components and computes the union of their declared dependencies. Shared JavaScript modules and styles appear once per artifact when their resolved identities and bytes agree. Conflicting versions or identical IDs with different bytes fail rather than relying on load order. The first optimization is deterministic reachability and deduplication, not a promise of general JavaScript dead-code elimination. Page-level splitting can follow if measured loading costs justify it.

## Targets, themes, and enforcement

The portable part is a component's discovery, semantics, constraints, and verification vocabulary. A target adapter supplies rendering and integration rules. Start with a Clanker Native HTML/Datastar target; do not build a general plugin loader before a second target exists. A React package can participate in discovery for a React target without making React components automatically renderable in Clanker Native. If a host later supports a React island, it must give that island exclusive DOM ownership and admit its compiled resources.

Each app owns its theme. Packages use semantic CSS custom properties and component-level defaults for color, spacing, type, density, radius, motion, focus, hover, selected, disabled, and other states. The target checks required tokens and emits the app's selected theme with the compiled assets. JavaScript should be component-scoped and purposeful: focus, keyboard interaction, and morph lifecycle may need it in addition to animation. Optional motion libraries must be declared and pinned, not included by default.

Agent instructions and examples make correct composition easy; checks make it accountable. A strict, opt-in Clanker Native UI integration should validate component selection, props, variants, required accessibility features, supported targets, token completeness, bindings, and asset closure. Clanker Native's existing template/form admission still applies. Plain custom UI remains possible without the package; custom components can join its checked path by publishing a contract and fixtures. Neither manifest validation nor dependency admission is a sandbox for third-party browser code.

## Development split

Keep three boundaries separate while developing them together in this folder:

1. **Catalog and agent CLI.** Parse and validate package/component contracts, build the generated navigation and search index, expose `search`, `describe`, and dependency/verification information, and resolve local packages into an app-specific lock. This layer knows intent, compatibility, package identity, and provenance. Its pure expansion core renders symbolic templates without interpreting Roc business data; script-free scenes use explicitly supplied fake data. An MCP server could later wrap the same CLI/library API if agents benefit from it, not become a second catalog.
2. **Vanilla component package.** Adapt the proven Toolframe components into portable semantic HTML, colocated CSS, small browser modules, themes, fixtures, and accessibility behavior. Keep a single component source contract with target-specific assets where necessary. The package does not own an app's design override, queries, commands, routes, or persistent state.
3. **Clanker Native adapter.** Invoke the pinned CLI once, verify its declared inputs and outputs, and stage ordinary templates/resources in the private snapshot. Independently admit templates and resources against app capabilities. The platform owns admission, staging, serving, and Datastar integration—not component rendering or helper semantics. A future React adapter can consume the same discovery model without pretending that the rendering source is identical.

The agent selects components and declares their options at the point of use; the build expands them deterministically. Avoid making the agent hand-copy template, CSS, and JS files or maintain a second manual asset list. Changes to an app-owned theme stay in the app and do not mutate the downloaded package. Keep a package's default tokens and required semantic roles explicit so an app can override values without forking component source.

The local authoring loop should be short: acquire a package in a local checkout or cache; pin its declared bytes; discover components through the CLI; write declarations in app templates and app-owned token overrides; run the Native build and inspect the rendered UI. Registration is per app through the lock, not an ambient global install that silently changes builds. CLI responses should be versioned, machine-readable, and include actionable diagnostics. Toolframe already proves `inspect → narrow context → preview → apply → verify` and provides `list`, `describe`, `graph`, and `context`; semantic `search` is proposed here, not a command in the F# CLI. The package and lock formats remain provisional, despite the tested button integration.

## Evidence and migration order

The F# Toolframe has 54 component manifests across layout (14), forms (10), data (8), feedback (7), overlays (6), navigation (5), and actions (4). All 54 are marked `experimental`; a complete catalog is not evidence that every component is ready to ship in another host. Its agent tooling already provides `list`, `describe`, `graph`, and narrow `context`, versioned JSON diagnostics, preview/apply, verification, and a catalog lock. The proposed `search` and multi-package resolver are new work.

Start by importing **catalog knowledge for one component**: purpose, category, dependency edges, accessibility and responsive requirements, fixtures, and agent guidance. Leave the other Toolframe components unregistered until each has validated metadata in a package; port runtime components by dependency and risk, not by copying all F# source files. Semantic HTML, styling, and behaviors can inform a new implementation; F# constructors and Scriban templates cannot execute in the Roc/Rust host. For example, Toolframe `button` depends on `icon`, `data-table` on `badge`, and interactive `date-picker` on `date-calendar`, `icon`, and its browser interaction. Each migrated slice must prove its own target-specific contract and fixtures (`../../internal-tools-toolframe/src/components/`). Do not mutate the original Toolframe workspace.

Prove the system in bounded steps:

1. Define the local package/component manifest and lock for one component. Test opt-in discovery, omission of unmarked files, and diagnostics for malformed declared entries without claiming 54 usable Native components.
2. Implement **button** and **icon**, then the independent static **badge**, **divider**, and **status-indicator** slices, followed by **tag**, **alert**, **progress**, native **form-field**, and the **avatar**, **empty-state**, **metric**, **skeleton**, and **page-header** presentation slice, with app-owned token overrides, closed icon catalog, golden rendering fixtures, Native admission, and a real app proof. Port an interactive component only after the native browser-module lifecycle and no-JS fallback work.
3. Prove the CLI in this folder without modifying Clanker Native: read a locally locked package, preview and materialize deterministic output into a separate staging directory, reject conflicts, and validate the complete output graph. Do not silently copy into an app or fetch an unpinned Git branch during a build.
4. After that proof, validate and extend the optional Clanker Native adapter around its existing UI packaging seam (`../platform/crates/xtask/src/build_native.rs`), generated template handles, page/form checks, and pinned resource catalog. Validate a real app build and browser flow before claiming integration. The current host accepts relative, admitted `ui/` JavaScript imports, not an external package catalog (`../platform/crates/day2/src/web_resources.rs`). Keep Native's runtime admission and app operation catalog authoritative.
5. Expand the migrated catalog by tier—static controls, layout/data, then overlays and focused interactions—and only then test an independently authored package or second rendering target. Add semantic ranking if structured search and the generated category tree prove insufficient.

Decide the exact manifest and local lock formats, override compatibility, CSS assembly method, and shared-versus-target-specific props from the first proof. The proven F# NuGet/MSBuild/Sass consumption path is not itself the Native solution (`../../internal-tools-toolframe/ARCHITECTURE.md`).
