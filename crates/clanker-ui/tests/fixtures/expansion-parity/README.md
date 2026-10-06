# Expansion parity corpus

These are source-only copies of the self-authored gallery, GoLinks experiment, and Studio isolated gallery UI. They contain symbolic template expressions, not user database records. Unused source JavaScript, locks, and assets were excluded.

Expected HTML, CSS and property metadata originated from the frozen pre-migration Native renderer (`native_ui.rs` SHA-256 `f2d40391320fd13acb1e89f9b6d9c1beda6dccdaa29e2ec05a2cbbded4e44b45`) using the reviewed 31-ready-component package. Expected WOFF2 resources are checked by their original byte digests rather than duplicated in each expected-output tree; the captured package retains the original font bytes. `expansion_parity.rs` checks 28 managed outputs across the three consumers plus five outputs from a seven-static-port fixture, covering all 38 supported roots. The additional seven contracts are selected in a private copy of the frozen package. Their current readiness is established separately by fresh Native/browser checks, not by this corpus.

For binding ABI 2, expected symbolic HTML was explicitly rebased: `cui_*` callbacks
became generic `ui_*` checks, component CSS mappings became provider-authored
branches, and generic host flags/assertions replaced provider-specific markers.
Resource bytes and source coverage remain frozen. These reviewed changes break
symbolic ABI-1 byte parity; they do not establish fresh Native/browser evidence.

`package/` preserves the frozen locked package inputs, plus the seven static contracts' fixture assets. Tests use this captured package rather than the changing live package, so intentional token/readiness additions cannot silently redefine the migration baseline. The current package must pass separate readiness tests.

Do not regenerate expected outputs with the new renderer merely to make a failure disappear. Investigate every difference; intentional changes require reviewed expected-output updates. This corpus proves expansion compatibility, not Roc-context admission, interactions, browser readiness, or portable CLI distribution.
