# Detached Popout Placement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make detached popout and settings windows open beside the tray using the same tray-anchor parity rules as the egui reference whenever tray anchor data exists, while preserving the existing deterministic no-anchor fallback.

**Architecture:** Keep `apps/desktop-tauri/src-tauri/src/window_positioner.rs` as the single source of truth for placement math. Refactor the anchored tray-placement math so `calculate_popout_position` and `calculate_panel_position` share one helper, then prove `apps/desktop-tauri/src-tauri/src/shell.rs` continues routing detached surfaces through that shared path and preserving the explicit no-anchor fallback.

**Tech Stack:** Rust, Tauri 2, workspace `cargo test`, Windows validation on Granet (`ssh granet-windows-vps`)

---

## File Map

- Modify: `apps/desktop-tauri/src-tauri/src/window_positioner.rs`
  - Owns the pure positioning math for tray panel, shortcut panel, and detached popout placement.
  - Add a shared anchored-placement helper here instead of duplicating tray-anchor math across surface types.
- Modify: `apps/desktop-tauri/src-tauri/src/shell.rs`
  - Owns the surface-mode routing and fallback selection for tray panel vs popout/settings windows.
  - Keep detached-surface routing thin: choose monitor/anchor context here, but leave placement math in `window_positioner.rs`.
- Reference only: `rust/src/native_ui/app.rs`
  - Egui parity reference for tray-anchored/popout placement behavior.
- Test via:
  - `cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml window_positioner`
  - `cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml visible_surface_position`
  - `cd apps/desktop-tauri && npm run build`
  - Windows host validation on Granet via `ssh granet-windows-vps`

### Task 1: Refactor the anchored placement math in `window_positioner.rs`

**Files:**
- Modify: `apps/desktop-tauri/src-tauri/src/window_positioner.rs`
- Test: `apps/desktop-tauri/src-tauri/src/window_positioner.rs`

- [ ] **Step 1: Write the failing test for shared anchored placement**

Add a focused unit test near the existing popout tests proving the anchored popout path matches the intended tray-anchor rules instead of drifting into a separate heuristic:

```rust
#[test]
fn anchored_popout_uses_same_tray_anchor_x_as_panel_positioning() {
    let icon = Rect { x: 1800, y: 1040, width: 24, height: 24 };

    let (panel_x, _) = calculate_panel_position(&icon, &standard_monitor(), &panel(), 1.0);
    let (popout_x, _) = calculate_popout_position(Some(&icon), &standard_monitor(), &panel(), 1.0);

    assert_eq!(popout_x, panel_x);
}
```

- [ ] **Step 2: Run the focused test to verify the current logic fails if parity is missing**

Run: `cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml anchored_popout_uses_same_tray_anchor_x_as_panel_positioning -- --exact`

Expected: either FAIL because the detached popout logic diverges, or PASS immediately and confirm the test still captures the intended parity before continuing with the refactor.

- [ ] **Step 3: Extract the shared anchored-placement helper**

Refactor the tray-anchor math so both tray panel and detached popout placement derive from one helper, while leaving shortcut placement untouched and keeping the explicit no-anchor branch in `calculate_popout_position`:

```rust
fn calculate_anchored_position(
    icon_rect: &Rect,
    monitor_rect: &Rect,
    panel_size: &PanelSize,
    scale_factor: f64,
    anchor_y: i32,
) -> (i32, i32) {
    let (pw, ph) = physical_panel_size(panel_size, scale_factor);
    let anchor_x = icon_rect.x + (icon_rect.width as i32) / 2;
    let target_x = anchor_x - pw / 2;
    let space_above = anchor_y - monitor_rect.y - MARGIN;
    let space_below =
        monitor_rect.y + monitor_rect.height as i32 - anchor_y - MARGIN;
    let target_y = if space_above >= ph + GAP || space_above > space_below {
        anchor_y - ph - GAP
    } else {
        anchor_y + GAP
    };

    clamp_to_work_area(target_x, target_y, monitor_rect, panel_size, scale_factor)
}
```

Use the tray icon’s top edge for detached popouts and preserve the existing panel-specific anchor behavior if that still differs intentionally.

- [ ] **Step 4: Add or adjust edge-case tests around the shared helper**

Keep the existing deterministic tests, and add coverage if needed for:

```rust
#[test]
fn anchored_popout_keeps_no_anchor_bottom_right_fallback() {
    let target = calculate_popout_position(None, &hd_monitor(), &panel(), 1.0);
    assert_eq!(target, (1492, 512));
}
```

Also preserve the top-taskbar, high-DPI, and clamp tests already in this file.

- [ ] **Step 5: Run the pure positioning tests**

Run: `cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml window_positioner -- --nocapture`

Expected: PASS with the full `window_positioner` test group green.

- [ ] **Step 6: Commit the refactor**

```bash
git add apps/desktop-tauri/src-tauri/src/window_positioner.rs
git commit -m "refactor: share anchored popout placement logic"
```

