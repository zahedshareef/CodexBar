# Tauri parity shell design

## Problem

Win-CodexBar currently ships a Rust/egui tray app, while the repository also contains a Tauri/React desktop shell in `apps/desktop-tauri`. The next step is not a greenfield rewrite: it is a controlled migration where the Rust app stays shippable while the Tauri shell is brought up to parity for the core tray and window behaviors that define CodexBar on desktop.

The immediate goal of this subproject is to make the Tauri shell capable of the same desktop-surface contract as the current Rust app: startup hidden to tray, tray-opened summary surface, native tray menu, dashboard/provider popout behavior, settings with a dedicated About entry path, close-to-tray behavior, and stable placement near the tray or in the work area fallback.

## Scope

### In scope

- Implement desktop shell parity in `apps/desktop-tauri`.
- Keep the existing Rust `codexbar` crate as the backend/domain source of truth.
- Use React for visible desktop surfaces in the Tauri app.
- Match the current parity contract already proven for the Rust app:
  - startup hidden
  - tray-opened panel
  - native tray menu
  - dashboard popout
  - provider-detail popout
  - close hides to tray
  - settings opens as its own shell surface
  - about remains reachable through a dedicated shell entry path
  - placement is tray-anchored when possible and work-area-clamped otherwise
- Reuse or extend the existing Tauri proof harness so parity can be verified deterministically.

### Out of scope

- Full product cutover from Rust to Tauri in this spec.
- Reimplementing provider/auth/settings/update business logic in TypeScript.
- Broad visual redesign beyond what is needed for parity.
- Removing the current Rust app as the shipping path.

## Recommended approach

Use a dual-track migration:

1. Keep the Rust app as the shipping shell.
2. Build Tauri parity in parallel in `apps/desktop-tauri`.
3. Use the current Rust parity artifacts as the behavior contract for the Tauri shell.
4. Delay cutover decisions until the Tauri shell has proof for the same tray/window behaviors.

This approach keeps the app shippable while turning the Tauri shell into a serious migration target instead of an experimental branch.

## Architecture

### Shell ownership

- `apps/desktop-tauri/src-tauri` owns desktop shell behavior:
  - tray integration
  - surface transitions
  - window visibility/focus rules
  - positioning
  - proof-mode activation
- `apps/desktop-tauri/src` owns the visible UI surfaces in React.
- `rust/` remains the backend/domain source:
  - provider data
  - settings persistence
  - updates
  - auth/manual cookie flows
  - any shared business logic already implemented in the Rust crate

### Core rule

Tauri should own shell and rendering. The Rust crate should keep owning data and business logic.

## Key components

### 1. Surface machine

The Tauri shell already models surface state. This subproject should harden it into the parity authority for:

- `Hidden`
- tray panel
- popout/dashboard
- settings
- settings

Every tray/menu/window event should route through that state machine rather than creating independent ad hoc visibility changes.

The current Tauri shell only exposes:

- `Hidden`
- `TrayPanel`
- `PopOut`
- `Settings`

This spec keeps that shell shape unless parity work proves a fifth shell mode is necessary. In particular, **About should remain a tab inside Settings for the first Tauri parity pass**, but the shell must still support a dedicated entry path that opens Settings with the About tab selected.

### 2. Tray bridge

The tray bridge should be the single entry point for:

- primary tray activation
- secondary/native menu activation
- dashboard popout action
- provider-detail action
- settings
- about entry path
- quit

The tray bridge should emit explicit shell transitions instead of embedding UI assumptions in the tray handlers.

To avoid ambiguity around dashboard vs provider-detail behavior, the tray bridge and shell should use a typed target model for visible content. A minimal parity-safe version is:

- `TrayPanel(Summary)`
- `PopOut(Dashboard)`
- `PopOut(Provider { provider_id })`
- `Settings(General)`
- `Settings(About)`

This does **not** require a new top-level `SurfaceMode` enum for About in phase 1. It does require the shell, bridge payloads, and proof harness to carry a target payload in addition to the coarse surface mode.

### 3. Window positioner

The Tauri shell already contains a window positioner and should be extended so its behavior matches the Rust contract:

