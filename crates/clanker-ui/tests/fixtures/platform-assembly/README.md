# Provider-to-host assembly contract fixture

This corpus is owned by the Clanker UI producer. `response.json` is the exact
JSON response from `clanker-ui assemble --request` for the adjacent immutable
UI and package sources. The producer-side default test runs the built CLI
against these sources and requires full JSON equality with that recording:

```sh
cargo test --locked --offline -p clanker-ui --test platform_assembly_cli
```

Refresh the golden only after an intentional, reviewed contract change: build
the producer CLI, construct a protocol-2 request with absolute `ui` and package
paths and metadata from `app/ui/ui.lock.json`, run
`clanker-ui assemble --request REQUEST_FILE`, and review the entire source and
response diff. Do not refresh merely to make a failed conformance test pass.
The test writes its request under a temporary directory and does not modify the
fixture sources.

## Separate real Native host smoke

From the Clanker UI checkout, after the operator has explicitly reviewed and
approved the source revision, package tree, and built executable, make a fresh
private directory, place the approved executable there, and create its pin:

```sh
PRIVATE=$(mktemp -d)
cargo +1.98.1 build --locked --offline --release --bin clanker-ui
cp target/release/clanker-ui "$PRIVATE/clanker-ui"
"$PRIVATE/clanker-ui" native-pin --output "$PRIVATE/operator-pin.json"
```

From the Clanker UI checkout, run the ignored generic external-provider smoke
with `PLATFORM` set to the absolute Platform checkout path. Both environment
values resolve to absolute paths:

```sh
DAY2_UI_TEST_FIXTURE="$PWD/crates/clanker-ui/tests/fixtures/platform-assembly" \
DAY2_UI_TEST_PROVIDER_PIN_JSON=/absolute/path/to/operator-pin.json \
cargo test --manifest-path "$PLATFORM/crates/xtask/Cargo.toml" --locked --offline \
  ui_assembly::external_tests::external_provider_matches_expected_outputs -- --ignored
```

This test invokes the real pinned CLI and checks its bundle receipt and staged
templates/resources against the golden. It does not prove Roc app/type/form/browser admission or a full
isolated build; those remain consumer-side checks. The fixture is not a
published package or release. No pin is downloaded or selected automatically,
and old absolute pins must not be reused.
