# Clanker Native UI — agent field guide

Clanker Native UI is a provisional **agent-first component catalog and build-time assembly system**. It is not a runtime UI framework. An agent discovers a component, reads its manifest and fixtures, chooses typed options, and declares it in an app template. The Native build expands the declaration from a locked local package, stages admitted HTML/CSS, and leaves the app in charge of its domain behavior. The browser does not fetch package source or run the catalog CLI.

## Where things live

- `crates/catalog-core/`: package and component metadata, lock validation, discovery, dependency resolution, and typed component contracts. Keep this independent of app I/O.
- `crates/clanker-ui/`: local CLI (`find`, `describe`, `graph`, `verify`, `lock`) and isolated composition proofs. The old `build`/`compose` commands are **not** the app integration path.
- `packages/vanilla/ui-package.json`: package identity, theme, and declared shared resources. `packages/vanilla/components/<name>/component.json` opts a component into discovery; absent metadata means it is not available. `icons.json` is the shared closed SVG geometry catalog.
- `packages/vanilla/components/<name>/`: component manifest, fragment/template, CSS, and self-authored fixtures. The app owns token overrides; don't edit package defaults to theme one app.
- `examples/`: small local package/CLI proof. `README.md` describes the architecture and its current limitations.
- `../platform/crates/xtask/src/native_ui.rs`: the **separate Native host adapter** in the local platform worktree. It checks the app lock and package digest, expands supported `<cui-… />` declarations in a private captured snapshot, and stages CSS before ordinary template/resource admission. This is not a general runtime renderer.
- `../golinks-clanker-ui-button/`: a local integration worktree. Its `ui/clanker-ui.lock.json` points to this sibling package and pins its declared bytes. Its `FRONTEND.md` explains the button experiment and rebuild caveat. These sibling paths are not portable CI provisioning.
- `../../internal-tools-toolframe/src/components/`: F# reference contracts, component manifests, fixtures, styles, and tests. **Read but do not copy F# constructors or Scriban templates into this Rust/HTML target.** Port semantics, accessibility, visual tokens, and tested behavior to a Native contract.

## How to use a component

1. Run `cargo run --locked --offline -- find <term> --lock examples/button-app/clanker-ui.lock.json` and `describe <name>`; inspect its `component.json`, fixtures, and `graph <name>` before selecting it. Search is literal, not semantic. The lock names the exact package version and declared input digest.
2. In a Native app, use a supported `<cui-button … />` or `<cui-icon … />` declaration in an app-owned template. Bind labels, typed page fields, routes, forms, and commands through the host's existing rules. A component must not create a command or own a route.
3. Keep app-specific `--cui-*` overrides in `ui/clanker-theme.css`. The host must validate and stage the package's styles and app overrides; do not paste generated HTML/CSS into the app.
4. Update the app lock explicitly when any declared package byte changes; a local package edit alone does **not** trigger an app rebuild. Check the output HTML, keyboard behavior, accessibility semantics, responsive layout, and no-JS behavior in a real host build.

## Porting one component well

1. Read Toolframe's `component.yaml`, contract, tests, styles, fixtures, and dependencies. Choose a dependency-first, bounded slice. A Toolframe `experimental` manifest is evidence of a design, not a production readiness claim.
2. Write a target-specific manifest with accurate agent `useWhen`/`avoidWhen`, assets, tokens, invariants, and fixtures. Register only working components as `ready`; unregistered directories stay invisible. Preserve the closed icon names and the difference between decorative and labeled icons.
3. Implement typed validation and escaping, deterministic rendering, and tests for valid variants/states and invalid input. Reject unknown props, invalid names, unsafe markup/URLs, malformed declarations, and inaccessible states rather than silently changing intent. Keep component CSS and fixtures beside the manifest.
4. Extend the Native host adapter only as needed for the declaration and locked asset closure. Preserve its lock/path/digest checks, staging budget, and normal template/form/resource admission. Do not introduce runtime package lookups, arbitrary JS, or a second app-operation catalog.
5. Prove the CLI and host separately, then integrate in a real app with an app-owned theme and test its rendered result. If the host cannot admit a component's lifecycle/no-JS behavior, leave it unready rather than claiming it works.

## Local checks and cautions

- In this repo: `cargo fmt --check`, `cargo test --locked --offline`, and `cargo run --locked --offline -- verify --lock examples/button-app/clanker-ui.lock.json` (refresh the example lock after declared package changes).
- In the platform checkout: use the repository's pinned Rust toolchain (`cargo +1.98.1 test --manifest-path ../platform/crates/xtask/Cargo.toml native_ui --offline` from here). The adapter is a separate repository and must be tested there.
- For GoLinks: regenerate its lock against the changed package, run its Native build through the local platform, and inspect the staged/served page. A sibling path is a local experiment, not a distributable dependency.
- Preserve pre-existing changes in sibling worktrees. A green local check does not mean the sibling-path lock is portable to CI or that the host/app changes have a reviewable PR.
