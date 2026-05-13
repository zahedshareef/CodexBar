# Release Trust

CodexBar for Windows publishes two Windows assets per release:

- `CodexBar-<version>-Setup.exe`
- `CodexBar-<version>-portable.exe`

Each asset is paired with a `.sha256` file generated after the final binary is
built and, when configured, code signed.

## Local Release Build

Run from the repository root on Windows:

```powershell
powershell -ExecutionPolicy Bypass `
  -File .\scripts\build-windows-release-assets.ps1
```

The script writes assets to `rust\target\release-assets`, verifies Microsoft
signatures on downloaded WebView2 and Visual C++ bootstrap dependencies, builds
the Inno Setup installer, and writes SHA-256 checksum files.

## Optional Code Signing

Code signing is opt-in so contributors can build unsigned local artifacts. To
require signing for a trusted release build:

```powershell
$env:WINDOWS_SIGNING_CERT_PATH = "C:\secure\codexbar-signing.pfx"
$env:WINDOWS_SIGNING_CERT_PASSWORD = "<pfx-password>"

powershell -ExecutionPolicy Bypass `
  -File .\scripts\build-windows-release-assets.ps1 `
  -RequireSigning
```

The script signs:

- the portable executable
- the executable embedded in the installer
- the installer itself

It then verifies each Authenticode signature before creating the checksum.

## GitHub Actions Secrets

`.github/workflows/release-windows.yml` supports the same signing path when
these optional repository secrets are configured:

- `WINDOWS_SIGNING_CERT_BASE64` — base64-encoded `.pfx` bytes
- `WINDOWS_SIGNING_CERT_PASSWORD` — password for that `.pfx`

When the certificate secret is present, the workflow requires signing. When it
is absent, the workflow still produces unsigned assets with checksums.

## Verifying A Download

```powershell
Get-AuthenticodeSignature .\CodexBar-<version>-Setup.exe
Get-FileHash -Algorithm SHA256 .\CodexBar-<version>-Setup.exe
Get-Content .\CodexBar-<version>-Setup.exe.sha256
```

The SHA-256 hash from `Get-FileHash` should match the value in the `.sha256`
file. A signed trusted release should report a valid Authenticode signature.

## Release Checklist

```text
[ ] npm run test
[ ] npm run build
[ ] cargo test --manifest-path apps\desktop-tauri\src-tauri\Cargo.toml
[ ] scripts\build-windows-release-assets.ps1 completed
[ ] Microsoft bootstrapper signatures verified by the script
[ ] portable exe checksum generated
[ ] installer checksum generated
[ ] Authenticode signatures valid when signing is configured
[ ] install/uninstall smoke test completed on a clean Windows VM
```
