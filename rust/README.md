# CodexBar for Windows

A Windows port of [CodexBar](https://github.com/steipete/CodexBar) - a system tray application for monitoring AI provider usage limits.

![CodexBar Windows](screenshots/tray-icon.png)

## Features

- **System Tray Icon** - Color-coded usage indicator with incident badges
- **Multiple Providers** - Support for 12 AI providers:
  - Claude (Anthropic)
  - Codex (OpenAI)
  - Cursor
  - Gemini (Google)
  - Copilot (GitHub)
  - Antigravity
  - Windsurf (Factory/Codeium)
  - Zai
  - Kiro (AWS)
  - Vertex AI (Google Cloud)
  - Augment
  - MiniMax
- **Usage Notifications** - Windows toast alerts when usage hits thresholds
- **Settings Panel** - Enable/disable providers, configure refresh intervals
- **Manual Cookie Input** - Fallback for when automatic cookie extraction fails
- **Status Page Polling** - Shows provider incidents with visual badges
- **CLI Tool** - Command-line interface for scripts and automation

## Installation

### From Release

Download the latest release from the [Releases](https://github.com/Finesssee/Win-CodexBar/releases) page.

### Build from Source

Requirements:
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- Windows 10/11

```powershell
# Clone the repository
git clone https://github.com/Finesssee/Win-CodexBar.git
cd Win-CodexBar/rust

# Build release version
cargo build --release

# Run the CLI
./target/release/codexbar.exe --help

# Run the GUI (system tray)
./target/release/codexbar.exe menubar
```

## Usage

### GUI Mode (System Tray)

```powershell
codexbar menubar
```

This launches the system tray application:
- Click the tray icon to show the usage panel
- Use the Settings button to configure providers and refresh intervals
- Use the Cookies button for manual cookie input if needed
- Use the About button for version info

### CLI Mode

```powershell
# Show usage for all enabled providers
codexbar

# Show usage for specific provider
codexbar -p claude

# Show usage for all providers
codexbar -p all

# Output as JSON
codexbar --json --pretty

# Include provider status
codexbar --status

# Show local cost usage (Claude + Codex logs)
codexbar cost

# Enable/disable auto-start on Windows boot
codexbar autostart enable
codexbar autostart disable
codexbar autostart status
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Unexpected failure |
| 2 | Provider not installed |
| 3 | Parse error |
| 4 | Timeout |

## Configuration

Settings are stored in `%APPDATA%\CodexBar\settings.json`:

```json
{
  "enabled_providers": ["claude", "codex"],
  "refresh_interval_secs": 300,
  "show_notifications": true,
  "high_usage_threshold": 70.0,
  "critical_usage_threshold": 90.0
}
```

Manual cookies are stored separately in `%APPDATA%\CodexBar\manual_cookies.json`.

## Provider Authentication

Each provider has different authentication methods:

| Provider | Auth Method |
|----------|-------------|
| Claude | Browser cookies (Chrome/Edge), OAuth |
| Codex | Local CLI, Browser cookies |
| Cursor | Browser cookies |
| Gemini | gcloud CLI credentials |
| Copilot | GitHub device flow |
| Antigravity | Local language server |
| Windsurf | Browser cookies, local config |
| Zai | Local config |
| Kiro | AWS credentials |
| Vertex AI | gcloud OAuth |
| Augment | VS Code extension |
| MiniMax | API key |

## Screenshots

### System Tray
![Tray Icon](screenshots/tray-icon.png)

The tray icon color indicates overall usage:
- Green: 0-50% used
- Yellow: 50-80% used
- Orange: 80-95% used
- Red: 95-100% used
- Gray: Unknown/loading

A badge appears in the corner for status issues:
- Yellow badge: Degraded performance
- Red badge: Major outage

### Main Panel
![Main Panel](screenshots/main-panel.png)

### Settings
![Settings](screenshots/settings.png)

### About
![About](screenshots/about.png)

## Development

### Running Tests

```powershell
cargo test
```

### Project Structure

```
rust/
├── src/
│   ├── main.rs           # Entry point
│   ├── cli/              # CLI commands
│   ├── core/             # Core data models
│   ├── providers/        # Provider implementations
│   ├── browser/          # Cookie extraction
│   ├── tauri_app/        # Tauri GUI
│   ├── tray/             # Tray icon types
│   ├── settings.rs       # Settings management
│   ├── status.rs         # Status page polling
│   └── notifications.rs  # Windows notifications
├── ui/                   # HTML/CSS/JS for GUI
│   ├── index.html        # Main panel
│   ├── settings.html     # Settings page
│   ├── about.html        # About dialog
│   └── cookies.html      # Cookie input
└── Cargo.toml
```

## Credits

This is a Windows port of [CodexBar](https://github.com/steipete/CodexBar) by [Peter Steinberger](https://twitter.com/steipete).

## License

MIT
