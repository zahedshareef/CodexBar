# Tray-first startup and popout VM validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove in the Windows VM that Win-CodexBar launches hidden to tray by default, opens the compact popup on tray left-click, exposes `Pop Out Dashboard` in the tray context menu, and opens a decorated detached window from that popout action.

**Architecture:** Keep the current tray-first product behavior and add only the minimum regression coverage and proof instrumentation needed to verify it reliably. Use local Rust tests to lock the tray-action and window-mode invariants first, then use the VM harness for popup/popout proof and one explicit native right-click tray-menu verification step for the OS context menu label.

**Tech Stack:** Rust (`eframe`/`egui` app, tray manager, debug test server), PowerShell VM proof scripts, Bash proof wrapper, Parallels Windows VM

---

## File structure / responsibilities

- **Modify:** `rust/src/tray/manager.rs`
  - Owns native tray menu items and `TrayMenuAction` ID mapping.
  - Add the missing regression for the top-level `"popout"` menu event ID if it is not already covered.

- **Modify:** `rust/src/native_ui/app.rs`
  - Owns tray-left-click popup mode, popout mode, and debug state JSON written by the test server.
  - Add proof-visible window-mode state if the current JSON is insufficient to distinguish popup vs popout during VM validation.

- **Modify:** `rust/src/native_ui/test_server.rs`
  - Owns debug-only TCP commands used by the VM proof harness.
  - Add a dedicated popout trigger command so VM automation can exercise the same code path as `TrayMenuAction::PopOut`.

- **Modify:** `scripts/vm/provider_osclick_proof_unc.ps1`
  - Owns guest-side Windows proof launch and screenshot capture.
  - Extend it with a `popout` capture path that opens the detached window and captures fresh evidence.

- **Modify:** `scripts/run_vm_provider_proof.sh`
  - Owns host-side sync, artifact fetch, and validation.
  - Teach it to fetch and report the new popout proof artifacts.

- **Check only:** `docs/ui-parity-workflow.md`
  - Update only if the proof command surface changes enough that future runs would be confusing without documentation.

## Task 1: Lock the tray action and window-mode regression points

**Files:**
- Modify: `rust/src/tray/manager.rs`
- Modify: `rust/src/native_ui/app.rs`
- Test: `rust/src/tray/manager.rs`
- Test: `rust/src/native_ui/app.rs`

- [ ] **Step 1: Write the failing tray-action test for the top-level popout menu item**

```rust
#[test]
fn test_tray_action_from_event_id_maps_top_level_popout() {
    assert_eq!(tray_action_from_event_id("popout"), Some(TrayMenuAction::PopOut));
}
```

- [ ] **Step 2: Run the targeted tray test to verify it fails if coverage is missing**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar/rust
cargo test test_tray_action_from_event_id_maps_top_level_popout -- --exact
```

Expected: FAIL if the test does not exist yet, or PASS immediately if coverage already exists and you can move on without code changes in this file.

- [ ] **Step 3: Add proof-visible window-mode output in the debug state payload**

Use a minimal helper so the VM proof can distinguish popup vs popout without guessing from screenshots alone.

```rust
fn debug_window_mode(is_popout_mode: bool) -> &'static str {
    if is_popout_mode { "popout" } else { "popup" }
}
```

Include it in the debug state JSON near the existing viewport/tray fields.

- [ ] **Step 4: Write the failing app test for the debug state field**

```rust
#[test]
fn debug_window_mode_reports_popup_and_popout() {
    assert_eq!(debug_window_mode(false), "popup");
    assert_eq!(debug_window_mode(true), "popout");
}
```

- [ ] **Step 5: Run the focused app test**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar/rust
cargo test debug_window_mode_reports_popup_and_popout -- --exact
```

Expected: FAIL before the helper exists, then PASS after the helper and payload wiring are in place.

