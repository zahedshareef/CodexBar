---
name: popup-localization-worker
description: Finish popup-only localization work in locale.rs and native_ui/app.rs without touching broader settings or tray surfaces.
---

# Popup Localization Worker

NOTE: Startup and cleanup are handled by `mission-worker-base`. This skill defines the work procedure.

## When to Use This Skill

Use only for the popup-localization feature that finishes app-owned strings in `rust/src/locale.rs` and `rust/src/native_ui/app.rs`.

## Required Skills

- `tdd` — invoke before implementation; add or extend failing locale/helper tests first.
- `verification-before-completion` — invoke before ending the feature to ensure popup evidence is real.

## Work Procedure

1. Stay tightly scoped to `rust/src/locale.rs` and `rust/src/native_ui/app.rs` unless a compile error forces a minimal supporting edit elsewhere.
2. Preserve any existing partial popup-localization changes already in those files.
3. Add or extend failing tests for new locale keys or popup-label helpers first.
4. Finish replacing remaining app-owned popup strings, including at least one non-happy-path popup state.
5. Do not work on Preferences or tray text in this feature.
6. Run focused tests, then baseline `test`, `typecheck`, and `lint` commands.
7. Capture before/after popup evidence under Wine/Xvfb in the same running session.

## Example Handoff

```json
{
  "salientSummary": "Finished popup-only localization on the PR #14 branch by completing the remaining app-owned popup strings in `locale.rs` and `app.rs`. Verified live switching in the popup, including an error/update state, without changing Preferences or tray surfaces.",
  "whatWasImplemented": "Extended the locale key set for popup-owned labels and completed the popup rendering migration in `native_ui/app.rs` so actions, usage/reset labels, and one non-happy-path state switch live between English and Chinese. No tray or Preferences work was included in this feature.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml",
        "exitCode": 0,
        "observation": "Tests passed after finishing popup localization."
      },
      {
        "command": "cargo check --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml --target x86_64-unknown-linux-gnu",
        "exitCode": 0,
        "observation": "Typecheck passed."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Changed the language while the popup was open under Wine/Xvfb.",
        "observed": "Popup actions and app-owned labels switched live."
      },
      {
        "action": "Viewed a popup non-happy-path state after the language switch.",
        "observed": "The non-happy-path popup copy was localized in the selected language."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "rust/src/locale.rs",
        "cases": [
          {
            "name": "popup locale keys exist in both languages",
            "verifies": "All popup-owned labels have both English and Chinese variants."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- The feature cannot be completed without pulling in broader Preferences or tray work
- The working tree contains unrelated uncommitted changes outside the popup scope that cannot be isolated safely
