# Release legal inputs — bounded review

Project MIT was explicitly approved: `LICENSE`, copyright (c) 2026 Clanker Native
contributors. All four authored crates inherit workspace SPDX `MIT` metadata.
Third-party/font terms remain separate; project MIT does not assert rights over
material whose original provenance has not been established.

## Distributed closure

- `native-bundle` embeds the source-reviewed root `LICENSE` and `NOTICES.txt` at
  compile time. It stages **exactly** `legal/LICENSE` and `legal/NOTICES.txt`,
  records their paths/bytes/SHA-256 in mandatory manifest `legal` entries, and
  verifies that closed shape and bytes during bundle verification/restore.
- Legal files are outside `package/`: vanilla 0.7.0 resources, declared catalog
  digests and canonical app locks are unchanged. Legal bytes share existing
  count/file/total budgets; no security limit is raised.
- `native-release` captures these files in the archive and takes the adjacent
  target-specific `.NOTICES.txt` asset from the same validated snapshot. It
  includes the project MIT text as well as third-party texts, must accompany
  standalone binary redistribution, and has an independently reviewed digest.
  Metadata/checksum files identify it, not substitute for actual notices.
- Existing unreleased manifests lacking the mandatory legal closure fail;
  there is no legacy layout compatibility or arbitrary-resource copier.
- App authors redistributing generated package CSS/JS and catalog assets must
  retain appropriate project MIT and applicable third-party notices, including
  icon terms and font OFL. Keep notices with the app distribution. This does not
  change app-owned licenses/domain rights or require a component-aware host port.

## Reviewed license inputs

The static `NOTICES.txt` inventory covers 123 registry package/version pairs in
the union of `cargo tree --locked --offline -p clanker-ui --edges normal,build`
for `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`, including build/proc-macro
dependencies conservatively. Declared license expressions and actual cached
license/COPYRIGHT files were inspected without new tools or worker networking.
Cached MIT alternatives are used where offered; documented missing-file
fallbacks use the declared Apache option. ISC, Apache-2.0, MPL-2.0 and Unicode-3.0
texts/attributions are retained otherwise. `unicode-ident`'s additional Unicode
terms are included, not reduced to MIT.

Four cached packages lack standalone license files: `fxhash`, `mac`,
`match_token`, and `selectors`. Their upstream Cargo license declarations and
source attribution are recorded. The first three use their Apache-2.0 option
with full cached Apache text; `selectors` retains its MPL source header and full
cached MPL-2.0 text. No copyright dates or rights holders were invented.

The MPL-covered packages (`cssparser` 0.35.0, `cssparser-macros` 0.6.1,
`dtoa-short` 0.3.5, `selectors` 0.31.0) are unmodified registry sources.
Exact-version source archive URLs and source-form rights are provided in
notices. Coordinator qualification reported HTTPS HEAD 200 for all four source
archives. Distributors must preserve source availability and notices; this is
not a permanent guarantee of an upstream service.

The installed Rust 1.98.1 `COPYRIGHT-library.html` was also inspected. Its
file/dependency attributions and full license texts are retained in text form;
identical repeated license blocks are referenced and reproduced once. This is
conservative standard-library coverage, not a claim that every listed library
is linked. Release 0.1.1 also includes complete cached license texts for the
worker's locked Linux/macOS normal/build closure, rquickjs 0.14.0's family MIT
notice, its exact embedded QuickJS-NG MIT notice, Apache ECharts 6.0.0 NOTICE
and LICENSE, and embedded d3/zrender BSD notices. The worker inventory alone
was not sufficient for binary redistribution; these full terms are retained
in the same closed `legal/NOTICES.txt` and adjacent release notices. Each target producer must use this reviewed toolchain inventory or
refresh/review notices for its actual toolchain before distribution.

Geist Sans/Mono remain under SIL OFL-1.1, with the existing Vercel/basement.studio
copyright and full font license retained both in the unchanged declared package
closure and notices. Project MIT does not replace OFL terms.

## Icon provenance and review boundary

Original commit `56c583e50c8ee272b46fcc5bf60a7d61240dde21` added `icons.json`
with a one-time `import-toolframe-icons.rs` importer for Toolframe's
`icon/template.html`. That establishes imported geometry, not independent
Clanker authorship. The original Toolframe checkout is unavailable here;
coordinator comparison identified compatible Feather `external-link` geometry.

Full Feather MIT and Lucide ISC license texts (including Lucide's Feather MIT
section) were supplied by the coordinator from official GitHub license APIs and
retained exactly in notices. Both are conservative compatible/derived-geometry
attributions, not an assertion that all 100 shape origins have been established.
No SVG or catalog bytes were changed or relicensed under project MIT.

MIT approval and retained notices do not certify all source rights. A bounded
source/history/privacy and public-surface review is not a certified secret-free
or exhaustive legal audit. Target qualification, hosted acquisition and
publication remain separate evidence gates; consult the exact release page for
actual qualified assets. Dependency, toolchain or legal-input changes require
fresh explicit review, rebuild and reviewed asset hashes. Never silently change
catalog locks or replace already published assets.
