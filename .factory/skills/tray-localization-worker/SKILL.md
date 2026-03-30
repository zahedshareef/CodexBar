---
name: tray-localization-worker
description: Finish tray-only localization work in locale.rs and tray/manager.rs without touching popup or Preferences surfaces.
---

# Tray Localization Worker

NOTE: Startup and cleanup are handled by `mission-worker-base`. This skill defines the work procedure.

## When to Use This Skill

Use only for the tray-localization feature that finishes single-icon and per-provider tray text behavior in `rust/src/tray/manager.rs`.

## Required Skills

- `tdd` — invoke before implementation; add or extend failing locale/helper tests first.
- `verification-before-completion` — invoke before ending the feature to ensure tray evidence is real.

## Work Procedure

1. Stay tightly scoped to `rust/src/locale.rs` and `rust/src/tray/manager.rs` unless a compile error forces a minimal supporting edit elsewhere.
2. Preserve any existing partial tray-localization changes already in those files.
3. Add or extend failing tests for new locale keys or tray-label helper behavior first.
4. Finish localizing single-icon and per-provider tray menu/tooltip text, including one tray-owned special state.
5. Do not work on popup or Preferences text in this feature.
6. Run focused tests, then baseline `test`, `typecheck`, `lint`, and Windows GNU build verification.
7. Capture tray evidence under Wine/Xvfb for both a normal state and one tray-owned special state.

## Example Handoff

```json
{
  "salientSummary": "Finished tray-only localization on the PR #14 branch by completing locale-backed single-icon and per-provider tray strings in `locale.rs` and `tray/manager.rs`. Verified live tray refresh in a normal state and a tray-owned special state under Wine/Xvfb.",
  "whatWasImplemented": "Extended locale keys for tray-owned labels and completed the tray rendering migration so single-icon and per-provider menu/tooltip text switch live between English and Chinese, including a special tray-owned state. Popup and Preferences behavior were left to their own features.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml",
        "exitCode": 0,
        "observation": "Tests passed after finishing tray localization."
      },
      {
        "command": "cargo build --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml --bin codexbar --release --target x86_64-pc-windows-gnu",
        "exitCode": 0,
        "observation": "Windows GNU release build passed."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Changed the language while the tray app was running under Wine/Xvfb.",
        "observed": "Single-icon and per-provider tray text refreshed live."
      },
      {
        "action": "Viewed one tray-owned special state after the language switch.",
        "observed": "The special tray-owned state text was localized in the selected language."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "rust/src/locale.rs",
        "cases": [
          {
            "name": "tray locale keys exist in both languages",
            "verifies": "All tray-owned labels have both English and Chinese variants."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- The feature cannot be completed without pulling in popup or Preferences work
- The working tree contains unrelated uncommitted changes outside the tray scope that cannot be isolated safely
