# User Testing

Testing surface notes and validator guidance for this mission.

**What belongs here:** validation surfaces, tool choices, environment setup, isolation notes, concurrency limits.
**What does not belong here:** build/lint/test aliases (use `.factory/services.yaml`).

---

## Validation Surface

### Surface: Rust tests and CLI output

- Primary tools: `cargo test`, `cargo run --manifest-path rust/Cargo.toml -- ...`
- Use for:
  - Claude parser correctness
  - settings persistence and language-default behavior
  - updater selection/cache logic
- Deterministic validation is preferred here over live external-provider calls

### Surface: Windows GUI under Wine/Xvfb

- Primary tools: `xvfb-run`, `wine`, screenshot capture, any existing repo or local desktop tooling needed to expose the popup/preferences/tray UI
- Use for:
  - runtime language switching
  - visible auth-recovery affordances
  - post-install installed-app smoke checks
- Always isolate with a mission-owned `WINEPREFIX`
- When validating Claude GUI parsing, prefer an isolated environment with no OAuth/cookie credentials and a fixture-backed Claude CLI first on `PATH`

### Surface: MSI packaging and installer smoke checks

- Primary tools: `cargo build`, `cargo wix` if available, `wine msiexec`
- Use for:
  - MSI artifact generation
  - installed payload checks
  - updater-to-installer handoff checks
- If the environment cannot produce MSI artifacts, workers must return that blocker instead of silently downgrading to exe-only release behavior

## Validation Concurrency

### CLI/build validation

- Max concurrent validators: **1**
- Rationale:
  - machine load was already high during dry run
  - cargo test/build peaks were roughly 1.2–1.5 GiB RSS each
  - build parallelism is better kept inside Cargo than across multiple validator sessions

### Wine UI validation

- Max concurrent validators: **1**
- Rationale:
  - Wine/Xvfb runs are stateful and easier to reason about in isolation
  - auth-recovery and tray behavior become harder to attribute when multiple UI runs share environment state

## Dry-run findings to preserve

- `cargo test --manifest-path rust/Cargo.toml` works on this machine
- `cargo test` from `rust/` requires explicit host-target override because the repo default target is Windows MSVC
- Windows GNU release builds already succeed locally
- Wine/Xvfb can launch `codexbar.exe menubar`
- MSI packaging was blocked during planning because `cargo-wix` and WiX build tooling were missing locally

## Validator watchouts

- Exclude unrelated local listeners and off-limits ports from any validation setup
- Prefer temporary config directories or prefixes when validating first-launch or legacy-settings behavior
- Do not treat provider names or third-party status-page strings as localization failures when they are intentionally source-owned
