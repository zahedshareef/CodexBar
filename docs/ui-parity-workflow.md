# UI Parity Workflow

Use this workflow when matching the Windows Rust UI to the macOS CodexBar app.

## Goal

For each provider/settings UI pass:

1. Check the upstream macOS source.
2. Check the live macOS app behavior.
3. Apply the Windows change.
4. Verify in the Windows VM with fresh proof captures.

Do not rely on only one of those sources.

## Source Of Truth Order

Use these in order:

1. Live macOS CodexBar app on `imac-ca-mac`
2. Upstream temp repo clone at `/tmp/steipete-CodexBar`
3. Windows Rust implementation in this repo

If the live app and upstream source disagree, trust the live app first and record the mismatch in the thread.

## Reference Inputs

### Upstream temp repo

- Path: `/tmp/steipete-CodexBar`
- Remote: `https://github.com/steipete/CodexBar.git`
- Current known commit during this parity work: `21d2eed`

Useful files:

- `Sources/CodexBar/PreferencesProviderDetailView.swift`
- `Sources/CodexBar/PreferencesProviderSettingsRows.swift`
- `Sources/CodexBar/PreferencesProvidersPane.swift`
- provider-specific implementation files under `Sources/CodexBar/Providers/*`

### Live macOS app

- Machine: `mac@imac-ca-mac`
- Requirement: prefer read-only inspection
- Avoid stealing foreground input from the shared machine user
- Allowed:
  - process checks
  - accessibility tree dumps
  - non-interactive window/process inspection
- Avoid by default:
  - pointer movement
  - focus changes
  - keystroke injection

Useful checks:

```bash
ssh mac@imac-ca-mac "pgrep -af CodexBar || true"
ssh mac@imac-ca-mac "ps -p <pid> -o pid=,ppid=,comm=,command="
```

If a provider pane needs to be confirmed from the live app, use accessibility dumps or other non-invasive inspection before changing Windows UI.

## Windows VM Proof Loop

### Shared repo mirror

Always sync local changes to the Mac-hosted share before VM proof runs:

```bash
rsync -az --delete \
  --exclude '.git' \
  --exclude 'rust/target' \
  /home/fsos/Developer/Win-CodexBar/ \
  mac@imac-ca-mac:/Users/mac/codexbar-share/repo/
```

Do not assume the VM is using the local Linux checkout directly.

### Proof scripts

Current helper scripts in this repo:

- `scripts/run_vm_provider_proof.sh`
- `scripts/fetch_vm_preferences_proof.sh`
- `scripts/crop_vm_preferences_proof.sh`
- `scripts/vm/run_provider_proof_remote.sh`
- `scripts/vm/provider_osclick_proof_unc.ps1`

The generic runner is the preferred path now. Avoid reintroducing provider-
specific temporary wrappers unless a provider truly needs a separate path.

Use `scripts/run_vm_provider_proof.sh` for repeatable runs. It serializes
proof jobs with a local lock so concurrent `rsync --delete` mirror syncs do not
stomp each other.

By default, provider proof runs are cached builds. The host sync excludes
`rust/target`, the guest mirror excludes both the source and destination
`rust\target` paths, and the guest script preserves built Rust artifacts so
repeated Settings proofs do not pay a cold compile every time.

Use a clean proof build only when the cache itself is suspect, such as a stale
binary symptom, a linker-lock recovery pass, or a toolchain/config change:

```bash
CODEXBAR_PROOF_CLEAN_BUILD=1 scripts/run_vm_provider_proof.sh <provider>
```

### Known failure mode

The guest PowerShell screenshot path is currently not reliable.

Observed failure:

- `CopyFromScreen` throws `The handle is invalid`

Treat the guest screenshot as unproven unless the image file is actually produced and inspected.

### Reliable capture path

Use host-side Parallels capture after the guest run:

```bash
ssh mac@imac-ca-mac '"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl" capture "Windows 11" --file /Users/mac/codexbar-share/<name>.png'
scp mac@imac-ca-mac:/Users/mac/codexbar-share/<name>.png /tmp/win-codexbar-settings/<name>.png
```

Run multiple retries. A single host capture can be black or partially stale.
Prefer the reusable wrapper in `scripts/run_vm_provider_proof.sh`, which now:

