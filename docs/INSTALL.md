# First install — early access

CLI **0.1.0**, vanilla **0.7.0**, assembly protocol **2**, binding ABI **2**.
Consult the [exact-version release page](https://github.com/clankernative/clanker-ui/releases/tag/v0.1.0)
for available assets, actually qualified targets, reviewed expected hashes and
producer/source provenance. These instructions alone do not establish hosted
availability or target qualification: if the exact release/target or reviewed
identities are absent, stop. No source clone or Cargo is required.

## 1. Review identity before downloading or executing

The target names are **`macos-aarch64`** (Apple Silicon, primary Native
qualification target) and **`linux-x86_64`** (CLI only, subject to producer tests).
A Linux CLI test does **not** establish full Linux Native builder support.
macOS host/consumer qualification is separate; Windows is unsupported.

For exact tag `v0.1.0`, review the producer/source revision and the target-specific
`clanker-ui-release-0.1.0-<target>.json`. It connects:

- `clanker-ui-native-0.1.0-<target>.tar.gz` and its `.sha256`;
- standalone `clanker-ui-0.1.0-<target>` and its `.sha256`, identical to the
  archive's `bin/clanker-ui`;
- adjacent `clanker-ui-0.1.0-<target>.NOTICES.txt` and its `.sha256`, identical
  to the archive's `legal/NOTICES.txt` and containing project MIT plus third-party
  terms; redistribute it with the standalone binary;
- tool/catalog identities and package digest, source revision, legal input
  identities, and support scope.

Obtain the **expected bootstrap and archive SHA-256 values from the published,
reviewed identity/provenance decision**. A checksum downloaded beside an asset,
an unsigned manifest, or a source-revision claim alone does not authenticate its
producer. Review provenance and explicitly approve the bootstrap's first
execution; successful download/hash verification is not execution authority.

## 2. Download and check the standalone bootstrap

Use operator-installed `curl` supporting streaming `--max-filesize` enforcement
and `shasum` (macOS) or `sha256sum` (Linux). Do not use an older curl that limits
only declared Content-Length; the first download must remain bounded.
Run these commands individually from the existing parent directory of your app;
keep downloads and the fresh install **project-adjacent, outside the app's
served `ui/` directory**. Do not reuse an existing installation.

```sh
umask 077
VERSION=0.1.0
TARGET=macos-aarch64
# On qualified Linux CLI hosts only, use TARGET=linux-x86_64 instead.
BOOTSTRAP="clanker-ui-$VERSION-$TARGET"
DOWNLOAD_DIR="$(mktemp -d "$PWD/.clanker-download.XXXXXX")"
INSTALL_DIR="$PWD/.clanker-ui-$VERSION-$TARGET"
curl --disable --fail --silent --show-error --location --proto '=https' \
  --proto-redir '=https' --max-redirs 5 --connect-timeout 15 --max-time 120 \
  --max-filesize 67108864 --output "$DOWNLOAD_DIR/$BOOTSTRAP" \
  --url "https://github.com/clankernative/clanker-ui/releases/download/v$VERSION/$BOOTSTRAP"
```

Also download the reviewed target-specific notices (no execution):

```sh
curl --disable --fail --silent --show-error --location --proto '=https' \
  --proto-redir '=https' --max-redirs 5 --connect-timeout 15 --max-time 120 \
  --max-filesize 1048576 --output "$DOWNLOAD_DIR/$BOOTSTRAP.NOTICES.txt" \
  --url "https://github.com/clankernative/clanker-ui/releases/download/v$VERSION/$BOOTSTRAP.NOTICES.txt"
```

Set `EXPECTED_NOTICES_SHA256` to the separately reviewed **64 lowercase hex
digits** and verify on macOS (`sha256sum -c -` on Linux):

```sh
printf '%s  %s\n' "$EXPECTED_NOTICES_SHA256" "$DOWNLOAD_DIR/$BOOTSTRAP.NOTICES.txt" | shasum -a 256 -c -
```

Read and retain the notices with the bootstrap. Set `EXPECTED_BOOTSTRAP_SHA256`
to the reviewed **64 lowercase hex digits**
(no `sha256:` prefix), then check on macOS:

```sh
printf '%s  %s\n' "$EXPECTED_BOOTSTRAP_SHA256" "$DOWNLOAD_DIR/$BOOTSTRAP" | shasum -a 256 -c -
```

On Linux use the same `printf` with `sha256sum -c -` instead. Stop on any error,
mismatch, wrong target, or incomplete provenance. **Pause here for explicit
operator approval of first execution.** Only after approval:

```sh
chmod 755 "$DOWNLOAD_DIR/$BOOTSTRAP"
```

## 3. Restore the reviewed archive to a fresh install

Set `EXPECTED_ARCHIVE_SHA256` to the separately reviewed archive's **64 lowercase
hex digits**. Invoke the existing exact-version GitHub acquisition adapter:

```sh
"$DOWNLOAD_DIR/$BOOTSTRAP" restore-native-bundle \
  --github-repository clankernative/clanker-ui --version "$VERSION" \
  --expected-sha256 "sha256:$EXPECTED_ARCHIVE_SHA256" --output "$INSTALL_DIR"
```

Restore checks the archive hash **before decompression**, applies existing
hostile-path/type/size budgets (including the unchanged 64 MiB executable guard),
verifies the complete package, and publishes atomically without clobbering.
It never executes the installed compiler. The output parent must already exist;
failed restore leaves no partial installation. There is no install script,
`latest` resolver, automatic upgrade, or build-time download. Local reviewed
archives also work with `--archive FILE` and the same `--expected-sha256`.

The bootstrap and installed CLI must have the same reviewed hash; likewise
adjacent and installed notices:

```sh
shasum -a 256 "$DOWNLOAD_DIR/$BOOTSTRAP" "$INSTALL_DIR/bin/clanker-ui"
shasum -a 256 "$DOWNLOAD_DIR/$BOOTSTRAP.NOTICES.txt" "$INSTALL_DIR/legal/NOTICES.txt"
```

On Linux substitute `sha256sum`. Keep the installed `manifest.json`,
`provider-pin.json`, mandatory `legal/` closure, and complete `package/` closure
together when relocating. Missing/changed legal bytes fail verification; old
unreleased bundles without legal entries are not a supported layout.

## 4. Admit inputs separately

Installation is not host activation. Review/approve the installed executable
and select its relocatable `provider-pin.json` through the host's separately
approved configuration (local Platform override: `DAY2_UI_PROVIDER_PIN_JSON`).
The app's canonical `ui/ui.lock.json` is **not** executable authority. Point the
host at the installed package; the Native builder privately stages it at
`../../packages/clanker-vanilla` relative to the staged app lock. Use existing
lock/native-lock commands with explicit `--update` only when intentionally
changing declared package bytes; never silently refresh a pin or lock.

No arbitrary app networking or app build downloads are allowed. The catalog has
**54 component-complete contracts / 45 Native-supported / 9 adapter-required**;
host admission and browser/consumer proof remain independent gates. Retain old
installs until an operator approves removal. Project MIT is approved; read
`legal/LICENSE` and the separate third-party/font terms in `legal/NOTICES.txt`.
When redistributing generated package CSS/JS or catalog assets in an app, retain
the project MIT notice and applicable third-party notices, including icon terms
and OFL for fonts. Keep appropriate copies with the app distribution; app-owned
code, content, theme and domain rights remain the app author's responsibility.
Installation is not a legal/provenance trust grant.
