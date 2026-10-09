## Why

Charts must use changing request-time data, not SVG fixtures. A component-owned, bounded ECharts renderer can supply checked geometry to Native without Node or chart-specific rules in Platform.

## What Changes

- Introduce an independent pinned chart worker using reviewed ECharts 6.0.0 and an embedded single-threaded JS engine, with memory/stack/interrupt limits and no DOM/network/file APIs exposed to JavaScript.
- Define closed line/bar input and SVG-scene output contracts, stable namespaces and explicit UTC formatting; forbid arbitrary ECharts options and callbacks.
- Supply package-owned SVG templates/styles and optional interaction ports; keep routes, queries and accepted view state app-owned.
- Add deterministic/reference and real-worker conformance tests; do not mark new component Native-supported before actual host/browser qualification.

## Capabilities

### New Capabilities
- `server-chart-renderer`: deterministic bounded server chart geometry for authorized current data.

### Modified Capabilities
None. The assembly executable remains build-time-only and does not become a request-time compiler.

## Impact

A new isolated worker crate and non-executable typed contract/scene assets. New JS-engine dependencies are explicitly restored during development/setup, never by Native compilation. Existing package defaults and existing apps remain untouched until explicit new-contract adoption. No component release or deployment.
