# Rust tray parity hardening design

## Problem

The current Rust shipping app is still the real desktop surface for Win-CodexBar, but it is not yet solid enough as the official Windows CodexBar:

1. tray interaction still needs to be verified against real mac CodexBar behavior
2. open positioning must stay stable and tray-adjacent on Windows
3. branding and logo usage still contain placeholder or drifted elements instead of consistently using the official CodexBar assets

At the same time, the repo already contains a Tauri + React shell that will eventually replace the egui surface. The immediate goal is **not** to switch to Tauri now. The immediate goal is to harden the current Rust app so the product works correctly while we lock the tray contract against the live mac app and harvest official assets without expanding into the shell migration.

## Current context

- The active shipping implementation is the Rust tray app in `rust/`.
- The live mac CodexBar reference is available on `mac@imac-ca-mac` at `/Applications/CodexBar.app`.
- Real parity captures are available on the mac host under `~/codexbar-share/`.
- Windows behavior must be verified in the Windows VM, not by code inspection alone.
- A Tauri + React shell already exists in `apps/desktop-tauri/`, but it still contains placeholder branding and is not the shipping desktop entry point yet.
- The current canonical provider icon location is already `rust/assets/icons/`.

## Scope

- Keep the current Rust app as the shipping surface for this subproject.
- Use the live mac app as the source of truth for:
  - tray interaction behavior
  - panel/open positioning expectations
  - official app/logo/branding assets
- Harden the current Rust tray app so it behaves like the Windows CodexBar should.
- Harvest official mac assets into the existing Rust asset pipeline for the current shipping app.

## Out of scope

- Switching the shipping desktop entry point from egui to Tauri.
- Rewriting provider/runtime logic.
- Doing a full React/Tauri parity pass.
- Broad visual redesign beyond what is needed to eliminate obvious parity drift in the current shipping app.

## Options considered

### Option 1: Harden the current Rust app first, harvest assets in parallel

Use the current Rust tray app as the shipping surface, fix behavior and positioning against the mac reference, and pull official assets from the mac app into the existing Rust asset pipeline.

**Pros**
- Fastest path to a working official Windows app
- Uses the mac app as real parity truth instead of guesswork
- De-risks the later Tauri migration

**Cons**
- Requires discipline to avoid drifting into the full shell rewrite too early

### Option 2: Make the Tauri shell parity-ready first

Prioritize the Tauri shell visually and structurally before the current Rust app is fully hardened.

**Pros**
- Moves the future shell forward sooner

**Cons**
- Risks polishing the wrong behavior model
- Delays stabilization of the app users actually run today

### Option 3: Start both tracks equally

Push current Rust parity fixes and Tauri parity work with equal weight.

**Pros**
- Keeps both implementations moving

**Cons**
- Higher context cost
- Easier to blur shipping fixes with rewrite work

## Chosen design

Use **Option 1**.

### Architecture

- The current Rust tray app remains the only shipping desktop surface in this subproject.
- The mac app on `mac@imac-ca-mac` is the parity reference.
- The Windows VM is the acceptance environment for real Windows behavior.
- The Tauri shell is allowed to reference the resulting asset inventory later, but it does not become the shipping surface in this pass and is not rewired during this subproject.

### Work streams

#### 1. Behavior stream

Harden the current Rust tray app around the exact interaction model we want to mirror from the mac app.

This subproject plans against the following explicit parity contract. If live mac verification disproves any line item, update the contract first and then implement against the corrected version.

1. **Startup posture**
   - The app launches hidden when the tray is available.
   - The main window is only forced visible at startup when the tray is unavailable or when an explicit debug/test override requests visibility.
2. **Primary tray open**
   - Primary tray activation opens the summary dashboard in popup mode, not popout mode.
   - The popup opens tray-adjacent rather than centered or pointer-anchored.
3. **Context menu behavior**
   - Secondary tray activation opens the native tray context menu.
   - Dismissing the tray context menu returns the app to its hidden tray-first state.
