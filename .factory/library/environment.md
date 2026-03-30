# Environment

Environment variables, external dependencies, and setup notes for this mission.

**What belongs here:** required env vars, external tools, platform constraints, setup quirks.
**What does not belong here:** service ports or command aliases (use `.factory/services.yaml`).

---

## Current platform facts

- Host OS for mission execution: Linux
- Rust app under test: `rust/`
- Windows validation path: cross-build `x86_64-pc-windows-gnu`, then run under Wine/Xvfb
- `rust/.cargo/config.toml` currently pins `build.target = "x86_64-pc-windows-msvc"`
- `cargo test` from `rust/` needs an explicit host override on this machine; `cargo test --manifest-path rust/Cargo.toml` already works

## Tooling assumptions

- Required and already present: `cargo`, `rustc`, `rustup`, `wine`, `xvfb-run`, MinGW GNU cross toolchain
- Missing at planning time: `cargo-wix` and native WiX authoring/build tooling
- Installer milestone must either make MSI packaging runnable from this environment or return to orchestrator/user with a concrete blocker

## Mission-specific execution constraints

- Use isolated `WINEPREFIX` directories under `/tmp` or another mission-owned temp path
- Do not rely on unrelated local listeners already in use on this machine
- Do not assume PowerShell Core (`pwsh`) is available on Linux; if `dev.ps1` validation needs Windows PowerShell behavior, use the branch’s chosen validation path and capture evidence

## External credentials

- No new shared credentials were provisioned during planning
- Claude parser correctness may be validated with focused deterministic tests and fixture-backed CLI inputs
- If any worker decides live provider validation is required for a change, they must stop and return to orchestrator rather than improvising with unavailable credentials
