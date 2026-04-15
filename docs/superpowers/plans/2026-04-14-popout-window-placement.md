# Popout Window Placement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make tray-triggered popouts open as normal windows near the tray area instead of falling into the current left-side pointer-anchor placement.

**Architecture:** Keep popup behavior unchanged, but split tray-triggered popout placement away from the generic pointer-anchor branch in `rust/src/native_ui/app.rs`. Both `TrayMenuAction::PopOut` and `TrayMenuAction::PopOutProvider(...)` should resolve tray placement from the tray manager when possible, then fall back to a bottom-right work-area placement on the window's current monitor.

**Tech Stack:** Rust, egui/eframe, Win32 monitor APIs via `windows`, existing tray manager/test server/debug-state plumbing

---

## File map

- `rust/src/native_ui/app.rs`
  - Owns popup/popout placement state, tray action handling, monitor work-area lookup, debug-only test hooks, and unit tests.
  - Primary file for this change.
- `rust/src/native_ui/test_server.rs`
  - Inspect only. `SimulateTrayPopOut` already exists; behavior should improve through `activate_popout_mode()` unless the implementation decides a new input is cleaner.
- `docs/superpowers/specs/2026-04-14-popout-window-placement-design.md`
  - Reference only while implementing; do not expand scope beyond the approved design.

## Task 1: Extract testable popout placement decisions

**Files:**
- Modify: `rust/src/native_ui/app.rs` (placement helpers near `layout_main_window()` and tests near the existing `#[cfg(test)]` block)
- Test: `rust/src/native_ui/app.rs`

- [ ] **Step 1: Write failing unit tests for popout placement selection**

```rust
#[test]
fn popout_prefers_tray_anchor_when_available() {
    let tray_anchor = Some(egui::pos2(1180.0, 740.0));
    let placement = compute_popout_target(tray_anchor, work_area(), size());
    assert!(placement.x > 700.0);
    assert!(placement.y > 300.0);
}

#[test]
fn popout_fallback_avoids_left_side_pointer_anchor() {
    let placement = compute_popout_target(None, work_area(), size());
    assert!(placement.x > 700.0);
}

#[test]
fn popout_tray_anchor_stays_clamped_inside_work_area() {
    let tray_anchor = Some(egui::pos2(1915.0, 1075.0));
    let placement = compute_popout_target(tray_anchor, work_area(), size());
    assert!(placement.x <= 1568.0);
}
```

- [ ] **Step 2: Run targeted tests to verify they fail**

Run:
- `cd rust && cargo test --lib popout_prefers_tray_anchor_when_available -- --nocapture`
- `cd rust && cargo test --lib popout_fallback_avoids_left_side_pointer_anchor -- --nocapture`
- `cd rust && cargo test --lib popout_tray_anchor_stays_clamped_inside_work_area -- --nocapture`

Expected: FAIL because the helper does not exist yet or still returns the old pointer-anchor behavior.

- [ ] **Step 3: Write the minimal helper layer**

```rust
fn compute_popout_target(
    tray_anchor: Option<egui::Pos2>,
    work_area: Rect,
    target_size: egui::Vec2,
) -> egui::Pos2 {
    if let Some(tray_pos) = tray_anchor {
        // tray-adjacent placement
    } else {
        // bottom-right fallback
    }
}
```

Keep it pure and small so it can be unit-tested without spinning up egui UI state.

- [ ] **Step 4: Re-run the targeted tests**

Run:
- `cd rust && cargo test --lib popout_prefers_tray_anchor_when_available -- --nocapture`
- `cd rust && cargo test --lib popout_fallback_avoids_left_side_pointer_anchor -- --nocapture`
- `cd rust && cargo test --lib popout_tray_anchor_stays_clamped_inside_work_area -- --nocapture`

Expected: PASS for the new placement tests.

- [ ] **Step 5: Commit**

```bash
git add rust/src/native_ui/app.rs
git commit -m "test: cover popout placement decisions"
```

## Task 2: Wire both tray popout actions to tray-aware placement

**Files:**
- Modify: `rust/src/native_ui/app.rs:1636-1643`
- Modify: `rust/src/native_ui/app.rs:2526-2527`
- Modify: `rust/src/native_ui/app.rs:2839-2848`
- Test: `rust/src/native_ui/app.rs`

- [ ] **Step 1: Write a failing regression test for tray-triggered popout prep**

```rust
#[test]
fn tray_popout_preparation_prefers_tray_rect_over_pointer_anchor() {
    let request = prepare_tray_popout_position(Some(tray_rect()));
    assert_eq!(request.anchor_source, AnchorSource::Tray);
}

#[test]
fn tray_popout_provider_uses_same_preparation_path() {
    let request = prepare_tray_popout_position(Some(tray_rect()));
    assert_eq!(request.mode, PopoutPlacementMode::TrayOrBottomRightFallback);
}
```

- [ ] **Step 2: Run the new regression tests**

Run:
- `cd rust && cargo test --lib tray_popout_preparation_prefers_tray_rect_over_pointer_anchor -- --nocapture`
- `cd rust && cargo test --lib tray_popout_provider_uses_same_preparation_path -- --nocapture`

Expected: FAIL until the preparation path is introduced.

- [ ] **Step 3: Implement the tray-aware wiring**

