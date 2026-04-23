# Win-CodexBar

[简体中文说明](./README.zh-CN.md)

The Windows port of [CodexBar](https://github.com/steipete/CodexBar) — a system tray app that keeps your AI coding-tool usage limits visible at a glance.

> Built with **Tauri + React** on a shared **Rust** backend. The original CodexBar is a macOS Swift app by [Peter Steinberger](https://github.com/steipete).

<p align="center">
  <img src="extra-docs/images/tray-panel.png" width="280" alt="Tray panel showing provider grid and Codex usage"/>
  &nbsp;&nbsp;
  <img src="extra-docs/images/settings-providers.png" width="480" alt="Settings — Providers tab"/>
</p>

## Features

- **28 AI providers** — Codex, Claude, Cursor, Factory, Gemini, Copilot, Antigravity, z.ai, MiniMax, Kiro, Vertex AI, Augment, OpenCode, Kimi, Kimi K2, Amp, Warp, Ollama, OpenRouter, Synthetic, JetBrains AI, Alibaba, NanoGPT, Infini, Perplexity, Abacus AI, OpenCode Go, Kilo
- **System tray icon** — dynamic two-bar meter showing session + weekly usage
- **Browser cookie import** — Chrome, Edge, Brave, Firefox (DPAPI decryption on Windows)
- **Per-provider credentials** — API keys, cookies, and OAuth all managed from the provider detail pane
- **CLI** — `codexbar usage` and `codexbar cost` for scripting and CI
- **WSL support** — CLI works out of the box; desktop shell via WSLg

## Quick Start

```powershell
# Prerequisites: Node.js — Rust and MinGW are installed automatically
git clone https://github.com/Finesssee/Win-CodexBar.git
cd Win-CodexBar
.\dev.ps1
```

The script installs Rust/MinGW if needed, builds the Tauri desktop shell, and launches the app.

```powershell
.\dev.ps1 -Release          # optimised build
.\dev.ps1 -SkipBuild        # relaunch last build
```

## Download

Grab the latest build from [GitHub Releases](https://github.com/Finesssee/Win-CodexBar/releases).

- **Installer**: `CodexBar-<version>-Setup.exe`
- **Portable**: `codexbar-desktop-tauri.exe`

## First Run

1. Launch CodexBar — it sits in the system tray
2. Click the tray icon to open the usage panel
3. Open **Settings → Providers**, enable the services you use
4. For cookie-based providers, click the provider and use **Browser Cookies → Import**
5. For CLI-based providers (`codex`, `claude`, `gemini`), make sure you're logged in

## CLI

```bash
codexbar usage -p claude          # single provider
codexbar usage -p all             # all enabled providers
codexbar cost  -p codex           # local cost from JSONL logs
```

## Providers

| Provider | Auth | Tracks |
|----------|------|--------|
| Codex | OAuth / CLI | Session, Weekly, Credits |
| Claude | OAuth / Cookies / CLI | Session (5h), Weekly |
| Cursor | Cookies | Plan, Usage, Billing |
| Factory | Cookies | Usage |
| Gemini | gcloud OAuth | Quota |
| Copilot | GitHub Device Flow | Usage |
| Antigravity | Cookies / LSP | Usage |
| z.ai | API Token | Quota |
| MiniMax | API / Cookies | Usage |
| Kiro | Cookies / CLI | Monthly Credits |
| Vertex AI | gcloud OAuth | Cost |
| Augment | Cookies | Credits |
| OpenCode | Local Config | Usage |
| Kimi | Cookies | 5h Rate, Weekly |
| Kimi K2 | API Key | Credits |
| Amp | Cookies | Usage |
| Warp | Local Config | Usage |
| Ollama | Cookies | Usage |
| OpenRouter | API Key | Credits |
| JetBrains AI | Local Config | Usage |
| Alibaba | Cookies | Usage |
| NanoGPT | API Key | Credits |
| Infini | API Key | Session, Weekly, Quota |
| Perplexity | Cookies | Credits, Plan |
| Abacus AI | Cookies | Credits |
| OpenCode Go | Cookies | Usage |
| Kilo | API Key / CLI | Usage |

## Privacy

- **On-device only** — no data sent anywhere except provider APIs
- **No disk scanning** — only reads known config paths and browser cookies
- **Opt-in cookies** — extraction only runs for providers you enable

## More Docs

| Topic | Link |
|-------|------|
| Building from source | [extra-docs/BUILDING.md](extra-docs/BUILDING.md) |
| WSL setup & auth tips | [extra-docs/WSL.md](extra-docs/WSL.md) |
| Browser cookie details | [extra-docs/COOKIES.md](extra-docs/COOKIES.md) |

## Credits

- **Original CodexBar**: [steipete/CodexBar](https://github.com/steipete/CodexBar) by Peter Steinberger
- **Inspired by**: [ccusage](https://github.com/ryoppippi/ccusage) for cost tracking

## License

MIT — same as the original CodexBar.

---

*For the macOS version, visit [steipete/CodexBar](https://github.com/steipete/CodexBar).*
