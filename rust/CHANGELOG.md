# Changelog

All notable changes to Win-CodexBar will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [1.2.0] — 2026-03-18

### Added
- **Kilo Provider**: Full Kilo credits + Kilo Pass usage tracking via tRPC batch API. Supports KILO_API_KEY env var, Windows Credential Manager, and ~/.local/share/kilo/auth.json (from `kilo login`). Includes credit blocks, pass subscription, auto-top-up display, and plan tier labels.
- **Overview Tab**: Merged-menu Overview mode showing compact multi-provider usage rows. Select up to 3 providers in Settings → Display → Overview Tab. Remembers last-selected state across sessions.
- **Overview Provider Settings**: Preferences UI for selecting which providers appear in the Overview tab with provider-icon checkboxes and cap enforcement.

### Changed
- Provider count increased from 21 to 22 (added Kilo)
- Warp provider now checks WARP_TOKEN as fallback when WARP_API_KEY is not set (matches upstream)

### Fixed
- **Cursor usage limit**: Use `plan.limit` instead of `breakdown.total` for usage cap calculation (upstream fix #240). Prevents incorrect limit interpretation.
- **OpenRouter API URL**: Corrected base URL from `/api/v1/auth` to `/api/v1` — credits and key endpoints were hitting wrong paths.

---

## [1.0.1] — 2026-01-19

### Added

#### Swift Feature Port (Wave 1)
Complete porting of Swift CodexBar features to Rust Windows version:

- **Icon Morphing**: "Unbraid" animation from ribbons to usage bars
- **Model-Level Cost Breakdowns**: Per-model cost tracking on chart hover
- **Augment Session Keepalive**: Background cookie refresh before expiry
- **VertexAI Token Refresher**: OAuth token refresh with caching
- **MiniMax LocalStorage Import**: Browser localStorage session extraction
- **Web Probe Watchdog**: Process watchdog for browser automation
- **Usage Pace Prediction**: On Track/Ahead/Behind quota calculation with ETA
- **Personal Info Redaction**: Email address privacy protection for streaming
- **Copilot Device Flow OAuth**: GitHub Device Flow authentication
- **Zai MCP Details Submenu**: Per-model usage breakdown
- **OpenAI Deep Scraper**: React Fiber inspection for dashboard scraping
- **Provider-Specific Icon Twists**: Unique visual styles per provider
- **Eye Blink System**: Micro-motion animations with per-provider state
- **Command Runner**: Process execution with timeout and stop conditions
- **Token Account Multi-Support**: Multi-account token management with parallel fetching
- **Credential Migration System**: Windows credential format upgrades with version tracking
- **OpenAI Friendly Errors**: Human-readable Cloudflare/login/rate-limit detection
- **OpenCode Advanced Scraper**: Workspace ID resolution from JSON/HTML
- **Kiro CLI Version Detection**: Semver parsing with compatibility checks
- **Weekly Indicator Bars**: 4px progress bars in provider switcher tabs
- **Smart Menu Invalidation**: Version-based tracking prevents unnecessary rebuilds
- **Eye Blink Animation**: Random blinks with 18% double-blink probability
- **Icon Twist System**: Provider-specific visual styles (Claude crab, Gemini sparkle, etc.)
- **Provider Status Indicators**: Health overlays with Statuspage.io integration
- **Session Quota Notifications**: Depleted/restored state tracking with alerts
- **Cost Usage Pricing**: Model-specific token pricing (GPT-5, Claude Opus/Sonnet/Haiku)
- **JSONL Scanner**: Incremental log file parsing with file-level caching for Codex/Claude sessions
- **OpenAI Dashboard Models**: Usage breakdown and credits data structures
- **Cookie Header Cache**: Cookie normalization and caching with staleness tracking
- **Provider Fetch Plan**: Orchestrated fetching with strategy pipelines and fallback logic
- **Widget Snapshot**: Data export structures for external widget integrations
- **TTY Command Runner**: Windows-optimized command execution with ConPTY-style features

#### New Providers
- **Amp Provider**: Sourcegraph/Cody with API token support
- **Synthetic Provider**: Usage tracking support

#### UI Enhancements
- **API Keys Tab**: Provider access token configuration UI
- **Tab Icons**: Emoji icons in preference tabs
- **Tilt Animation**: New surprise animation
- **Unbraid Animation**: New loading animation pattern
- Preferences window now resizable
- Console window hides automatically in GUI mode

#### New Modules
`keepalive`, `token_refresher`, `local_storage`, `watchdog`, `usage_pace`, `redactor`, `icon_twist`, `blink`, `device_flow`, `mcp_details`, `scraper`, `command_runner`, `token_accounts`, `credential_migration`, `friendly_errors`, `version`, `weekly_indicator`, `menu_invalidation`, `indicators`, `session_quota`, `cost_pricing`, `jsonl_scanner`, `openai_dashboard`, `cookie_cache`, `fetch_plan`, `widget_snapshot`, `tty_runner`

### Changed
- Renamed "Zed AI" to "Zai" across entire codebase (display names, docs, comments)
- Build now uses GNU toolchain (`x86_64-pc-windows-gnu`) to avoid MSVC linker PATH conflicts
- Provider count increased from 15 to 18
- Refactored `status.rs` into `status/mod.rs` + `status/indicators.rs`

### Fixed
- Fixed 80 compiler warnings with targeted `#[allow(...)]` attributes

### Technical
- Added `ApiKeys` storage system
- Enhanced release profile (`opt-level=3`, `panic=abort`)
- All 156 tests passing

---

## [1.0.0] — 2025-01-17

### Added
- Initial Windows port of CodexBar using Rust + egui
- System tray integration with animated icons
- Support for 15 AI providers: Claude, Codex, Cursor, Gemini, Copilot, Antigravity, Windsurf, Zai, MiniMax, Kiro, Vertex AI, Augment, OpenCode, Kimi, Kimi K2
- Native Windows notifications via toast
- Browser cookie extraction (Chrome, Edge, Firefox, Brave)
- Keyboard shortcuts via global-hotkey
- Cost history charts with egui_plot
- CLI commands: `usage`, `cost`, `menubar`, `autostart`
- Windows installer via Inno Setup
- Auto-update checker
- Loading animations: Knight Rider, Cylon, OutsideIn, Race, Pulse
- Surprise animations: Blink, Wiggle, Pulse, Rainbow
- Provider status page integration
- Manual cookie paste support
- Preferences window with provider toggles

---

[Unreleased]: https://github.com/Finesssee/Win-CodexBar/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/Finesssee/Win-CodexBar/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/Finesssee/Win-CodexBar/releases/tag/v1.0.0
