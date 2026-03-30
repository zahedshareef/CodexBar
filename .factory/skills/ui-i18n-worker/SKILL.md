---
name: ui-i18n-worker
description: Implement runtime localization and persisted language behavior across the Rust Windows UI surfaces.
---

# UI I18N Worker

NOTE: Startup and cleanup are handled by `mission-worker-base`. This skill defines the work procedure.

## When to Use This Skill

Use for features that add or modify app-owned UI language behavior in `settings.rs`, `native_ui/app.rs`, `native_ui/preferences.rs`, `tray/manager.rs`, or a shared locale helper module.

## Required Skills

- `tdd` — invoke before writing implementation code; localization persistence and helper logic must be driven by failing tests first.
- `verification-before-completion` — invoke before ending the feature to ensure commands and UI evidence actually support the claims.
- `systematic-debugging` — invoke if live UI refresh, tray rebuild, or persistence behavior does not match the expected surface behavior.

## Work Procedure

1. Identify which assertions the feature fulfills and which UI surfaces those assertions cover.
2. Add or extend failing tests first:
   - settings defaults / backward-compatible load behavior
   - locale helper or formatter behavior
   - any routing or state-change logic that can be exercised without Wine
3. Implement the smallest shared locale/persistence change that can support all touched surfaces. Do not duplicate strings independently across popup, preferences, and tray.
4. Wire the required surface only for the current feature scope. Keep provider names and source-owned third-party status strings separate from app-owned text.
5. Run fast focused tests until green, then run baseline repo validators from `.factory/services.yaml`.
6. Perform manual Wine/Xvfb verification for the exact surfaces in scope:
   - use isolated config directories or prefixes for first-launch / legacy-config cases
   - capture before/after evidence for live switching
   - capture tray evidence for the appropriate tray mode(s)
7. Confirm no orphaned Wine/Xvfb/helper processes remain.

## Example Handoff

```json
{
  "salientSummary": "Added persisted UI language selection with English default and wired live localization through the Preferences window. Verified legacy configs without a language field still open in English and captured before/after Wine screenshots for the live switch.",
  "whatWasImplemented": "Introduced a persisted language setting with backward-compatible loading semantics, added locale helpers for app-owned strings, and updated the Preferences viewport so title, tab labels, headings, and representative controls switch live between English and Chinese.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml settings",
        "exitCode": 0,
        "observation": "New settings default and roundtrip tests passed."
      },
      {
        "command": "cargo check --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml --target x86_64-unknown-linux-gnu",
        "exitCode": 0,
        "observation": "Typecheck passed on the Linux host target."
      },
      {
        "command": "cargo fmt --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml --all --check",
        "exitCode": 0,
        "observation": "Formatting check passed."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Launched `codexbar.exe menubar` under Wine/Xvfb with a clean config and opened Preferences.",
        "observed": "Preferences opened in English by default."
      },
      {
        "action": "Changed the language selector to Chinese without restarting.",
        "observed": "Preferences title, tabs, headings, and save/cancel style controls switched to Chinese in the same session."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "rust/src/settings.rs",
        "cases": [
          {
            "name": "missing ui_language defaults to english",
            "verifies": "Legacy configs without the new field remain backward-compatible."
          },
          {
            "name": "ui_language roundtrips through persisted settings",
            "verifies": "Selected language is saved and restored."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- A required UI surface cannot be exercised under Wine/Xvfb on this host
- Live language switching requires a broader architectural change than the current feature scope allows
- The feature would need credentials or provider state the mission has not provisioned
