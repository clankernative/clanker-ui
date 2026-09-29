# Clanker Native UI

**Status:** provisional package and build-time Native adapter for five components. The old per-instance `compose` output is deprecated. Local package paths are not yet a portable CI dependency; this is not production-ready.

Clanker Native UI is an optional, agent-first component discovery and assembly system. It helps an app-building agent find a component that fits a user's intent, read its contract, compose it for the app's target, and verify the result. The vanilla package provides Button, Icon, Badge, Divider, and Status Indicator with target-specific HTML and CSS. Native JavaScript interactions and Datastar integration are later slices. Other authors could publish their own packages and target adapters, including React catalogs, without making React a dependency of Clanker Native.

Clanker Native remains usable without Clanker Native UI. Apps keep ownership of their domain models, queries, commands, routes, presentation, and design flavor. Clanker Native UI describes and supplies UI components; it does not become a second app-operation catalog or an arbitrary callback SDK.

## Try the local proof

The Rust workspace has a pure `catalog-core` crate and a `clanker-ui` discovery CLI. Clanker Native's build owns target-specific expansion into its private staging area. The ready vanilla components are `button`, `icon`, `badge`, `divider`, and `status-indicator`. A component directory without `component.json` is ignored; malformed declared metadata fails. The lock at `examples/button-app/clanker-ui.lock.json` pins declared package inputs, not every file in the checkout. The app owns `examples/button-app/theme.css`; it does not edit package defaults.

From this directory:

```sh
cargo run --locked --offline -- find button --lock examples/button-app/clanker-ui.lock.json
cargo run --locked --offline -- describe button --lock examples/button-app/clanker-ui.lock.json
cargo run --locked --offline -- graph button --lock examples/button-app/clanker-ui.lock.json
cargo run --locked --offline -- verify --lock examples/button-app/clanker-ui.lock.json
cargo run --locked --offline -- find icon --lock examples/button-app/clanker-ui.lock.json
cargo run --locked --offline -- describe icon --lock examples/button-app/clanker-ui.lock.json
cargo run --locked --offline -- list --lock examples/button-app/clanker-ui.lock.json
cargo run --locked --offline -- graph badge --lock examples/button-app/clanker-ui.lock.json
```

`lock --lock <file> --package <path>` pins a local package relative to the lock file; changing its declared bytes requires `--update`. The CLI emits versioned JSON and nonzero status on failure. Its `verify` checks manifest shape, declared inputs, token references, and self-authored fixtures; it is not a substitute for Native's independent admission. Word matching is literal, not embedding-based search. The old `build` and `compose` commands still produce isolated generated files for the earlier proof, but **do not use them to integrate an app**.

A Native app pins the package in `ui/clanker-ui.lock.json` and declares a button directly in its template, inside an existing app-owned command form when submitting:

```html
<cui-button kind="submit" variant="primary" icon="plus" label="Create" />
```

The Native build checks the package digest, expands each declaration in a private snapshot, adds package CSS and optional `ui/clanker-theme.css` overrides to staged `ui/app.css`, then applies normal template, form, and resource admission. Local dev invokes the same build; neither the package nor the CLI runs at request time. The app contains no generated component HTML or CSS. The local GoLinks proof in `../golinks-clanker-ui-button` uses this path; its sibling package location must be replaced with a provisioned, pinned source before CI can build it elsewhere.

Button attributes are `kind` (`action`, `submit`, `link`), `label`, `variant` (`primary`, `secondary`, `danger`, `quiet`), `size` (`compact`, `standard`), optional `icon`, `edge-aligned`, `disabled`, `busy`, and `busy-label`. A link uses a safe `href` or app-owned `route`. Labels may be app-authored literals or complete typed page-field interpolations. Dynamic states select interactive or noninteractive markup from checked Bool fields; busy requires a replacement label. Disabled or busy links have no href. The standalone `<cui-icon name="search" size="medium" label="Search" />` contract uses the same closed 100-glyph catalog. Icons are decorative/hidden from assistive technology by default; standalone meaningful icons require a nonblank `label` and render as `role="img"`. Icon fragment HTML and CSS are package assets; the target adapter supplies SVG attributes and geometry through the exact `[[attributes]]` and `[[geometry]]` slots. The button does not create a command, form fields, or route. JavaScript-bearing components, multi-package assembly, and production package distribution remain future work.

The new static declarations use literal text only; they do not yet accept typed page-field interpolation:

```html
<cui-badge tone="neutral" label="Active links" show-icon="false" />
<cui-divider orientation="horizontal" label="Details" alignment="start" />
<cui-status-indicator tone="neutral" label="No links match your filters." />
```

Badge requires visible `label` text, accepts tones `neutral`, `info`, `success`, `warning`, `danger`, and `running`, and can suppress or override its decorative icon with `show-icon` or a closed-catalog `icon` name. Divider defaults to a horizontal, unlabelled separator; it accepts a `start`, `center`, or `end` label alignment and rejects labels on vertical separators. Status Indicator requires visible `label` text, accepts tones `neutral`, `info`, `success`, `warning`, and `danger`, plus optional `detail`, `size` (`small` or `large`), and `pulse`; it is static, not an ARIA live region. Each component is noninteractive, checks unknown attributes, and uses app-overridable `--cui-*` tokens. A real GoLinks build in `../golinks-clanker-ui-button` exercises all three. [Desktop](docs/screenshots/golinks-static-desktop.png) and [mobile](docs/screenshots/golinks-static-mobile.png) captures show the local proof with an empty sample list, not production data.

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

