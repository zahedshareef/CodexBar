# Repository Guidelines

## Current Project State
- This repository currently ships a Windows port of CodexBar implemented in Rust (`rust/`).
- Many files in `docs/` and some workflows reference the upstream macOS/Swift project. Treat those as historical or
  upstream-sync material unless the task is explicitly about upstream parity.
- When repo docs conflict, trust the active Rust sources in `rust/src` and the Rust manifests (`rust/Cargo.toml`).

## Project Structure & Modules
- `rust/src`: Main application code (CLI, providers, tray, native UI, browser cookie extraction, settings).
- `rust/src/providers`: Provider-specific fetch/parsing/auth logic. Keep provider boundaries clean.
- `rust/src/native_ui` and `rust/src/tray`: egui UI and tray integration.
- `rust/src/browser`: Browser detection + cookie extraction for Windows.
- `rust/assets`, `rust/icons`, `rust/gen`, `rust/wix`: UI assets, generated schemas, installer packaging.
- `docs`: Mixed documentation (Windows port docs plus upstream/macOS references). Update only the relevant docs.

## Build, Test, Run
- Work from `rust/` for most tasks: `cd rust`.
- Build: `cargo build` (debug) or `cargo build --release`.
- Test: `cargo test`.
- Run CLI locally: `cargo run -- --help`, `cargo run -- -p claude`, `cargo run -- cost`.
- Run the tray app (Windows): `cargo run -- menubar`.
- Format/lint before handoff when code changed: `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`
  (or explain why not run).
- There is no active root-level `Scripts/` build pipeline in this port. Do not rely on legacy `Scripts/*.sh` commands.

## Coding Style & Naming
- Prefer small, typed structs/enums and focused modules; keep changes local.
- Keep provider-specific logic inside the provider module instead of adding cross-provider branching.
- Preserve clear error handling and user-facing diagnostics (`anyhow`/`thiserror` + friendly messages where applicable).
- Use `tracing` for diagnostics; do not log raw secrets, cookies, or tokens.
- Avoid adding dependencies/tooling without confirmation.

## Testing Guidelines
- Add or extend focused Rust tests near the changed module (`#[cfg(test)]` unit tests are common in this repo).
- For parser/fetcher changes, add deterministic samples/fixtures where practical.
- Run `cargo test` after code changes; include any skipped checks in handoff.
- If UI/tray behavior changed, do a manual Windows validation when possible (`codexbar menubar`).

## Commit & PR Guidelines
- Use short imperative commit messages (for example: `Fix Claude CLI parser`, `Improve cookie import errors`).
- Keep commits scoped to one change.
- In PRs/patches, include:
  - Summary of behavior changes
  - Commands run (`cargo test`, `cargo fmt`, etc.)
  - Screenshots/GIFs for UI changes (Windows)
  - Linked issue/reference when relevant

## Agent Notes
- Active implementation is the Rust Windows port. Root Swift/macOS docs and scripts are not the default workflow here.
- Keep provider data siloed: never show identity/plan/email fields from provider A in provider B UI.
- Claude CLI output is user-configurable; do not depend on a customizable status line for usage parsing.
- Cookie import UX uses explicit browser selection in Preferences. Do not assume Chrome-only in general UI flows.
- Be conservative with secret handling (manual cookies, API keys, token accounts); use existing redaction/storage helpers.
- Prefer Windows-native validation for tray/DPAPI/browser-cookie behavior; WSL/Linux can be insufficient for those paths.
