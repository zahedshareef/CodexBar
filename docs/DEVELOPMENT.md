# CodexBar Development Guide

## Quick Start

### Building and Running

```bash
# Full build, test, package, and launch (recommended)
./Scripts/compile_and_run.sh

# Just build and package (no tests)
./Scripts/package_app.sh

# Launch existing app (no rebuild)
./Scripts/launch.sh
```

### Development Workflow

1. **Make code changes** in `Sources/CodexBar/`
2. **Run** `./Scripts/compile_and_run.sh` to rebuild and launch
3. **Check logs** in Console.app (filter by "codexbar")

## Keychain Prompts (Development)

### First Launch After Fresh Clone
You'll see **one keychain prompt per stored credential** on the first launch. This is a **one-time migration** that converts existing keychain items to use `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`.

### Subsequent Rebuilds
**Zero prompts!** The migration flag is stored in UserDefaults, so future rebuilds won't prompt.

### Why This Happens
- Ad-hoc signed development builds change code signature on every rebuild
- macOS keychain normally prompts when signature changes
- We use `ThisDeviceOnly` accessibility to prevent prompts
- Migration runs once to convert any existing items

### Reset Migration (Testing)
```bash
defaults delete com.steipete.codexbar KeychainMigrationV1Completed
```

## Auto-Refresh for Augment Cookies

### How It Works
CodexBar automatically refreshes Augment cookies from your browser:

1. **Automatic Import**: On every usage refresh, CodexBar imports fresh cookies from your browser
2. **Browser Priority**: Chrome → Arc → Safari → Firefox → Brave (configurable)
3. **Session Detection**: Looks for Auth0/NextAuth session cookies
4. **Fallback**: If import fails, uses last known good cookies from keychain

### Refresh Frequency
- Default: Every 5 minutes (configurable in Preferences → General)
- Minimum: 30 seconds
- Cookie import happens automatically on each refresh

### Supported Browsers
- Chrome
- Arc
- Safari
- Firefox
- Brave
- Edge

### Manual Cookie Override
If automatic import fails:
1. Open Preferences → Providers → Augment
2. Change "Cookie source" to "Manual"
3. Paste cookie header from browser DevTools

## Project Structure

```
CodexBar/
├── Sources/CodexBar/          # Main app (SwiftUI + AppKit)
│   ├── CodexBarApp.swift      # App entry point
│   ├── StatusItemController.swift  # Menu bar icon
│   ├── UsageStore.swift       # Usage data management
│   ├── SettingsStore.swift    # User preferences
│   ├── Providers/             # Provider-specific code
│   │   ├── Augment/           # Augment Code integration
│   │   ├── Claude/            # Anthropic Claude
│   │   ├── Codex/             # OpenAI Codex
│   │   └── ...
│   └── KeychainMigration.swift  # One-time keychain migration
├── Sources/CodexBarCore/      # Shared business logic
├── Tests/CodexBarTests/       # XCTest suite
└── Scripts/                   # Build and packaging scripts
```

## Common Tasks

### Add a New Provider
1. Create `Sources/CodexBar/Providers/YourProvider/`
2. Implement `ProviderImplementation` protocol
3. Add to `ProviderRegistry.swift`
4. Add icon to `Resources/ProviderIcon-yourprovider.svg`

### Debug Cookie Issues
```bash
# Enable verbose logging
export CODEXBAR_LOG_LEVEL=debug
./Scripts/compile_and_run.sh

# Check logs in Console.app
# Filter: subsystem:com.steipete.codexbar category:augment-cookie
```

### Run Tests Only
```bash
swift test
```

### Format Code
```bash
swiftformat Sources Tests
swiftlint --strict
```

## Distribution

### Local Development Build
```bash
./Scripts/package_app.sh
# Creates: CodexBar.app (ad-hoc signed)
```

### Release Build (Notarized)
```bash
./Scripts/sign-and-notarize.sh
# Creates: CodexBar-arm64.zip (notarized for distribution)
```

See `docs/RELEASING.md` for full release process.

## Troubleshooting

### App Won't Launch
```bash
# Check crash logs
ls -lt ~/Library/Logs/DiagnosticReports/CodexBar* | head -5

# Check Console.app for errors
# Filter: process:CodexBar
```

### Keychain Prompts Keep Appearing
```bash
# Verify migration completed
defaults read com.steipete.codexbar KeychainMigrationV1Completed
# Should output: 1

# Check migration logs
log show --predicate 'category == "KeychainMigration"' --last 5m
```

### Cookies Not Refreshing
1. Check browser is supported (Chrome, Arc, Safari, Firefox, Brave)
2. Verify you're logged into Augment in that browser
3. Check Preferences → Providers → Augment → Cookie source is "Automatic"
4. Enable debug logging and check Console.app

## Architecture Notes

### Menu Bar App Pattern
- No dock icon (LSUIElement = true)
- Status item only (NSStatusBar)
- SwiftUI for preferences, AppKit for menu
- Hidden 1×1 window keeps SwiftUI lifecycle alive

### Cookie Management
- Automatic browser import via SweetCookieKit
- Keychain storage for persistence
- Manual override for debugging
- Auto-refresh on every usage poll

### Usage Polling
- Background timer (configurable frequency)
- Parallel provider fetches
- Exponential backoff on errors
- Widget snapshot for iOS widget