### Task 2: Prove detached surfaces route through the shared placement path

**Files:**
- Modify: `apps/desktop-tauri/src-tauri/src/shell.rs`
- Test: `apps/desktop-tauri/src-tauri/src/shell.rs`

- [ ] **Step 1: Write the failing shell-level test for detached surfaces**

Add a test near the existing `visible_surface_position_for_mode_with_fallbacks` coverage that proves both detached surface modes use the tray-anchor path when tray data exists:

```rust
#[test]
fn settings_surface_uses_tray_anchor_position_when_available() {
    let anchor_monitor = MonitorPlacement { /* reuse existing test fixture shape */ };
    let anchor = crate::state::TrayAnchor { x: 1800, y: 1040, width: 24, height: 24 };

    let position = visible_surface_position_for_mode_with_fallbacks(
        SurfaceMode::Settings,
        Some(&[anchor_monitor]),
        Some(anchor),
        Some(anchor_monitor),
        None,
        None,
    );

    assert_eq!(
        position,
        Some(window_positioner::calculate_popout_position(
            Some(&tray_anchor_rect(anchor)),
            &anchor_monitor.work_area,
            &surface_panel_size(SurfaceMode::Settings),
            anchor_monitor.scale_factor,
        ))
    );
}
```

- [ ] **Step 2: Run the focused shell test**

Run: `cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml settings_surface_uses_tray_anchor_position_when_available -- --exact`

Expected: FAIL until the detached-surface routing and assertions match the intended shared placement path.

- [ ] **Step 3: Keep the shell routing thin and explicit**

Only change `shell.rs` if the new test reveals drift or missing coverage. The final shape should continue to look like this:

```rust
if let Some(anchor) = tray_anchor
    && let Some(monitors) = monitor_placements
    && let Some(monitor) = monitor_placement_for_anchor(monitors, anchor)
{
    return Some(popout_position(
        Some(&tray_anchor_rect(anchor)),
        &monitor,
        &panel_size,
    ));
}
```

Do not add new inferred-tray behavior for detached popouts in the no-anchor branches; keep those branches using `popout_position(None, ...)`.

- [ ] **Step 4: Run the detached-surface shell tests**

Run: `cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml visible_surface_position -- --nocapture`

Expected: PASS with anchored and no-anchor detached-surface cases still green.

- [ ] **Step 5: Commit the shell/test update**

```bash
git add apps/desktop-tauri/src-tauri/src/shell.rs
git commit -m "test: cover detached surface tray anchor routing"
```

### Task 3: Full validation and Windows proof on Granet

**Files:**
- Modify: none expected unless validation finds a real defect
- Validate: `apps/desktop-tauri/src-tauri/src/window_positioner.rs`, `apps/desktop-tauri/src-tauri/src/shell.rs`

- [ ] **Step 1: Run the Tauri Rust test suite for the touched area**

Run: `cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml`

Expected: PASS for the Tauri crate tests.

- [ ] **Step 2: Run formatting and clippy expectations**

Run:

```bash
cargo fmt --all
cargo clippy --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: formatting applied cleanly and clippy passes without new warnings.

- [ ] **Step 3: Run the frontend production build**

Run: `cd apps/desktop-tauri && npm run build`

Expected: Vite build completes successfully.

- [ ] **Step 4: Run the same validation on Granet**

Run:

```bash
ssh granet-windows-vps 'cmd /c "cd /d C:\Users\Administrator\Win-CodexBar && cargo test --manifest-path apps\desktop-tauri\src-tauri\Cargo.toml"'
ssh granet-windows-vps 'cmd /c "cd /d C:\Users\Administrator\Win-CodexBar && cargo clippy --manifest-path apps\desktop-tauri\src-tauri\Cargo.toml --all-targets -- -D warnings"'
ssh granet-windows-vps 'cmd /c "cd /d C:\Users\Administrator\Win-CodexBar\apps\desktop-tauri && npm run build"'
```

Expected: all commands succeed on the Windows target machine.

- [ ] **Step 5: Manually verify detached popout placement behavior on Windows**

Use the Granet desktop session to launch the app with the repo’s Windows flow and confirm behavior:

```bash
ssh granet-windows-vps 'cmd /c "cd /d C:\Users\Administrator\Win-CodexBar && powershell -ExecutionPolicy Bypass -File .\dev.ps1"'
```

Then confirm:

- opening Settings or detached popout from tray context lands beside the tray, not in a generic corner
- no-anchor fallback still stays on-screen and deterministic
- top/bottom taskbar behavior still matches the egui-style parity rules

- [ ] **Step 6: Commit the final validation state**

```bash
git status --short
git commit --allow-empty -m "chore: validate detached popout placement on Windows"
```

Only use the empty validation commit if all implementation commits are already in place and you want a checkpoint. Skip this commit if the branch is already clean and the team does not want validation-only commits.