- retries multiple host captures
- rejects dark or half-black captures with a brightness threshold
- falls back only when no bright capture is available

### Cropped preferences proof fallback

If the in-app preferences screenshot file is missing, first prefer the
guest-side interactive full-desktop capture, then crop it using the guest
state dump. Only fall back to host VM capture when the guest interactive
capture is missing or black.

```bash
/home/fsos/Developer/Win-CodexBar/scripts/fetch_vm_preferences_proof.sh \
  <provider> \
  /tmp/win-codexbar-settings/<host-capture>.png \
  /tmp/win-codexbar-settings/<provider>-state-YYYYMMDD.json \
  /tmp/win-codexbar-settings/<provider>-preferences-crop-YYYYMMDD.png
```

This helper now:

- fetches `C:\Users\mac\Desktop\<provider>-state.json`
- tries to fetch `C:\Users\mac\Desktop\<provider>-interactive-full.png`
- prefers that guest interactive screenshot when it contains real pixels
- otherwise falls back to the supplied host Parallels capture
- crops the Settings window from whichever full-screen source is usable

This is currently the most reliable way to get a fresh standalone Preferences
window image from the Windows VM.

Important freshness rule:

- do not trust an existing `*-ready.txt` or `*-state.json` on the guest Desktop
  unless its timestamp is from the current run
- stale Desktop artifacts can make a proof look valid while still showing an old
  UI build
- the guest proof script now deletes old `*-ready.txt`, `*-state.json`,
  `*-preferences-proof.png`, and related per-provider artifacts at startup
- if the wrapper is lagging or deadlocked, clear stale local lock state,
  wait for new guest timestamps, then salvage with a manual host capture plus
  `fetch_vm_preferences_proof.sh`

Important failure mode we hit:

- the debug test server reads the full TCP connection before queuing commands
- do not stream a long series of commands over one persistent socket and expect
  incremental behavior
- send each test command over its own short-lived TCP connection from the guest
  proof script so state dumps, clicks, and screenshot requests happen in order

### Current screenshot runtime finding

For `eframe 0.30` in this repo, the secondary Settings viewport accepts
`ViewportCommand::Screenshot` requests but does not emit any matching
`Event::Screenshot` back to the app in this environment.

Evidence path:

- guest debug log: `C:\Users\mac\AppData\Local\Temp\codexbar_preferences_screenshot.log`
- repeated `send_*_viewport_screenshot` entries appear
- no `event_received` entries ever appear

Treat the secondary-viewport screenshot path as runtime-broken until upstream
behavior or app architecture changes.

## Proof Acceptance Rules

Only count a proof as valid when all of these are true:

1. The repo was synced to `/Users/mac/codexbar-share/repo/` immediately before the run.
2. The VM build completed for the current pass.
3. The resulting screenshot is fresh and non-black.
4. The selected provider in Settings matches the intended provider.
5. The screenshot visibly reflects the specific UI change being claimed.

## Artifact Naming

Use provider-specific names under `/tmp/win-codexbar-settings/`:

- `claude-proof-host-YYYYMMDD-retryN.png`
- `cursor-proof-host-YYYYMMDD-retryN.png`
- `codex-proof-host-YYYYMMDD-retryN.png`
- `kiro-proof-host-YYYYMMDD-autoN.png`

Prefer the brightest valid image from a retry batch. Use file size only as a tiebreaker.

## Per-Pass Checklist

For every provider UI pass:

1. Read the matching upstream Swift files in `/tmp/steipete-CodexBar`.
2. Confirm the live macOS app behavior when the detail is ambiguous.
3. Edit the Windows Rust UI.
4. Run targeted Rust tests for the changed constants or provider behavior.
5. Run `cargo fmt --all --manifest-path rust/Cargo.toml`.
6. Sync to the shared Mac host repo mirror.
7. Run the VM proof script.
8. If guest screenshot fails, run host-side Parallels capture retries.
9. Inspect the newest non-black proof image before claiming progress.

## Current Working Rule

For this repo, do not claim provider UI parity from source inspection alone.

A pass needs:

- upstream source reference
- live macOS behavior reference when needed
- fresh Windows VM proof
