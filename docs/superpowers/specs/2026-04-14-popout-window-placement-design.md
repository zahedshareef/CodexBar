# Popout window placement near tray design

## Problem

`Pop Out Dashboard` currently opens as a normal window, but its default placement is wrong in the Windows VM. Instead of landing near the tray area, it follows the generic pointer-anchored path and can appear off to the left side of the monitor.

The desired behavior is:

1. Keep the detached popout as a normal decorated window.
2. Open it near the tray area by default.
3. Prefer a bottom-right placement above the taskbar, aligned near the tray icon.

## Current context

- `rust/src/native_ui/app.rs` uses `layout_main_window()` for both popup and popout placement.
- Tray popup mode already supports tray anchoring through `tray_anchor_pos`.
- Popout mode currently enters through `activate_popout_mode()` and uses `anchor_main_window_to_pointer = true`, which drives the non-tray fallback branch.
- That fallback intentionally places the window on the left side of the monitor for keyboard-style opens, which is appropriate for popup flows but not for detached tray popouts.

## Scope

- Change only the default placement behavior for tray-triggered popouts.
- Keep popup placement behavior unchanged.
- Keep popout chrome behavior unchanged:
  - decorated
  - resizable
  - normal window level
- Do not add position persistence in this pass.

## Options considered

### Option 1: Tray-anchored popout

When `Pop Out Dashboard` is triggered, resolve the tray icon position and place the detached window near that anchor, using a bottom-right-above-taskbar default.

**Pros**
- Matches the expected tray utility behavior
- Reuses the existing tray anchor concept
- Keeps popup and popout spatially related

**Cons**
- Needs a tray-unavailable fallback

### Option 2: Fixed bottom-right work-area placement

Always place the popout at a fixed inset from the monitor work area, without using the tray rect.

**Pros**
- Simple
- Predictable on a normal bottom taskbar

**Cons**
- Less accurate for non-standard taskbar placement
- Ignores the real tray location when it is available

### Option 3: Remember last popout position

Use tray-based placement only on first open, then reopen at the last moved location.

**Pros**
- Strong long-term desktop UX

**Cons**
- Adds persistence and extra state
- Out of scope for this fix

## Chosen design

Use **Option 1**.

### Placement rules

1. `Pop Out Dashboard` should prefer the tray icon rect when it is available.
2. The detached window should open near the tray area, biased to the bottom-right above the taskbar.
3. When popout is triggered from the tray menu, the app must resolve the tray rect at that moment and feed it into placement. The generic left-side pointer-anchor path must not be used for tray-triggered popouts.
4. The detached window should remain clamped to the work area of the monitor that contains the application window, not blindly to the primary monitor work area.
5. If the tray rect is unavailable, popout should fall back to a sane bottom-right work-area position on that same monitor instead of the current left-side pointer-anchor position.
6. Tray anchoring should still adapt to non-bottom taskbar placements; the "bottom-right above the taskbar" behavior is the normal bottom-taskbar preference, not a hard-coded assumption.

### Behavioral boundaries

- Tray left-click popup continues to use the existing popup placement path.
- Keyboard/test-only pointer anchoring remains unchanged for popup-oriented flows.
- This pass does not change menu items, startup behavior, or persistence.

## Implementation outline

- Split popout placement from the generic `anchor_to_pointer` behavior.
- Route `TrayMenuAction::PopOut` through a popout placement path that:
  - resolves and captures the tray anchor when available
  - computes a tray-adjacent bottom-right placement
  - falls back to bottom-right work-area placement on the window's current monitor if no tray anchor exists
  - uses the monitor containing the current application window as the source of truth for work-area bounds on Windows
- Keep the existing viewport chrome commands for popout mode.

## Error handling

- If tray coordinates are unavailable, use fallback placement instead of failing open.
- If monitor-aware work-area lookup fails, fall back to the existing broad work-area logic rather than blocking the window from opening.
- If the requested placement would be off-screen, clamp to the work area as today.

## Testing

- Keep existing Rust tests green.
- Add or adjust focused tests only if placement logic is extracted into testable helpers.
- Re-validate in the Windows VM:
  1. popup still opens near the tray
  2. popout opens as a normal window
  3. popout lands near the tray area instead of the left side of the monitor
