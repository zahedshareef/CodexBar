---
name: preferences-localization-worker
description: Finish Preferences-only localization work in locale.rs and native_ui/preferences.rs without touching popup or tray surfaces.
---

# Preferences Localization Worker

NOTE: Startup and cleanup are handled by `mission-worker-base`. This skill defines the work procedure.

## When to Use This Skill

Use only for the Preferences live-switch feature that localizes representative control-level strings in `rust/src/native_ui/preferences.rs`.

## Required Skills

- `tdd` — invoke before implementation; add or extend failing locale/helper tests first.
- `verification-before-completion` — invoke before ending the feature to ensure live Preferences evidence is real.

## Work Procedure

1. Stay tightly scoped to `rust/src/locale.rs` and `rust/src/native_ui/preferences.rs` unless a compile error forces a minimal supporting edit elsewhere.
2. Preserve any existing partial Preferences-localization changes already in those files.
3. Add or extend failing tests for new locale keys or helper behavior first.
4. Replace representative control-level Preferences strings, not just titles and headings.
5. Do not work on popup or tray text in this feature.
6. Run focused tests, then baseline `test`, `typecheck`, and `lint` commands.
7. If the feature fulfills contract assertions, capture before/after Preferences screenshots under Wine/Xvfb from the same running session while switching language.
8. If the feature is groundwork-only with an empty `fulfills` array, stop after code/test verification and leave the final live-switch UI proof to the later finishing feature.

## Example Handoff

```json
{
  "salientSummary": "Finished Preferences-only localization on the PR #14 branch by completing representative control-level strings in `locale.rs` and `preferences.rs`. Verified that the already-open Preferences window switches live between English and Chinese without touching popup or tray surfaces.",
  "whatWasImplemented": "Extended locale keys for the Preferences window and completed the Preferences rendering migration in `native_ui/preferences.rs` so representative controls in a control-heavy pane switch live between English and Chinese. Popup and tray behavior were left for their own features.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml",
        "exitCode": 0,
        "observation": "Tests passed after finishing Preferences localization."
      },
      {
        "command": "cargo check --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml --target x86_64-unknown-linux-gnu",
        "exitCode": 0,
        "observation": "Typecheck passed."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Changed the language while the Preferences window was open under Wine/Xvfb.",
        "observed": "Representative control-level strings switched live in the same session."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "rust/src/locale.rs",
        "cases": [
          {
            "name": "preferences locale keys exist in both languages",
            "verifies": "Representative Preferences controls have both English and Chinese variants."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- The feature cannot be completed without pulling in popup or tray work
- The working tree contains unrelated uncommitted changes outside the Preferences scope that cannot be isolated safely
