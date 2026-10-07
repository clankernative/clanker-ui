# Release legal inputs — bounded review

Project MIT was explicitly approved: `LICENSE`, copyright (c) 2026 Clanker Native
contributors. All three authored crates inherit workspace SPDX `MIT` metadata.
This does not relicense third-party code, fonts, or any material whose rights
still require coordinator review.

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

## Inputs reviewed without network or installed tooling

The static `NOTICES.txt` inventory covers 123 registry package/version pairs in
the union of `cargo tree --locked --offline -p clanker-ui --edges normal,build`
for `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`, including build/proc-macro
dependencies conservatively. Declared license expressions and actual cached
license/COPYRIGHT files were inspected. MIT alternatives are used where offered;
ISC, Apache-2.0, MPL-2.0 and Unicode-3.0 texts/attributions are retained otherwise.
`unicode-ident`'s additional Unicode terms are included, not reduced to MIT.

Four cached packages lack standalone license files: `fxhash`, `mac`,
`match_token`, and `selectors`. Their upstream Cargo license declarations and
source attribution are recorded. The first three use their Apache-2.0 option
with full cached Apache text; `selectors` retains its MPL source header and full
cached MPL-2.0 text. No copyright dates or rights holders were invented.

The MPL-covered packages (`cssparser`, `cssparser-macros`, `dtoa-short`,
`selectors`) are unmodified registry sources. Exact-version source archive URLs
and source-form rights are provided in notices. Before distribution, the
coordinator must confirm continued availability of those sources and any
additional required upstream notices; this review made no hosted requests.

The installed Rust 1.98.1 `COPYRIGHT-library.html` was also inspected. Its
file/dependency attributions and full license texts are retained in text form;
identical repeated license blocks are referenced and reproduced once. This is
conservative standard-library coverage, not a claim that every listed library
is linked. Each target producer must use this reviewed toolchain inventory or
refresh/review notices for its actual toolchain before publication.

Geist Sans/Mono remain under SIL OFL-1.1, with the existing Vercel/basement.studio
copyright and full font license retained both in the unchanged declared package
closure and notices. Project MIT does not replace OFL terms.

## Coordinator gates still open

MIT approval resolves project-license selection, not all source rights.
Coordinator-owned source/history/privacy and public-surface review, including
catalog icon geometry and Toolframe-derived design/source lineage, must confirm
redistribution rights and any attribution not established by this bounded
packaging review. In particular, original commit
`56c583e50c8ee272b46fcc5bf60a7d61240dde21` added `icons.json` with a one-time
`import-toolframe-icons.rs` importer for Toolframe's `icon/template.html`.
That establishes imported geometry, not independent authorship. The commit's
docs contain no Feather/Lucide attribution, and the original Toolframe source
is unavailable in this worker environment. Resolve its upstream license and
retain the appropriate actual notice before distribution; do not infer one
from geometric similarity or relicense icons as project MIT. No rights were
inferred from an absent license. Dependency,
toolchain or legal-input updates require fresh explicit review, rebuild and new
reviewed asset hashes; they must not silently modify catalog locks or replace
published assets. Target qualification, hosted acquisition and publication are
also separate gates. This repository work only prepares local candidates.
