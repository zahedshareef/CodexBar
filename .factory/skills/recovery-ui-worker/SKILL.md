---
name: recovery-ui-worker
description: Add source-appropriate auth-recovery actions to the Windows UI and prove that the provider can leave the error state after recovery.
---

# Recovery UI Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the work procedure.

## When to Use This Skill

Use for features that change visible error-state copy, recovery routing, in-app reauthentication affordances, or recovery-related popup/preferences flows.

## Required Skills

- `tdd` — invoke before implementation; routing or state-mapping logic should begin with failing tests where practical.
- `verification-before-completion` — invoke before ending the feature so the claimed recovery path is backed by command and UI evidence.
- `systematic-debugging` — invoke if the provider does not leave the error state, the wrong recovery path appears, or UI actions disappear.

## Work Procedure

1. Identify the precise failure family in scope:
   - cookie-backed auth failure
   - API-key-backed auth failure
   - helper-backed OAuth/CLI reauth failure
   - shared error-state utility actions
2. Add failing tests first for any mapping or routing logic that can be tested without the UI.
3. Keep recovery actions source-appropriate:
   - cookies -> cookie management
   - API keys -> key management
   - helper-backed providers -> in-app reauth
4. Reuse existing settings and login surfaces instead of inventing parallel credential flows.
5. Preserve existing utility actions on the error panel while adding the new recovery path.
6. Perform manual Wine/Xvfb checks for the exact error-state flows the feature fulfills:
   - induce or seed the failure state
   - invoke the recovery action
   - complete the credential or reauth step
   - refresh
   - confirm the provider leaves the error state
7. Capture both the recovery affordance and the post-recovery non-error state.

## Example Handoff

```json
{
  "salientSummary": "Added source-specific recovery actions to the provider detail panel and wired helper-backed providers to in-app reauthentication. Verified cookie, API-key, and helper-backed recovery flows under Wine and confirmed the affected providers left the error state after recovery.",
  "whatWasImplemented": "Updated provider error-state rendering so auth failures display specific recovery copy, preserve dashboard/status/copy-error utility actions, and route users to the correct recovery surface. Helper-backed providers now expose in-app reauthenticate actions, while cookie and API-key failures drive the user to the appropriate settings destination and recover back to a non-error state after refresh.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml recovery -- --nocapture",
        "exitCode": 0,
        "observation": "Recovery routing tests passed for cookie, API-key, and helper-backed failure mappings."
      },
      {
        "command": "cargo check --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml --target x86_64-unknown-linux-gnu",
        "exitCode": 0,
        "observation": "Typecheck passed after wiring new recovery actions."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Opened a cookie-backed provider in an induced auth failure state and used the visible recovery action to reach cookie management.",
        "observed": "After saving cookies and refreshing, the provider left the error state and returned to a normal usage surface."
      },
      {
        "action": "Opened a helper-backed provider auth failure and triggered the in-app reauthenticate action.",
        "observed": "The login flow launched with visible in-progress status, and after completion plus refresh the provider returned to a non-error state."
      },
      {
        "action": "Inspected the same error-state panel after adding recovery actions.",
        "observed": "Dashboard/settings escape hatches plus retry/refresh, copy-error, and status actions remained visible where applicable."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "rust/src/native_ui/app.rs",
        "cases": [
          {
            "name": "cookie failures map to cookie recovery",
            "verifies": "The UI points cookie-backed auth failures at the cookie-management flow."
          },
          {
            "name": "helper backed providers expose in app reauth",
            "verifies": "Claude, Codex, Gemini, and Copilot surface reauthenticate actions."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- A claimed recovery flow cannot be completed because the required credentials or provider state do not exist
- The current feature would need a broader provider-auth redesign instead of UI/routing work
- A required visible recovery destination does not exist yet elsewhere in the app