- [ ] **Step 6: Run the targeted Rust checks for both files**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar/rust
cargo test test_tray_action_from_event_id_maps_top_level_popout -- --exact
cargo test debug_window_mode_reports_popup_and_popout -- --exact
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
cd /home/fsos/Developer/Win-CodexBar
git add rust/src/tray/manager.rs rust/src/native_ui/app.rs
git commit -m "test: lock tray popout action and debug window mode"
```

## Task 2: Add a debug-only popout trigger for VM proof runs

**Files:**
- Modify: `rust/src/native_ui/test_server.rs`
- Modify: `rust/src/native_ui/app.rs`
- Test: `rust/src/native_ui/test_server.rs`

- [ ] **Step 1: Write the failing parse test for a dedicated popout command**

```rust
#[test]
fn parses_simulate_tray_popout() {
    assert_eq!(
        parse_test_input(r#"{"type":"simulate_tray_popout"}"#),
        Some(TestInput::SimulateTrayPopOut)
    );
}
```

- [ ] **Step 2: Run the targeted parser test to verify it fails**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar/rust
cargo test parses_simulate_tray_popout -- --exact
```

Expected: FAIL because the enum variant / parser branch does not exist yet.

- [ ] **Step 3: Add the new debug command to the test server**

Extend the enum, docs comment block, and parser:

```rust
pub enum TestInput {
    // ...
    SimulateTrayPopOut,
}

// ...
"simulate_tray_popout" => Some(TestInput::SimulateTrayPopOut),
```

- [ ] **Step 4: Route the new command through the same app path as `TrayMenuAction::PopOut`**

Extract the existing popout branch into a small helper if that makes the code less repetitive:

```rust
fn activate_popout_mode(&mut self, ctx: &egui::Context) {
    self.is_popout_mode = true;
    if let Ok(mut state) = self.state.lock() {
        state.selected_tab = SelectedTab::Summary;
    }
    self.pending_main_window_layout = true;
    self.anchor_main_window_to_pointer = true;
    self.layout_main_window(ctx, true);
}
```

Use that helper for both the real tray action and the debug-only test command.

- [ ] **Step 5: Re-run the focused parser test**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar/rust
cargo test parses_simulate_tray_popout -- --exact
```

Expected: PASS

- [ ] **Step 6: Run a small local regression sweep for startup/popout logic**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar/rust
cargo test forced_visible_startup_stays_in_popup_mode -- --exact
cargo test trayless_startup_uses_popout_mode_only_without_forced_visible -- --exact
cargo test parses_simulate_tray_left_click -- --exact
cargo test parses_simulate_tray_popout -- --exact
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
cd /home/fsos/Developer/Win-CodexBar
git add rust/src/native_ui/test_server.rs rust/src/native_ui/app.rs
git commit -m "test: add VM popout trigger command"
```

## Task 3: Extend the VM proof harness for detached-window evidence

**Files:**
- Modify: `scripts/vm/provider_osclick_proof_unc.ps1`
- Modify: `scripts/run_vm_provider_proof.sh`
- Check: `docs/ui-parity-workflow.md`

- [ ] **Step 1: Add a new guest-side capture mode for popout proof**

Extend the PowerShell parameter validation to accept `popout`:

```powershell
[ValidateSet('provider', 'tab', 'menu', 'popout')]
```

- [ ] **Step 2: Implement the guest-side popout flow**

After startup, use the new debug command to trigger the same path as `TrayMenuAction::PopOut`, then save state and capture the full desktop / window evidence.

```powershell
Send-TestCommand -Line '{"type":"simulate_tray_popout"}'
Start-Sleep -Milliseconds 1200
Send-TestCommand -Line ('{"type":"save_state","path":"' + ($statePath -replace '\\', '\\\\') + '"}')
```

Fetch and preserve:

- `C:\Users\mac\Desktop\<proof>-state.json`
- `C:\Users\mac\Desktop\<proof>-interactive-full.png`
- optional window-capture logs if the proof helper already emits them

- [ ] **Step 3: Teach the Bash wrapper to fetch/report popout artifacts**

Mirror the existing `menu` special-case branch with a `popout` branch so the command prints paths the executor can inspect:

```bash
if [[ "$capture_mode" == "popout" ]]; then
  state_json="${proof_dir}/${proof_name}-state-${date_stamp}.json"
  popout_png="${proof_dir}/${proof_name}-interactive-full-${date_stamp}.png"
  # fetch_guest_file ...
  echo "state_json=$state_json"
  echo "popout_full=$popout_png"
  exit 0
fi
```

- [ ] **Step 4: Update workflow docs only if the command surface changed materially**

If you add a new supported `CODEXBAR_PROOF_CAPTURE_MODE=popout` workflow, document the exact command in `docs/ui-parity-workflow.md`. Skip this step if the script change is self-evident and temporary.

- [ ] **Step 5: Run the existing script syntax/smoke checks that already exist in the repo**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar
bash -n scripts/run_vm_provider_proof.sh
```

Expected: no output

- [ ] **Step 6: Commit**

```bash
cd /home/fsos/Developer/Win-CodexBar
git add scripts/run_vm_provider_proof.sh scripts/vm/provider_osclick_proof_unc.ps1 docs/ui-parity-workflow.md
git commit -m "feat: add VM popout proof mode"
```

## Task 4: Run the Windows VM validation loop and patch only if proof fails

**Files:**
- Check: `scripts/run_vm_provider_proof.sh`
- Check: `/tmp/win-codexbar-settings/*`
- Modify if needed: `rust/src/native_ui/app.rs`
- Modify if needed: `rust/src/tray/manager.rs`
- Test if needed: `rust/src/native_ui/app.rs`
- Test if needed: `rust/src/tray/manager.rs`

- [ ] **Step 1: Run the full local validation baseline before touching VM behavior**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar/rust
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: PASS

- [ ] **Step 2: Prove the left-click popup path in the VM**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar
CODEXBAR_PROOF_CAPTURE_MODE=menu bash scripts/run_vm_provider_proof.sh codex 20260414 tray-popup-v1
```

Inspect the emitted `state_json=` and `menu_full=` artifacts.

Success criteria:
- app launched tray-first rather than as a normal foreground window
- popup is visible after the scripted tray-left-click path
- debug state reports popup mode (after Task 1 wiring)

- [ ] **Step 3: Prove the detached popout window in the VM**

Run:

```bash
cd /home/fsos/Developer/Win-CodexBar
CODEXBAR_PROOF_CAPTURE_MODE=popout bash scripts/run_vm_provider_proof.sh codex 20260414 tray-popout-v1
```

Inspect the emitted `state_json=` and `popout_full=` artifacts.

Success criteria:
- window is visible
- debug state reports `popout`
- screenshot shows a decorated/resizable detached window rather than the tray popup

- [ ] **Step 4: Manually verify the native right-click tray menu label in the Windows VM**

This check is manual on purpose: the current harness automates the egui popup, but the native tray context menu is OS-owned and not represented in the egui state dump.

Manual checklist:
1. Open the Windows VM interactively.
2. Right-click the Win-CodexBar tray icon.
3. Confirm the native context menu contains `Pop Out Dashboard`.
4. Capture a fresh screenshot from the VM/host and save it under `/tmp/win-codexbar-settings/`.

- [ ] **Step 5: If any check fails, patch only the failing behavior**

Patch guide:

- **Startup visible when tray is healthy:** fix `rust/src/native_ui/app.rs` around `start_visible_without_tray`.
- **Left-click opens the wrong window style:** force `self.is_popout_mode = false` in the `TrayLeftClick` path before layout.
- **Popout opens the popup style:** force `self.is_popout_mode = true` in the `PopOut` path before layout.
- **Menu label is wrong or missing:** fix the top-level `popout` menu item in `rust/src/tray/manager.rs`.

- [ ] **Step 6: Re-run only the impacted local tests after any patch**

Examples:

```bash
cd /home/fsos/Developer/Win-CodexBar/rust
cargo test test_tray_action_from_event_id_maps_top_level_popout -- --exact
cargo test debug_window_mode_reports_popup_and_popout -- --exact
cargo test forced_visible_startup_stays_in_popup_mode -- --exact
cargo test trayless_startup_uses_popout_mode_only_without_forced_visible -- --exact
```

Expected: PASS

- [ ] **Step 7: Re-run the affected VM proof(s) until the evidence is clean**

Use the same proof names with incremented suffixes:

```bash
CODEXBAR_PROOF_CAPTURE_MODE=menu bash scripts/run_vm_provider_proof.sh codex 20260414 tray-popup-v2
CODEXBAR_PROOF_CAPTURE_MODE=popout bash scripts/run_vm_provider_proof.sh codex 20260414 tray-popout-v2
```

- [ ] **Step 8: Commit the product fix if code changed**

Use the narrowest accurate message, for example:

```bash
cd /home/fsos/Developer/Win-CodexBar
git add rust/src/native_ui/app.rs rust/src/tray/manager.rs
git commit -m "fix: restore tray popup and popout behavior"
```

- [ ] **Step 9: Push and record the final proof artifact paths in the handoff**

Include:
- popup proof screenshot path
- popout proof screenshot path
- manual right-click menu screenshot path
- final `state_json` paths
