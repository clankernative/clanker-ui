# Generic UI binding ABI v2

The Native host's independent generic value guards are the semantic baseline.
`clanker-ui-runtime` implements those semantics for package-side testing and
script-free rendering. Platform must not depend on the component runtime crate.
This is a value ABI, not proof of Platform admission or a full Platform gate.

## Public API and template functions

All pure functions return `anyhow::Result`; `install(&mut Environment)` registers
only the six `ui_*` capabilities below, converting guard failures into MiniJinja
`InvalidOperation` errors. No legacy generic aliases are retained.

| Function | Rust arguments | Successful result |
| --- | --- | --- |
| `ui_text` | `&Value, &str, u64, u64` | `String` |
| `ui_key` | `&Value` | `String` |
| `ui_integer` | `&Value, &str, &str` | `String` |
| `ui_number` | `&Value, Option<&str>, Option<&str>, bool` | `String` |
| `ui_compare` | `&Value, &Value` | `i64` (`-1`, `0`, `1`) |
| `ui_image` | `&Value` | `ImageSource` (opaque template object) |

Integer bounds and optional number bounds are decimal **strings**, not i64 or
floating-point limits. In templates use quoted bounds and `none` for absent
number bounds. The installer accepts string-convertible bounds as the host does;
applications should generate quoted bounds to preserve precision.

- Text renders strings verbatim and other values through MiniJinja's display.
  Policies are `nonblank`, `plain`, `multiline`. Only multiline allows tab, CR,
  LF controls; nonplain policies require nonblank text. Bounds count Unicode
  graphemes, maximum zero is unbounded, and u64 bounds have no artificial
  one-million cap. Text is not HTML-safe markup; normal host escaping applies.
- Keys use the same rendering, 1–128 ASCII bytes, only alphanumerics, `_`, `-`.
  Colon is rejected, independently of component-specific token grammar.
- Integers must be integer-typed numeric Values (including i128/u128). Numeric
  strings and integral f64 Values are rejected. Both decimal bounds must be
  mathematically integral. Results preserve exact signed values above 2^53.
- Numbers accept numeric Values or bounded numeric strings. Minimum can be
  exclusive; maximum is inclusive. Comparison uses exact decimal strings and
  exact finite IEEE-754 values, not shortest float display or rounded integers.
- Compare accepts **numeric Values only**, rejecting even valid numeric strings.
  Integer/f64 ordering is exact in both directions; signed zero equals zero.
- Images require credential-free HTTPS with a host, at most 4096 bytes, no
  whitespace, controls, backslashes, wildcard host, or host above 253 bytes.
  Successful `ImageSource.0` preserves the input spelling; it does not normalize
  URL case, default ports, or escapes. Asset-path provenance stays host-owned.

## Bounded numeric representation

Decimal input is at most 256 bytes with exponent magnitude at most 10,000.
The representation stores sign, decimal digits, and scale; ordering compares
sign, magnitude and zero-padded digits without expanding exponents. Canonical
output removes redundant leading/trailing zeroes, normalizes negative zero,
and expands exponent notation (bounded to roughly 10,256 output bytes).
Nonfinite and malformed numbers are rejected.

Finite f64 ordering expands its mantissa and binary exponent exactly using
small decimal multiplications, at most 1074 iterations. Output text instead
canonicalizes Rust's shortest-roundtrip float display (internal parse limit
1200 bytes). Consequently f64 `0.1` renders `0.1` but is **greater than** the
exact decimal bound `"0.1"`; no host semantics are changed to conceal that.
This is deliberately bounded, not arbitrary-precision arithmetic or a new
number dependency. Existing component-specific helpers are not this ABI.

## Shared vectors and worker guidance

`tests/protocol/ui-binding-abi-v2.json` is the versioned portable oracle. Copy
its bytes unchanged to the independent Platform test harness; never import the
runtime implementation into Platform to make its conformance tests pass.

```json
{
  "abi": "ui-binding",
  "version": 2,
  "cases": [{
    "id": "compare-signed-large-negative",
    "helper": "ui_compare",
    "args": [
      {"type": "i64", "value": "-9007199254740993"},
      {"type": "f64", "value": "-9007199254740992"}
    ],
    "expect": {"ok": -1}
  }]
}
```

Argument tags are `string`, `i64`, `u64`, `i128`, `u128`, `f64`, `f64_bits`,
`bool`, `none`. Payloads are strings so JSON parsers cannot round integers.
`f64_bits` is exactly 16 hexadecimal digits representing IEEE bits, including
subnormals and nonfinite rejection cases. Bool payload is `"true"`/`"false"`;
`none` omits payload and represents an absent bound (or a null test Value).
Bounds/policies use `string`; text counts use `u64`.

Expect has exactly one field: `ok` (string or signed comparison integer), or
`error` (the exact host guard code). Image success compares the inner preserved
source, never the object's template display. Case IDs are unique; runners reject
unknown fields/tags/helpers. Corpus budget: 128 KiB and 256 cases. Tests also
exercise the registered functions and independently derived binary-fraction
bounds; running vectors only does not prove installer coercion parity.

For ABI work, update this guide and the shared vector file, not the root agent
instructions or host wiring. Use Rust 1.98.1 and, pending the release profile
worker's fix, `CARGO_PROFILE_DEV_DEBUG=0`. Check scoped runtime tests first, then
workspace tests and formatting. Keep host admission, browser proof, portable
release pins, and Platform gate claims separate from these pure tests.
