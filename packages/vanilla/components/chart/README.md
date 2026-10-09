# Experimental request-time chart

This is **experimental v1** source support. The component remains **draft** and
Native integration remains `adapter-required`; the ready-only catalog excludes
it. The generic host port, confined worker, ordinary Mac build, persisted-data
rerender and selected-range response have local evidence. Full browser/lifecycle
and no-JS acceptance remain separate gates, including the latest pointer fixes.
See [setup and scope](../../../../docs/runtime-chart-renderer.md). Existing Native
release bundles do not install or approve the separate chart worker.

## Declaration

```html
<cui-chart id="latency-week" data="{{ chart.chart }}" label="Latency by hour" />
```

Only `id`, `data`, `label`, and `enhance` are accepted. `data` must be one whole
dotted record field expression; it cannot contain calls, indexing, operators,
interpolation fragments, or raw JSON. The Native record holds the same scalar dimensions/domains/title/kind as the
worker input, but `samples` is a bounded `CollectionPage` envelope. Expansion
leaves a typed `ui_scene('echarts_chart_v1', {...})` record projection in the
template: scalar fields and `samples.items` only. Continuation cursors and
unrelated query fields are never sent to the worker. Native independently
checks literal unique record keys and every projected field against the
approved input contract. The sample app enforces Native's existing 100-item
page bound; the standalone worker contract remains bounded to 128 samples.
The locked `renderer-contract.json` is closed typed metadata, not executable
approval. Expansion and build do not execute an engine, download a library, or
produce fake chart geometry.

`enhance` is literal `true` or `false` and defaults to `false`. A false value
emits no chart browser installer. The server-rendered SVG, caption and exact
values table work without JavaScript. The table spells an explicit gap as
**Missing**, while a measured zero remains **0**. Chart samples, paths, labels
and hidden point metadata are bounded by the renderer ABI; SVG paint roles are
closed to 0–4 and dynamic values use normal escaped template expressions.

The root figure is the enhancement boundary: `[data-cui-chart]` carries
`data-cui-chart-enhance`, `data-start`, and `data-end`. The SVG uses the input
width and height as its viewBox. Bounded hidden `[data-cui-chart-point]` spans
expose only `data-key`, `data-time`, `data-value`, `data-missing`, `data-x`, and
`data-y`.

With enhancement enabled, the separate admitted installer owns Previous, Next,
Inspect, Start selection, Apply and Cancel buttons and preview SVG layers; the
fragment adds no toolbar. The app must explicitly load the staged `clanker-ui.js`
entrypoint from its admitted module (for example `import './clanker-ui.js';`);
resource staging alone does not execute an installer. Its bubbled application events are
`cui-chart:range-committed` with `{start,end}`, `cui-chart:range-cancelled`
with `{}`, and `cui-chart:point-activated` with `{key,time,value,missing}`.
Teardown also dispatches a document-scoped `cui-chart:unmounted` lifecycle
notification. Application events are data-only; the application owns URL
navigation, commands and dialogs. Pointer dragging selects a continuous time
interval from the checked plot/time domains, not a sample-snapped interval;
its right endpoint is exclusive. Keyboard sample selection still uses measured
sample boundaries. SVG drag selection does not disable copying the exact values
table. No Datastar signal or route is created here.

The reviewed visual direction inherits the existing gallery white surface, dark
ink, quiet gray axes and orange accent. Package defaults use `--cui-chart-*`
tokens with inherited `--cui-*` fallbacks; apps own all overrides. Renderer v1
uses UTC timestamps, English labels, integer domains, fixed dimensions from the
checked input, and no locale or application formatting policy.

Use when a bounded, persisted time series needs a server-rendered line or bar
chart and accessible exact values. Avoid client-owned dashboards, arbitrary
callbacks, unbounded data, and environments without approved request-time
presentation support. This package metadata is not a Native admission grant.