- when tray bounds are known, place the surface relative to the tray icon
- prefer sensible above/below logic based on available space
- clamp inside the current work area
- when tray bounds are unknown, use a bottom-right work-area fallback

This is the primary place where current Rust placement lessons should be transferred into Tauri.

### 4. Commands/events boundary

The backend boundary should stay narrow and explicit:

- Tauri commands fetch bootstrap/provider/settings/update data from Rust-backed state
- UI actions send typed commands back into the shell/backend boundary
- shell-state changes emit events for the React layer when needed

The goal is to avoid duplicating logic in React while keeping the UI responsive and typed.

### 5. React surfaces

The React app should expose clear surfaces rather than one monolithic window:

- tray panel
- dashboard popout
- provider-detail popout
- settings
- about tab inside settings

Each surface should map to a parity behavior, not just a route.

### 6. Proof harness

The existing Tauri proof harness should become the parity gate for this migration track. It should be able to:

- force a target surface
- read current shell state
- drive tray/menu/window transitions deterministically
- capture evidence for each parity behavior

## Data and control flow

1. A tray/menu/shortcut event enters through the Tauri shell.
2. The shell resolves both:
   - the next coarse surface mode (`Hidden`, `TrayPanel`, `PopOut`, `Settings`)
   - the next surface target payload (`summary`, `dashboard`, `provider:<id>`, `settings:<tab>`)
3. The window positioner computes placement if the next surface is visible.
4. The shell updates the webview/window state.
5. React renders the corresponding surface using Rust-backed data fetched through commands/events.

This keeps shell transitions deterministic and prevents React from becoming the de facto source of tray/window truth.

## Failure handling

- If tray bounds are unavailable, fall back to clamped work-area placement.
- If a requested shell transition cannot be completed safely, restore a reachable visible surface rather than hiding into an unusable state.
- Closing settings/about/popout should not terminate the app; the shell should return to tray-first behavior.
- Settings/about should never be the only recovery path to regain control of the app.

## Rollout

### Phase 1: tray panel and popout parity

- tray-opened panel
- native tray menu
- dashboard popout
- provider-detail popout
- placement logic
- close-to-tray behavior

### Phase 2: settings/about parity

- settings surface behavior
- about tab entry behavior through the shell
- tray-first routing while settings/about are open

### Phase 3: proof-driven readiness

- capture evidence for the same parity rows already used for the Rust app
- compare Tauri shell behavior directly against the established contract
- only then decide whether cutover work should begin

## Validation target

The Tauri shell should be considered parity-ready for this subproject only when it has proof for:

1. startup hidden
2. primary tray activation -> summary/tray panel
3. secondary tray activation -> native menu
4. dashboard popout
5. provider-detail popout
6. close hides to tray
7. placement stays on the correct work area
8. settings stays tray-first
9. about opens through the correct settings-tab shell path

## Required proof harness contract

The current Tauri proof harness only supports startup selection via `CODEXBAR_PROOF_MODE`. That is not enough for parity gating. For this subproject, the harness should grow a deterministic control/readback interface that can:

### Drive transitions

- trigger primary tray activation
- trigger secondary/native menu activation
- open dashboard popout
- open provider-detail popout for a specific provider id
- open settings on a specific tab
- open About through the settings/about shell path
- request hide/close behavior checks

### Dump shell state

- current coarse surface mode
- current target payload (`summary`, `dashboard`, `provider:<id>`, `settings:<tab>`)
- current window rect
- resolved tray anchor, if any
- resolved work-area rect / monitor bounds

### Capture evidence

- screenshot or equivalent proof artifact for visible surfaces
- native menu evidence for secondary tray activation
- geometry evidence for placement/work-area checks

For native menu parity specifically, acceptable evidence can be either:

- a visible captured native menu artifact, or
- a harness/state dump that proves the platform-native menu path was invoked and which menu items were emitted.

## Relationship to the other workstreams

This spec covers only the **Tauri parity shell** workstream.

The broader migration program remains:

1. Rust shipping hardening
2. Tauri parity shell
3. cutover contract

Rust shipping fixes may still happen in parallel, but they are not specified here except where their proven behavior defines the Tauri parity target.
