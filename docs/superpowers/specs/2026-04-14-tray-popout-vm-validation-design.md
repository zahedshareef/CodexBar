# Tray-first startup and popout validation design

## Problem

Win-CodexBar should behave like a tray utility by default on Windows:

1. Launch hidden to the tray during normal startup.
2. Open the compact tray popup on tray left-click.
3. Expose a detach action from the tray context menu.
4. Open a decorated standalone window when the user chooses that detach action.

The codebase already contains tray-popup and popout-mode plumbing. This pass is to confirm the intended behavior in the Windows VM and only make targeted fixes if the proof shows a broken or missing step.

## Current context

- `rust/src/native_ui/app.rs` starts hidden to tray unless `CODEXBAR_START_VISIBLE` is set or tray creation fails.
- `rust/src/native_ui/app.rs` handles two window modes:
  - tray popup mode via `is_popout_mode = false`
  - decorated popout mode via `is_popout_mode = true`
- `rust/src/tray/manager.rs` already defines:
  - a `PopOut` tray action
  - a `Pop Out Dashboard` tray menu item
  - provider-specific popout actions

## Approved scope

- Keep the app tray-first by default.
- Keep the detach affordance in the tray menu.
- Do not add a second in-window detach button in this pass.
- Validate the behavior in the Windows VM.
- Patch only the behavior that fails proof.

## Approach options considered

### Option 1: Verify and minimally patch existing flow

Use the current tray and popout implementation, prove the end-to-end behavior in the VM, and only change code if the proof fails.

**Pros**
- Lowest risk
- Matches the current architecture
- Avoids adding duplicate UI

**Cons**
- Depends on VM proof reliability

### Option 2: Add a second detach button inside the popup

Keep the tray menu action but also add an in-window detach button.

**Pros**
- More discoverable

**Cons**
- Adds UI surface the user did not choose
- Risks more UI churn

### Option 3: Switch to popout-first behavior

Launch the app as a normal window and make tray interaction secondary.

**Pros**
- Easier to test manually

**Cons**
- Wrong default UX for a tray utility

## Chosen design

Use **Option 1**.

### Expected behavior

#### Startup

- With a working tray manager and no `CODEXBAR_START_VISIBLE`, the app starts hidden.
- This remains the default production behavior.
- The existing forced-visible and trayless fallback behavior remains unchanged for automation and failure recovery.

#### Tray left-click

- Left-click opens the compact tray popup anchored near the tray icon.
- The popup uses borderless, fixed chrome.
- This is the normal quick-glance interaction path.

#### Tray right-click menu

- Right-click opens the native tray context menu.
- The menu includes `Pop Out Dashboard`.
- This is the supported detach path for users.

#### Pop out

- Choosing `Pop Out Dashboard` opens the main window in decorated/resizable mode.
- The detached window is usable as a separate window rather than a tray-anchored popup.

## Validation plan

Validate in the Windows VM using the existing proof flow and fresh captures.

### Checks

1. Start the app normally and confirm it does not appear as a normal foreground window at launch.
2. Confirm the tray icon is present.
3. Left-click the tray icon and confirm the compact popup appears.
4. Right-click the tray icon and confirm the context menu includes `Pop Out Dashboard`.
5. Activate `Pop Out Dashboard` and confirm the resulting window is decorated/resizable and detached from the tray-popup presentation.

### Evidence to collect

- Fresh VM screenshots for tray popup and popout states
- Any state dump or debug output needed to distinguish popup mode from popout mode
- If behavior fails, capture the exact failing step before patching

## Error handling

- If tray manager creation fails, the existing visible-window fallback is acceptable and should not be treated as a regression in this pass.
- If proof artifacts disagree, prefer fresh visual inspection over stale or ambiguous captures.
- If the current proof scripts are insufficient for tray-menu verification, use the VM directly and keep any code changes scoped to the product behavior, not broad proof-pipeline refactors.

## Testing implications

- Keep existing Rust tests green.
- Add or adjust focused tests only if implementation changes are required.
- VM validation is required before concluding the tray-first + popout behavior is correct.
