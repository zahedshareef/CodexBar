---
name: claude-parser-worker
description: Harden Claude usage parsing and the Windows launcher path resolution without relying on live provider credentials.
---

# Claude Parser Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the work procedure.

## When to Use This Skill

Use for features that change Claude CLI/OAuth/web normalization, Claude fixture-backed validation, or `dev.ps1` target discovery.

## Required Skills

- `tdd` — invoke before implementation; parser fixes must begin with failing fixtures or focused tests.
- `verification-before-completion` — invoke before ending the feature so every claimed parser or launcher fix is backed by command output.
- `systematic-debugging` — invoke if fixture expectations, source selection, or launcher path resolution behave unexpectedly.

## Work Procedure

1. Identify the exact assertion IDs fulfilled by the feature and the surface they map to:
   - CLI JSON/text
   - focused module tests
   - Wine UI smoke path
   - `dev.ps1`
2. Write failing tests or fixture-backed checks first:
   - decimal percentages
   - exhausted-session handling
   - Sonnet-only model fallback and labeling
   - OAuth/web normalization
   - target-discovery cases for `dev.ps1`
3. Keep parsing logic centralized in the Claude provider path; do not duplicate normalization logic in the CLI renderer or UI layer.
4. For `dev.ps1`, validate path discovery against the active target layout, not just the repo’s current hardcoded triples.
5. Run targeted tests first, then baseline repo validators from `.factory/services.yaml`.
6. For any UI-facing Claude assertion, use an isolated run with no OAuth/cookie credentials and a fixture-backed Claude CLI placed first on `PATH`.
7. Capture the exact fixture/transcript or process-path evidence that proves the bug is fixed.

## Example Handoff

```json
{
  "salientSummary": "Hardened Claude CLI parsing for decimal and Sonnet-only output, normalized OAuth/web utilization to 0-100 percentages, and fixed `dev.ps1` so `-SkipBuild` resolves the active target output path. Verified the parser through focused tests and fixture-backed CLI runs.",
  "whatWasImplemented": "Added failing parser fixtures first, updated Claude normalization logic for CLI/OAuth/web sources, corrected Sonnet-only human-readable labeling, and changed `dev.ps1` to discover the active Cargo target output path rather than assuming default or GNU-only directories.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml claude -- --nocapture",
        "exitCode": 0,
        "observation": "Focused Claude parser and normalization tests passed."
      },
      {
        "command": "PATH=/tmp/claude-fixture:$PATH cargo run --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml -- usage -p claude --source cli --json --pretty",
        "exitCode": 0,
        "observation": "JSON output preserved the expected decimal percentages."
      },
      {
        "command": "PATH=/tmp/claude-fixture:$PATH cargo run --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml -- usage -p claude",
        "exitCode": 0,
        "observation": "Human-readable output labeled the model-specific quota as Sonnet in the Sonnet-only fixture."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Launched the menubar app under Wine/Xvfb with no Claude OAuth/cookie credentials and fixture-backed Claude CLI on PATH.",
        "observed": "The Claude detail card matched the expected session, weekly, and model values from the CLI fixture."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "rust/src/providers/claude/mod.rs",
        "cases": [
          {
            "name": "decimal percentages survive cli parsing",
            "verifies": "CLI `/usage` decimals remain on the 0-100 scale."
          },
          {
            "name": "sonnet only quota stays visible and labeled",
            "verifies": "Model-specific output remains present and uses the correct label."
          }
        ]
      },
      {
        "file": "rust/src/providers/claude/oauth.rs",
        "cases": [
          {
            "name": "oauth utilization fractions normalize to percentages",
            "verifies": "OAuth payloads do not leak raw fractions into user-facing usage."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- The only way to validate a claimed behavior would require live provider credentials not available to the mission
- `dev.ps1` needs a broader tooling or platform decision than the feature description permits
- A required fixture seam does not exist and cannot be added within the current feature scope
