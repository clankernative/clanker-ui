## Purpose

Render bounded current chart data into deterministic script-free SVG geometry for server pages, with optional browser interaction and app-owned state.

## ADDED Requirements

### Requirement: Closed server chart contract
The renderer SHALL accept only declared line/bar data, explicit domains, dimensions and accessibility text; it SHALL reject unknown properties, unsafe labels, invalid numeric data, excessive budgets and arbitrary callbacks/options.

#### Scenario: Selected range and changed records
- **WHEN** valid chart inputs contain a different bounded time window or changed persisted values
- **THEN** newly computed geometry reflects those inputs without pre-generated fixtures

#### Scenario: Missing and zero values
- **WHEN** a series contains explicit gaps or true zero values
- **THEN** gaps are not joined or converted to zero and true zero remains data

### Requirement: Deterministic isolated rendering
The engine SHALL expose no app-controlled code, network, filesystem or DOM APIs and SHALL enforce execution and output limits with stable job state and explicit UTC formatting.

#### Scenario: Same-input replay
- **WHEN** identical inputs are rendered repeatedly or interleaved with other requests
- **THEN** canonical typed scene output is identical

### Requirement: Ownership and progressive enhancement
The chart SHALL remain readable without JS; optional enhancement SHALL emit bounded semantic events while the app owns queries, routes, authorization, modal content and accepted selections.

#### Scenario: Enhancement absent or replaced
- **WHEN** JS is disabled, omitted, cancelled or the chart root is replaced
- **THEN** the SVG/data fallback remains usable and enhancement releases root-owned listeners and state

### Requirement: Honest qualification
Component completion, runtime worker conformance, host admission, browser acceptance and full Platform verification SHALL remain separately evidenced.

#### Scenario: Incomplete Native/browser gate
- **WHEN** a worker or isolated simulation passes without the real host/browser gates
- **THEN** the component is not advertised as Native-supported or fully qualified
