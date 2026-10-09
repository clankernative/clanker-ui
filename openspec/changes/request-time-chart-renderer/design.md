## Context and scope

Native can render changing query data, but chart geometry needs a separately
approved request-time adapter. This UI change supplies a closed single-series
line/bar renderer, not a Node service, arbitrary callback SDK or client dashboard.

## Decisions

- `lib.rs` owns typed input validation, exact protocol identity, SVG parsing and
  scene admission. Its collections/order and formatting are deterministic.
  `Engine` is the only rendering port; tests can replace it without copying
  renderer policy or bypassing validation.
- `engine.rs` embeds reviewed ECharts 6.0.0/QuickJS in a fresh context per job.
  Heap/stack limits and monotonic interruption belong to this adapter. Clock,
  randomness, UTC/English formatting and bounded timers are fixed for engine
  code. Caller JSON cannot supply code, callbacks, DOM, network or filesystem APIs.
- `main.rs` owns bounded JSONL I/O. Replies are produced by the same checked core
  used in simulations. Malformed/nonfinite/missing coordinates fail closed,
  never panic or silently produce empty geometry.
- The package owns ordinary SVG/table templates and optional bounded events.
  Paint order, clipping, transforms, stroke metrics and text semantics survive
  conversion. Raw markup and unsupported SVG are rejected.
- The app owns data/ranges/queries/commands/dialogs. Native owns contract admission,
  independent executable approval and process supervision. Build assembly,
  binding ABI 2 and runtime presentation ABI 1 stay separate.

## Verification and migration

Seeded engine-port and DOM schedules compare outcomes with independent invariants
and retain seed/step replay traces. Recorded genuine producer vectors, real engine
subprocesses, host confinement and actual browsers remain distinct gates.
Schema parity, changing values/ranges, gaps/zero/singletons, source notices and
unchanged budgets are tested; latest Mac/browser and full application DST gates
remain open. Opt-in source support stays draft/adapter-required, with no component
release or deployment. Existing app/worktree/server state is preserved.