1. **Catalog and agent CLI.** Parse and validate package/component contracts, build the generated navigation and search index, expose `search`, `describe`, and dependency/verification information, and resolve local packages into an app-specific lock. This layer knows intent, compatibility, package identity, and source provenance; it does not render UI or interpret app business data. An MCP server could later wrap the same CLI/library API if agents benefit from it, not become a second catalog.
2. **Vanilla component package.** Adapt the proven Toolframe components into portable semantic HTML, colocated CSS, small browser modules, themes, fixtures, and accessibility behavior. Keep a single component source contract with target-specific assets where necessary. The package does not own an app's design override, queries, commands, routes, or persistent state.
3. **Clanker Native adapter.** Resolve selected, pinned components into the build's private snapshot; expand app-authored declarations, then admit the resulting templates and resources against existing typed query/command bindings. This adapter owns integration with Datastar and the platform, not the portable catalog. A future React adapter can consume the same discovery model without pretending that the rendering source is identical.

The agent selects components and declares their options at the point of use; the build expands them deterministically. Avoid making the agent hand-copy template, CSS, and JS files or maintain a second manual asset list. Changes to an app-owned theme stay in the app and do not mutate the downloaded package. Keep a package's default tokens and required semantic roles explicit so an app can override values without forking component source.

The local authoring loop should be short: acquire a package in a local checkout or cache; pin its declared bytes; discover components through the CLI; write declarations in app templates and app-owned token overrides; run the Native build and inspect the rendered UI. Registration is per app through the lock, not an ambient global install that silently changes builds. CLI responses should be versioned, machine-readable, and include actionable diagnostics. Toolframe already proves `inspect → narrow context → preview → apply → verify` and provides `list`, `describe`, `graph`, and `context`; semantic `search` is proposed here, not a command in the F# CLI. The package and lock formats remain provisional, despite the tested button integration.

## Evidence and migration order

The F# Toolframe has 54 component manifests across layout (14), forms (10), data (8), feedback (7), overlays (6), navigation (5), and actions (4). All 54 are marked `experimental`; a complete catalog is not evidence that every component is ready to ship in another host. Its agent tooling already provides `list`, `describe`, `graph`, and narrow `context`, versioned JSON diagnostics, preview/apply, verification, and a catalog lock. The proposed `search` and multi-package resolver are new work.

Start by importing **catalog knowledge for one component**: purpose, category, dependency edges, accessibility and responsive requirements, fixtures, and agent guidance. Leave the other Toolframe components unregistered until each has validated metadata in a package; port runtime components by dependency and risk, not by copying all F# source files. Semantic HTML, styling, and behaviors can inform a new implementation; F# constructors and Scriban templates cannot execute in the Roc/Rust host. For example, Toolframe `button` depends on `icon`, `data-table` on `badge`, and interactive `date-picker` on `date-calendar`, `icon`, and its browser interaction. Each migrated slice must prove its own target-specific contract and fixtures (`../../internal-tools-toolframe/src/components/`). Do not mutate the original Toolframe workspace.

Prove the system in bounded steps:

1. Define the local package/component manifest and lock for one component. Test opt-in discovery, omission of unmarked files, and diagnostics for malformed declared entries without claiming 54 usable Native components.
2. Implement **button** and **icon**, then the independent static **badge**, **divider**, and **status-indicator** slices with app-owned token overrides, closed icon catalog, rendering fixtures, Native admission, and a real app proof. Port an interactive component only after the native browser-module lifecycle and no-JS fallback work.
3. Prove the CLI in this folder without modifying Clanker Native: read a locally locked package, preview and materialize deterministic output into a separate staging directory, reject conflicts, and validate the complete output graph. Do not silently copy into an app or fetch an unpinned Git branch during a build.
4. After that proof, validate and extend the optional Clanker Native adapter around its existing UI packaging seam (`../platform/crates/xtask/src/build_native.rs`), generated template handles, page/form checks, and pinned resource catalog. Validate a real app build and browser flow before claiming integration. The current host accepts relative, admitted `ui/` JavaScript imports, not an external package catalog (`../platform/crates/day2/src/web_resources.rs`). Keep Native's runtime admission and app operation catalog authoritative.
5. Expand the migrated catalog by tier—static controls, layout/data, then overlays and focused interactions—and only then test an independently authored package or second rendering target. Add semantic ranking if structured search and the generated category tree prove insufficient.

Decide the exact manifest and local lock formats, override compatibility, CSS assembly method, and shared-versus-target-specific props from the first proof. The proven F# NuGet/MSBuild/Sass consumption path is not itself the Native solution (`../../internal-tools-toolframe/ARCHITECTURE.md`).