```rust
fn activate_popout_mode(&mut self, ctx: &egui::Context) {
    self.is_popout_mode = true;
    self.prepare_tray_popout_position();
    self.layout_main_window(ctx, false);
}
```

Implementation requirements:
- resolve `tray_manager.rect()` at popout time
- set `self.tray_anchor_pos` when available
- ensure tray-triggered popouts do **not** set `anchor_main_window_to_pointer = true`
- apply the same preparation path to `TrayMenuAction::PopOutProvider(...)`
- keep `SimulateTrayPopOut` going through the same path as real popout behavior

- [ ] **Step 4: Re-run the targeted regression tests**

Run:
- `cd rust && cargo test --lib tray_popout_preparation_prefers_tray_rect_over_pointer_anchor -- --nocapture`
- `cd rust && cargo test --lib tray_popout_provider_uses_same_preparation_path -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/src/native_ui/app.rs
git commit -m "fix: anchor tray popouts near tray"
```

## Task 3: Make work-area fallback monitor-aware on Windows

**Files:**
- Modify: `rust/src/native_ui/app.rs:2280-2319`
- Test: `rust/src/native_ui/app.rs`

- [ ] **Step 1: Write failing tests around fallback work-area selection**

```rust
#[test]
fn popout_fallback_uses_bottom_right_in_work_area() {
    let area = Rect::from_min_max(pos2(1000.0, 0.0), pos2(2000.0, 1000.0));
    let placement = compute_popout_target(None, area, size());
    assert!(placement.x >= 1600.0);
}

#[test]
fn popout_tray_anchor_supports_top_taskbar_layouts() {
    let tray_anchor = Some(pos2(1800.0, 40.0));
    let placement = compute_popout_target(tray_anchor, work_area(), size());
    assert!(placement.y > 40.0);
}
```

If direct Win32 monitor lookup is not unit-testable on Linux, keep the test on the pure placement math and separately validate the Windows lookup by code path and VM proof.

- [ ] **Step 2: Run the targeted tests**

Run:
- `cd rust && cargo test --lib popout_fallback_uses_bottom_right_in_work_area -- --nocapture`
- `cd rust && cargo test --lib popout_tray_anchor_supports_top_taskbar_layouts -- --nocapture`

Expected: FAIL until the helper/math is wired correctly.

- [ ] **Step 3: Implement monitor-aware work-area lookup**

```rust
#[cfg(target_os = "windows")]
fn work_area_rect(ctx: &egui::Context) -> Option<Rect> {
    // use MonitorFromWindow + GetMonitorInfoW for the app window
}
```

Implementation requirements:
- prefer the monitor containing the current app window
- fall back to the existing broad work-area logic if monitor lookup fails
- do not change popup placement rules beyond the monitor-awareness improvement

- [ ] **Step 4: Re-run targeted tests, then the full library suite**

Run:
- `cd rust && cargo test --lib popout_fallback_uses_bottom_right_in_work_area -- --nocapture`
- `cd rust && cargo test --lib popout_tray_anchor_supports_top_taskbar_layouts -- --nocapture`
- `cd rust && cargo test --lib`

Expected:
- targeted fallback test passes
- full library suite stays green

- [ ] **Step 5: Commit**

```bash
git add rust/src/native_ui/app.rs
git commit -m "fix: use monitor-aware popout fallback placement"
```

## Task 4: Validate the real VM behavior and close the loop

**Files:**
- Modify: `rust/src/native_ui/app.rs` only if proof exposes a remaining placement bug
- Reference: `docs/superpowers/specs/2026-04-14-popout-window-placement-design.md`

- [ ] **Step 1: Run focused local validation before VM work**

Run:
- `cd rust && cargo fmt --all`
- `cd rust && cargo clippy --all-targets -- -D warnings`
- `cd rust && cargo test --lib`

Expected: all commands pass.

- [ ] **Step 2: Rebuild and launch the app in the Windows VM**

Run the existing proven flow:

```bash
rsync -rlz --delete --exclude target --exclude .git ./ mac@imac-ca-mac:/Users/mac/codexbar-share/repo/
ssh mac@imac-ca-mac '/usr/local/bin/prlctl exec "Windows 11" --current-user powershell -Command "robocopy Z:\repo C:\Users\mac\Win-CodexBar /E /NFL /NDL /NJH /NJS /XD target .git"'
ssh mac@imac-ca-mac '/usr/local/bin/prlctl exec "Windows 11" --current-user powershell -Command "cd C:\Users\mac\Win-CodexBar\rust; cargo build"'
```

- [ ] **Step 3: Validate all four popout cases in the VM**

Check:
1. tray popup still opens near the tray
2. `Pop Out Dashboard` opens as a normal window near the tray area
3. provider-specific popout opens as a normal window near the tray area
4. neither popout path lands on the old left-side placement

Use the existing debug/test-server path plus screenshots/state dumps as needed.

- [ ] **Step 4: If VM proof required a final patch, apply the minimal fix and rerun validation**

Keep the patch scoped to placement only. Re-run the local checks from Step 1 and the VM validation from Step 3.

- [ ] **Step 5: Commit**

```bash
git add rust/src/native_ui/app.rs
git commit -m "fix: place popout windows near tray"
```

## Final handoff checklist

- [ ] `docs/superpowers/specs/2026-04-14-popout-window-placement-design.md` still matches shipped behavior
- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --lib` passes
- [ ] VM screenshots/state dumps clearly show corrected placement for both popout paths
