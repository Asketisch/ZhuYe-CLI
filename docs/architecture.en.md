# Architecture Reference

<p align="center">  <a href="architecture.md">中文</a> · <b>English</b>  </p>

> The architecture reference for ZhuYe-CLI: repository positioning and the
> engine relationship, presentation-layer module layout, the event loop and
> UiState state machine, the protocol projection boundary, and the
> cross-workspace dependency constraints. The README covers usage; this
> document is for readers who need to understand the internals or do
> secondary development. See [configuration.md](configuration.en.md) for
> configuration and [development.md](development.en.md) for the development
> and testing workflow.

## 1. Repository Positioning: Presentation Layer + Engine Submodule

ZhuYe-CLI is ZhuYe's interactive terminal client (ratatui + crossterm), aimed
at terminal-based AI agent workflows. This repository is a **single-crate
workspace**: just `crates/zhuye-cli` (lib `zhuye_cli` + thin binary
`zhuye-cli`).

The core engine (AgentThread / StateGraph / tools / protocol) is provided by
the `zhuye-agent/` **git submodule**; this crate only handles the
presentation layer: event loop, layout, rendering, and protocol-event
adaptation.

```
zhuye-agent (core engine, submodule provider)
  └─ crates/{protocol,abilities,resources,orchestration,integrations,runtime}
  └─ zhuye-core (AgentThread + StateGraph)

ZhuYe-CLI (this repo)  ──submodule──► zhuye-agent
  └─ crates/zhuye-cli (ratatui UI + event loop)
  └─ zhuye-cli binary (terminal entry point)
```

Launching without a subcommand enters the TUI; subcommands such as `session`
/ `traces` take the headless path and exit when done.

## 2. Module Layout (dependencies flow top-down)

| Module | Responsibility |
|------|------|
| `src/bin/main.rs` | Thin binary: clap routing. No subcommand → TUI; `session`/`traces`/`workspace`/`discussion`/`security` → headless, exit when done |
| `bootstrap.rs` | Loads config, builds the ModelRegistry / provider / ToolRegistry / AgentThread, enters the TUI |
| `tui/` + `events.rs` + `adapter.rs` | The **local** event loop, layout, the `UiEvent`/`UiState` state machine, and the protocol-event adaptation boundary |
| `tui_core/` | Self-contained presentation primitives (custom terminal backend, streaming history insertion, CJK wrapping, composer, etc.) |
| `history_render/` (root `history_render.rs` + directory) | History line rendering, statusline, the slash legacy channel |
| `cli/` (session / traces / discussion / security / workspace; models.rs not wired) | Headless subcommand implementations (calling zhuye-rollout / zhuye-telemetry / zhuye-discussion / zhuye-integration) |
| `views/`, `term/`, `utils/`, `crate_stubs/`, `compat/` | Former src-root flat files grouped by function: human-readable/overlay views, terminal infrastructure, helpers, `zhuye_*` engine stubs, compatibility shims (old paths kept compatible via lib.rs re-exports) |

Dependencies are strictly one-way: `tui` → `tui_core` / `adapter` / `events`
/ `history_render` / `terminal`; no module may reference `crate::tui` back.

## 3. Event Loop and the UiState State Machine

The TUI event loop is **locally single-threaded**:

- `tui/mod.rs::run_async` is the event-loop entry; `handle_event` digests
  terminal events, `handle_key` does key routing, and `draw` /
  `draw_fullscreen` render;
- `events.rs` defines `UiState` (shared mutable state), `LoopState`, and the
  pending-approval states (`PendingApproval` / `PlanApprovalState`);
- `adapter.rs` defines `UiEvent` / `UiHistoryItem` and the state reducer
  `apply_event` — engine events are reduced here into UI state changes plus
  the lines that need flushing into the scrollback;
- background tasks (`background_tasks`) are spawned / aborted on the main
  loop; abort timing is bound to turn-interruption semantics.

**bootstrap must use a `current_thread` runtime**: the forked crossterm's
`event::poll/read` bind an intra-thread lock, and under a multi-threaded
runtime poll is constantly false due to lock contention (see the comments in
`bootstrap.rs`).

## 4. Protocol Projection: the Submission / EventMsg Consumption Boundary

Only a single protocol layer sits between the TUI and the engine:

- User input is validated by the composer and submitted to `AgentThread` as a
  `Submission`;
- engine-side events (`EventMsg`) are uniformly converted into UI events in
  `tui/conversion.rs` — this is the **single** conversion boundary; conversion
  logic is not allowed to scatter into `handle_key` / `draw`;
- the assistant message persisted for a conversation turn is the reply of the
  **last round**: one user submission triggers multiple model calls inside
  the engine, and streaming intermediate states are not persisted.

Evolution of the wire protocol (Submission / Op / EventMsg) belongs to the
engine repository; this repository only does consumer-side adaptation.

## 5. Cross-Workspace Dependency Constraints

- **Cargo does not support nested workspaces**: the core crates are not
  listed in this repository's `workspace.members`; they are consumed via
  `path` dependencies only. The core crates inherit many dependencies with
  `workspace = true`, which resolve against **this repository's** root
  `[workspace.dependencies]` — that section must therefore declare every
  zhuye-* and external dependency the core references; when upgrading the
  submodule or adding core crate dependencies, mirror the declarations.
- **ratatui/crossterm are the nornagon forks** (pinned revs via
  `[patch.crates-io]` in the root `Cargo.toml`), relying on unstable
  features absent from the official releases (`scrolling-regions`,
  `unstable-backend-writer`, `rendered-line-info`, `WidgetRef`,
  `event-stream`, `bracketed-paste`); do not switch back to the crates.io
  official versions.
- `tui_core/` is a self-contained set of presentation primitives owned by
  this repository; third-party origins and licenses are listed in
  [NOTICE](../NOTICE).

For the engine-side layered architecture (the six-layer crate organization,
StateGraph transitions) see the submodule's
[zhuye-agent/docs/architecture.en.md](../zhuye-agent/docs/architecture.en.md).

## 6. Build Profile Strategy

- `[profile.release]`: thin LTO + `codegen-units = 1` + `strip = "symbols"`,
  for release distribution.
- `[profile.release-fast]`: no LTO, `opt-level = 2`, `incremental = true`, for
  fast local iteration (`scripts/build.sh --fast`); artifacts land in
  `target/release-fast/`, and cargo never reclaims that incremental
  directory — `make gc` does.
- `[profile.dev]`: `incremental` is deliberately disabled — cargo does not
  garbage-collect `target/<profile>/incremental/`, so leaving it on makes disk
  usage grow without bound; the cost is a slightly slower first build.
- `[profile.test]`: `debug = 1` (line tables only) + `codegen-units = 256`, to
  speed up compiling and linking test artifacts.

For the full cleanup semantics (including cross-compilation roots
`target/<triple>/<profile>/`) see the `gc` / `gc-deep` targets in the root
`Makefile`.

## 7. Further Reading

- [AGENTS.md](../AGENTS.md) —— AI-assistant working contract: module layout, key constraints, development checklist (in Chinese)
- [configuration.md](configuration.en.md) —— data directory, CLI flags, environment variables
- [development.md](development.en.md) —— builds, test layering, verification matrix
- [zhuye-agent/docs/architecture.en.md](../zhuye-agent/docs/architecture.en.md) —— engine-side architecture reference (external link, submodule)
