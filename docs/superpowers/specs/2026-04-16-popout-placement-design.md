# Detached Popout Placement Parity Design

## Problem

The detached popout/settings window in the Tauri desktop shell does not consistently open near the tray the way the old egui app does. The current behavior appears to treat detached popouts as a separate placement case with its own heuristic, which can drift away from tray-first parity on Windows.

## Goal

Make detached popout and settings windows follow the same tray-anchor placement model as the egui reference whenever tray context is available, while preserving a deterministic fallback when no tray anchor exists.

## Scope

In scope:

- Detached popout placement in the Tauri desktop shell
- Shared tray-anchor positioning logic between tray panel and detached popouts
- Pure positioning tests for tray-anchor, fallback, and work-area clamping behavior
- A shell-level check that detached popouts route through the shared placement path

Out of scope:

- New persisted window-position preferences
- Broader tray animation or tray icon behavior
- Unrelated window-management changes outside the detached popout/settings path

## Chosen Approach

Use a shared tray-anchor parity algorithm for detached popouts as well as the tray panel.

This is preferred over a popout-only heuristic tweak because it fixes the root cause of drift between surfaces. It also avoids a fragile one-off patch that would only cover the currently reported review case.

## Design

### 1. Positioning model

Detached popouts and settings windows should use the same tray-anchor rules as the tray-first UX when a tray icon rectangle is available:

- derive the anchor from the tray icon rectangle
- center horizontally on the tray parity anchor
- choose above vs below based on available work-area space
- clamp the final position inside the monitor work area

If no tray anchor exists, the app should use the existing explicit fallback path instead of trying to invent a tray-like position from incomplete context.

### 2. Code boundaries

The Tauri positioning module should remain the source of truth for window placement math.

- The shell decides which surface is opening and what context is available
- The positioner decides where that surface should open

Detached popout placement should route through the same shared tray-anchor helper used by tray-aligned surfaces, with only a thin wrapper for surface-specific size and fallback behavior. Settings windows should use the same detached-surface placement entry point as other detached popouts, not a separate placement algorithm. This keeps the positioning rules aligned without forcing all surfaces to share the same fallback behavior.

### 3. Data flow

The shared placement path should take only the inputs required to compute a tray-aligned position:

- tray icon rectangle, if known
- monitor work area
- surface size
- scale factor

Detached popout placement should not depend on extra persisted state or ad hoc “last position” rules when tray context exists, because those would undermine parity with the egui behavior.

### 4. Fallback and normalization

Failure handling should stay deterministic:

- if the tray icon rectangle is missing, keep the existing no-anchor fallback branch in the popout positioner instead of introducing a new inferred-tray heuristic for detached surfaces
- if the scale factor is invalid, normalize it the same way the current positioner already does
- if the computed position would fall off-screen, clamp it to the monitor work area

The fallback path should be clearly secondary to tray-anchor placement, not a competing heuristic.

## Testing and Validation

Add or preserve pure positioning coverage for:

- detached popout anchored near a bottom taskbar tray icon
- detached popout anchored near a top taskbar tray icon
- detached popout placement constrained by work-area limits
- detached popout fallback when no tray anchor exists

Add one shell-level verification that detached popout/settings placement now routes through the shared tray-anchor path instead of bypassing it with separate placement math.

Windows validation on Granet should confirm that detached popout/settings windows open next to the tray the same way the egui reference does, rather than drifting to a generic corner or heuristic location.

## Success Criteria

- Detached popout/settings windows open near the tray when tray anchor data exists
- Placement decisions match egui-style tray parity rules closely enough to remove the review complaint
- No-anchor behavior remains deterministic and visible on-screen
- Positioning logic stays centralized enough that tray panel and detached popouts cannot silently diverge again