4. **Popout behavior**
   - `Pop Out Dashboard` opens a normal popout window rather than the anchored popup surface.
   - Provider-specific tray popout actions open a normal popout window directly into the selected provider detail.
5. **Close behavior**
   - Closing the main window hides it back to tray instead of exiting the app.
6. **Settings behavior**
   - Opening Settings from the tray keeps the product tray-first; settings are shown without turning the main window into the primary desktop surface.
7. **Positioning**
   - Any tray-triggered open path stays anchored to the tray/work area on the correct monitor.
   - Popout placement may differ from popup placement, but both must remain tray-originated and intentional.

#### 2. Asset stream

Harvest official assets from the mac app bundle into the current Rust asset layout:

- app icon/logo assets
- provider icon assets when needed for parity
- any branding inputs required to remove placeholder drift in the current shipping app

This stream is **harvest only** for future consumers:

- canonical harvested SVG/icon sources live in `rust/assets/icons/`
- provider icons with matching official filenames are updated in place there
- Windows packaging/runtime copies under `rust/icons/` may be refreshed if needed, but they are downstream packaging assets rather than the canonical source
- no Tauri consumer rewiring happens in this subproject

## Behavioral rules

1. The live mac app is the first reference for how tray interaction should behave, and parity questions must resolve against it before implementation drifts.
2. Windows-native constraints are allowed, but only when they are required by the platform rather than by convenience.
3. The current Rust app must not regress tray-first behavior while being hardened.
4. Open positioning must be verified in the Windows VM, not assumed from code.
5. Placeholder branding in the shipping Rust path should be replaced with official CodexBar branding sources from the mac app where appropriate.
6. Future Tauri reuse is a downstream benefit, not an implementation requirement for this subproject.

## Implementation boundaries

- Primary implementation files are expected to live under:
  - `rust/src/tray/`
  - `rust/src/native_ui/`
  - `rust/assets/icons/`
  - `rust/icons/` only when packaging/runtime derivatives must be refreshed
- Tauri files in `apps/desktop-tauri/` may be referenced for later parity work, but this subproject should not turn them into the active desktop entry point or rewire Tauri consumers to the harvested assets.
- Do not touch Tailscale or other remote connectivity setup used for parity work.

## Validation

This subproject is complete only when all three checks agree:

1. **macOS parity reference**
   - the tray contract above is confirmed or corrected from the live mac app before implementation diverges
   - the official assets used in the Rust shipping path are confirmed from the live mac app bundle

2. **Windows VM behavior**
   - the actual Rust shipping app behaves correctly in the Windows VM

3. **Local code/test validation**
   - focused Rust tests remain green
   - product behavior is supported by the real implementation rather than dead helper logic or stale proofs

## Evidence to collect

- a short parity matrix from the live mac app confirming startup, primary click, context menu, popout, settings, close, and positioning behavior
- mac screenshots or resource extraction results that establish the expected tray/branding behavior
- fresh Windows VM screenshots or state dumps showing corrected behavior
- local build/test results for touched Rust code

## Error handling

- If mac reference material and prior captures disagree, prefer the live mac app over stale captures.
- If Windows VM proof disagrees with local assumptions, treat the VM as authoritative for Windows behavior.
- If an asset cannot be cleanly harvested from the mac bundle, use the closest existing official repo asset only as a temporary unblocker and record that gap explicitly rather than inventing a new placeholder; the subproject is not done until the shipping asset is confirmed from the live mac bundle or the exception is explicitly accepted.
- If a parity choice would require a large platform rewrite, defer it to the later Tauri migration instead of forcing it into this subproject.

## Testing implications

- Keep existing Rust tests green.
- Add focused tests for any tray or positioning logic that changes.
- Validate tray interaction and position in the Windows VM.
- Re-check the live mac app when behavior questions depend on parity rather than pure implementation correctness.
