# Building from Source

## Prerequisites

- **Rust** 1.70+ with `x86_64-pc-windows-gnu` target
- **MinGW-w64** (for the GNU linker)
- **Node.js** 18+ and npm

Install prerequisites automatically:
```powershell
.\scripts\setup-windows.ps1
```

## Build the Desktop App

```powershell
cd apps/desktop-tauri
npm install
cd ../..
npm --prefix apps/desktop-tauri run tauri:build
```

The release binary lands at `target/release/codexbar-desktop-tauri.exe`.

For a debug build (faster compile, no optimisations):
```powershell
cd apps/desktop-tauri
npm run tauri:build:debug
```

## Build the CLI Only

```powershell
cargo build -p codexbar --release
# Binary at: target/release/codexbar.exe
```

## Dev Mode (Hot Reload)

```powershell
.\dev.ps1                   # default debug build + launch
.\dev.ps1 -Release          # optimised build
.\dev.ps1 -Verbose          # debug logging
.\dev.ps1 -SkipBuild        # run last build without rebuilding
```

Or directly:
```powershell
cd apps/desktop-tauri && npm run tauri:dev
```

## Project Structure

```
Win-CodexBar/
├── apps/desktop-tauri/          # Tauri desktop shell
│   ├── src/                     # React frontend (TypeScript)
│   └── src-tauri/               # Tauri/Rust backend
│       └── src/
│           ├── commands/        # Tauri IPC commands
│           ├── shell/           # Window management, DWM, tray bridge
│           └── main.rs          # App entry point
├── rust/                        # Shared backend crate + CLI
│   └── src/
│       ├── providers/           # Per-provider fetch/parse/auth
│       ├── core/                # Provider IDs, cost pricing
│       ├── browser/             # Browser cookie extraction (DPAPI)
│       ├── tray/                # Tray icon rendering
│       └── main.rs              # CLI entry point
├── extra-docs/                  # Additional documentation
├── dev.ps1                      # Windows dev launcher
└── dev.sh                       # WSL/Linux dev launcher
```

## Running Tests

```bash
# Shared crate tests
cargo test --manifest-path rust/Cargo.toml

# Tauri crate tests
cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml

# TypeScript type check
cd apps/desktop-tauri && npx tsc --noEmit

# Lint
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```
