# `rust/legacy/` — Retired egui menubar shell

This directory holds the original Windows-native egui menubar shell that
shipped before the Tauri desktop app became the default UI. The code is kept
compiled (via `#[path]` declarations in `rust/src/lib.rs` and
`rust/src/tray/mod.rs`) so existing tests and the public library API keep
working, but it is **no longer wired into any binary**.

## What lives here

| Path                          | Purpose (egui shell only)                                   |
| ----------------------------- | ----------------------------------------------------------- |
| `native_ui/`                  | egui app, charts, preferences pane, theme, in-process test server |
| `tray/blink.rs`               | Eye-blink animation system for the legacy tray icon         |
| `tray/icon_twist.rs`          | Whimsical icon decoration / personality engine              |
| `tray/manager.rs`             | egui-driven `TrayManager` / `MultiTrayManager` / `UnifiedTrayManager` |
| `tray/menu_invalidation.rs`   | Smart context-menu rebuild tracker                          |
| `tray/weekly_indicator.rs`    | Weekly progress mini-bars for the provider switcher         |
| `single_instance.rs`          | Mutex-based single-instance guard used by the legacy launcher |

## What is **not** legacy

- `rust/src/tray/icon.rs` and `rust/src/tray/render.rs` stay in the shared
  crate. The Tauri shell calls `codexbar::tray::render_bar_icon_rgba` to
  generate tray-icon RGBA bytes.
- All providers, settings, login, status, sound, and CLI logic remain in
  `rust/src/`.

## Where the live UI lives now

- Tauri desktop shell: `apps/desktop-tauri/`
- Tauri Rust backend & tray bridge: `apps/desktop-tauri/src-tauri/src/`

## Removing this directory in the future

When the legacy shell is no longer useful as reference material:

1. Drop the `#[path = "../legacy/..."]` declarations in
   `rust/src/lib.rs` and `rust/src/tray/mod.rs`.
2. Delete this directory.
3. Audit any remaining references to `codexbar::native_ui`,
   `codexbar::single_instance`, or the egui-only tray submodules
   (`blink`, `icon_twist`, `manager`, `menu_invalidation`,
   `weekly_indicator`) and remove them.
