# Architecture

High-level map of how the Windows Rust app is structured for this mission.

**What belongs here:** components, relationships, data flow, important invariants.
**What does not belong here:** step-by-step feature tasks or branch choreography.

---

## System shape

CodexBar is a Rust application with two user-facing entrypoints:

- **CLI** via `rust/src/main.rs` and `rust/src/cli/*`
- **Windows tray/menubar app** via `rust/src/native_ui/*` and `rust/src/tray/*`

Both entrypoints consume the same provider layer and settings layer. Mission work must preserve that shared-core model instead of forking separate logic for CLI and UI.

## Shared core

### Provider layer

- `rust/src/core/provider.rs` defines provider-facing contracts and shared fetch context
- `rust/src/providers/*` contains provider-specific fetch, parse, and auth logic
- Claude is special for this mission:
  - `rust/src/providers/claude/mod.rs` selects source and parses CLI output
  - `oauth.rs` maps OAuth usage responses
  - `web_api.rs` maps cookie/web usage responses

Invariant: the UI and CLI should consume the same normalized usage semantics for a provider rather than each inventing their own interpretation.

### Settings and persistence

- `rust/src/settings.rs` owns persisted user settings and secret-adjacent supporting files
- Settings are the natural home for persisted UI language
- Existing configs must continue to load when new settings fields are introduced

Invariant: adding a new preference must be backward-compatible for existing user config files unless the mission explicitly says otherwise.

## UI surfaces

### Preferences window

- `rust/src/native_ui/preferences.rs`
- Holds the heaviest concentration of app-owned text and recovery-adjacent settings surfaces
- Also contains cookie import, API key management, and update/about surfaces used by this mission

### Main popup / provider detail

- `rust/src/native_ui/app.rs`
- Renders provider summaries, detail panels, error states, update affordances, and top-level actions
- This is where auth-recovery affordances must become visible from failure states

### Tray integration

- `rust/src/tray/manager.rs`
- Builds single-icon and per-provider tray menus/tooltips
- Language work must refresh live tray state, not just window-local text

Current-state note: there is no shared locale module or persisted language preference yet; localization is still scattered and partially hardcoded.

Target-state invariant for PR #14: app-owned strings should come from a shared locale source once the i18n work lands; popup, preferences, and tray must not drift independently.

## Launcher and packaging surfaces

### PowerShell development launcher

- `dev.ps1`
- Builds and launches the Windows binary, then resolves an existing binary path for `-SkipBuild`
- The mission bug is path discovery, not the menubar app itself

### Installer packaging

- `rust/wix/main.wxs` is the existing MSI template path
- `rust/src/updater.rs` handles release asset selection, cached update discovery, and install handoff

Current-state note: `rust/src/updater.rs` is still `.exe`-centric at planning time; MSI preference and MSI-specific handoff are target-state changes for issue #13, not already-present behavior.

Target-state invariant for issue #13: installer/update flow should treat MSI as the preferred modern installer path without regressing legacy `.exe` fallback behavior when MSI is absent.

## Auth and recovery flow

- `rust/src/login.rs` already contains helper-backed login runners for a subset of providers
- Recovery UI today is fragmented across popup error states and Preferences management panes
- Issue #13 work should connect visible failure states to the correct existing recovery surface instead of inventing a separate auth subsystem

Current-state note: `login.rs` helpers mostly exist as available scaffolding; they are not yet broadly wired into popup failure actions.

Target-state invariant for issue #13: recovery actions must be source-appropriate. Cookie failures should route to cookie recovery; API-key failures should route to key management; helper-backed providers should expose in-app reauth when available.
