---
name: installer-worker
description: Restore and verify the MSI packaging and updater handoff path using the existing WiX-based delivery flow.
---

# Installer Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the work procedure.

## When to Use This Skill

Use for features that touch `rust/wix/main.wxs`, `rust/src/updater.rs`, release-asset handling, MSI packaging enablement, or Wine installer smoke validation.

## Required Skills

- `tdd` — invoke before implementation; updater-selection and cache behavior should start with failing focused tests.
- `verification-before-completion` — invoke before ending the feature to ensure MSI build/install evidence is real.
- `systematic-debugging` — invoke if packaging tooling, cached update behavior, or Wine install flow is unreliable.

## Work Procedure

1. Identify whether the feature is about:
   - WiX packaging output
   - installed payload shape
   - updater asset selection/cache logic
   - updater-to-installer handoff
2. Add failing automated tests first for any updater behavior change.
3. Preserve the existing WiX-based delivery path; do not replace it with a different installer technology.
4. If MSI tooling is missing, do the smallest necessary enablement work within feature scope. If the environment still cannot build an MSI after reasonable setup, return to orchestrator with the exact blocker.
5. Verify packaging from a clean build state when touching installer content.
6. When validating installed payload or handoff behavior, use an isolated `WINEPREFIX` and capture:
   - generated MSI artifact
   - installed file list
   - post-install launch evidence
7. Run baseline repo validators plus any feature-specific packaging/update commands.

## Example Handoff

```json
{
  "salientSummary": "Restored the MSI packaging path, removed stale non-shipping payload assumptions from the WiX template, and taught the updater to prefer and hand off cached MSI installers correctly. Verified the flow with focused updater tests and a Wine `msiexec` smoke install.",
  "whatWasImplemented": "Updated the checked-in WiX packaging path so it builds a shipping MSI from a clean release build, removed stale helper-binary assumptions from the installed payload, and extended updater asset selection and pending-update handling to support MSI preference, MSI cache rediscovery, and MSI-specific install handoff.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test --manifest-path /home/fsos/Developer/Win-CodexBar/rust/Cargo.toml updater -- --nocapture",
        "exitCode": 0,
        "observation": "MSI preference, exe fallback, and cached-MSI rediscovery tests passed."
      },
      {
        "command": "cd /home/fsos/Developer/Win-CodexBar/rust && cargo build --release --target x86_64-pc-windows-gnu --bin codexbar && cargo wix --package codexbar --profile release --target x86_64-pc-windows-gnu --output target/wix/codexbar-test.msi",
        "exitCode": 0,
        "observation": "Packaging succeeded from a clean release build and produced a non-empty MSI."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Installed the generated MSI under an isolated Wine prefix with `msiexec`.",
        "observed": "Installation completed and the installed app launched to the tray/menubar entrypoint."
      },
      {
        "action": "Listed the installed files in the Wine prefix.",
        "observed": "The install tree contained the supported app payload and did not include stale helper binaries such as `gen_icons.exe`."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "rust/src/updater.rs",
        "cases": [
          {
            "name": "prefers msi when both msi and exe are present",
            "verifies": "MSI becomes the preferred installer asset."
          },
          {
            "name": "falls back to legacy exe when no msi exists",
            "verifies": "Legacy releases still auto-select the exe path."
          },
          {
            "name": "cached msi stays pending across restart",
            "verifies": "Downloaded MSI updates remain discoverable and installable."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- MSI tooling cannot be made runnable from this environment within the current feature scope
- The change would require replacing WiX with another installer technology
- The installed payload or updater handoff behavior cannot be validated under Wine on this host
