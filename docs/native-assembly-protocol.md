# Native UI assembly protocol

Native UI assembly is an optional build-time boundary. A provider owns component
expansion. Native owns capture, admission, rendering, resources, routes, forms,
and transport. Providers are not invoked while serving a page. No provider code
is loaded into the host renderer.

## Versioned boundary

Protocol version 2 targets `minijinja-2.12.0` with binding ABI 2. ABI 1 bundles and
component-specific `cui_*` callbacks are not supported by this protocol. Assembly is
optional preprocessing: the producer returns ordinary templates and resources; Native
owns their normal admission and serving.

An app opts in with `ui/ui.lock.json`:

```json
{
  "schemaVersion": 1,
  "provider": "clanker-ui.native",
  "package": {
    "name": "@clanker/vanilla",
    "version": "0.7.0",
    "path": "../packages/vanilla",
    "digest": "sha256:<locked-input-manifest-digest>",
    "inputs": [{"path": "ui-package.json", "bytes": 123, "digest": "sha256:<file-digest>"}]
  }
}
```

Inputs are the complete declared package input closure, including non-executable
contracts. Paths are unique safe relative paths with no case-fold collisions: at most 512
ASCII bytes, eight segments, and only letters, digits, `-`, `_`, and `.` within
nonempty segments (never `.` or `..`). The existing sibling-root, symlink, file
and total-byte limits remain unchanged. Sort by path and hash the UTF-8
concatenation of `path + NUL + decimal(bytes) + NUL + digest + LF` for each input,
with SHA-256. `digest` includes its `sha256:` prefix. This manifest digest binds
paths, sizes, and byte digests without asking Native to interpret a provider's
package metadata. Native checks every captured byte. Files are not browser
resources merely because they are locked inputs.

The operator separately trusts an executable pin with `schemaVersion: 1`,
`provider`, `assemblyProtocol: 2`, `bindingAbi: 2`, and a `targets` map of host
targets to `{executable, digest}`. Protocol 2 requires an updated operator pin;
the app package lock schema and contents do not change. An app lock cannot authorize execution. The
local explicit override is `DAY2_UI_PROVIDER_PIN_JSON`; no portable executable
release is implied. Execution trusts that local executable: private snapshots,
cleared environment variables, and time/output limits are not a hostile-code
sandbox. Native independently admits its returned UI bytes.

Native invokes the pinned executable as `assemble --request REQUEST_FILE`.
The private request has this shape:

```json
{
  "schemaVersion": 1,
  "assemblyProtocol": 2,
  "provider": "clanker-ui.native",
  "target": {"bindingAbi": 2, "templateEngine": "minijinja-2.12.0"},
  "package": {"name": "@clanker/vanilla", "version": "0.7.0", "path": "/private/package", "digest": "sha256:<manifest-digest>", "inputs": []},
  "ui": "/private/ui"
}
```

Absolute paths refer to Native's captured private inputs, never mutable app
sources. The response is the CLI envelope `{schemaVersion: 1, ok, command:
"assemble", data, diagnostics}`. Successful data contains only `schemaVersion: 1`,
`runtimeAbi: 2`, `templateEngine`, `packageDigest`, `templates`, `resources`,
`inputs`, and `consumedInputs`. Producer-selected modules, including the generated
`ui/ui-package.js` bootstrap when needed, are ordinary `module` resources with
relative imports. That conventional module imports the producer's selected
`ui/clanker-ui.js` entrypoint; no entrypoint or component binding manifest crosses the boundary.
Generic checked value helpers remain unchanged. The producer validates collisions,
resource closure, source preservation, and input/output byte budgets before returning
or writing output. Native performs its ordinary independent template/resource admission.
Output paths must not collide by case or prefix. A provider cannot replace or consume
`ui/ui.lock.json`. Diagnostic component/attribute labels from standalone expansion are
opaque and are not part of the assembly response.

## Host binding capabilities

The target exposes a closed set of generic value checks, not component renderers.
Policies, flags, grapheme bounds, and integer bounds must be admitted literals.
`ui_number` may also use checked numeric fields as bounds. Field operands must
resolve against checked app types, including checked loop variables.

- `ui_text(value, policy, minimum_graphemes, maximum_graphemes)`: stringify and
  validate text. Policies are `nonblank` (no controls), `plain` (blank allowed,
  no controls), and `multiline` (nonblank; only tab/CR/LF controls allowed).
  Nonnegative grapheme bounds apply; maximum zero means no extra grapheme limit.
- `ui_key(value)`: preserve a nonempty ASCII key, at most 128 bytes, using only
  letters, digits, underscore, and hyphen.
- `ui_integer(value, minimum, maximum)`: accept only integral number values within
  inclusive literal bounds and return their decimal text. No float coercion.
- `ui_number(value, minimum, maximum, exclusive_minimum)`: validate finite numeric
  values or numeric literal strings and return canonical decimal text. Bounds
  may be checked numeric operands; `none` omits a bound. The exclusive flag is a
  literal boolean. Mixed integer/real comparisons preserve integer precision.
- `ui_compare(left, right)`: precisely compare finite numeric operands, returning
  -1, 0, or 1. Component state/class selection remains provider-authored ordinary
  template branching.
- `ui_image(value)`: admit a credential-free HTTPS image URL and return a
  host-owned image-source witness. URL length is at most 4096 bytes; surrounding
  whitespace, controls, backslashes, and wildcard hosts are rejected. Native
  retains image provenance and CSP enforcement; this does not fetch an image.

Providers emit their own literal CSS mappings and ordinary template branches.
The host does not know button variants, avatar components, or component CSS.

Generic HTML attributes `data-ui-selected-flag`, `data-ui-checked-flag`, and
`data-ui-hidden-flag` materialize only closed boolean attributes on supported
HTML targets. `data-ui-choice-set` marks a constrained single-select group;
`data-ui-choice-value` declares its requested selection, and
`data-ui-placeholder` marks a placeholder option. Generic navigation assertions
use `data-ui-navigation="true"` with literal
`data-ui-navigation-minimum-items`, `data-ui-navigation-current-last`, and
`data-ui-navigation-ancestor-links` constraints. Producer-specific component markers have no host
semantics. Command/form authority remains independently checked by Native.

## Optional request-time presentation metadata

Experimental charts select the locked `components/chart/renderer-contract.json`
as an ordinary non-browser `metadata` resource at `ui/presentation.json`. There
is no assembly wire-field or binding ABI change. Native captures these typed
contracts for `ui_scene` admission; it does not execute ECharts during assembly.
An independently approved runtime worker consumes current authorized data later.
Its presentation ABI 1 and `DAY2_PRESENTATION_PIN_JSON` approval are separate from
the assembly provider and its pin. See [runtime chart setup](runtime-chart-renderer.md).

## Evidence

A simulator implements the assembly port, not component expansion. Protocol
fixtures must come from the real producer and remain clearly labeled. Simulator
success does not prove a real CLI build. Native independently rejects malformed
bundles, incompatible versions, unsafe paths/resources, forged choices, and
invalid runtime values. A missing UI lock leaves the ordinary build path intact.
