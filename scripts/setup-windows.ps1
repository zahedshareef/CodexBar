#Requires -Version 5.1
<#
.SYNOPSIS
    Installs build prerequisites for CodexBar on Windows.

.DESCRIPTION
    Sets up:
      1. Rust (via rustup) with the x86_64-pc-windows-gnu target
      2. MinGW-w64 toolchain (WinLibs) for the GNU linker and dlltool
      3. Adds both to the user PATH

    Run from an elevated (Admin) PowerShell if you want system-wide PATH changes,
    otherwise user-level PATH is updated.

.EXAMPLE
    .\scripts\setup-windows.ps1
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$MinGWDir = "C:\mingw64"
$MinGWBin = "$MinGWDir\bin"
$CargoBin = "$env:USERPROFILE\.cargo\bin"
$WinLibsVersion = "15.2.0posix-13.0.0-ucrt-r6"
$WinLibsZip = "winlibs-x86_64-posix-seh-gcc-15.2.0-mingw-w64ucrt-13.0.0-r6.zip"
$WinLibsUrl = "https://github.com/brechtsanders/winlibs_mingw/releases/download/$WinLibsVersion/$WinLibsZip"

function Write-Step($msg) {
    Write-Host "`n=> $msg" -ForegroundColor Cyan
}

function Add-ToUserPath($dir) {
    $current = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($current -notlike "*$dir*") {
        [Environment]::SetEnvironmentVariable("PATH", "$dir;$current", "User")
        Write-Host "   Added $dir to user PATH (restart terminal to take effect)" -ForegroundColor Yellow
    } else {
        Write-Host "   $dir already in user PATH" -ForegroundColor Green
    }
    # Also update current session
    if ($env:PATH -notlike "*$dir*") {
        $env:PATH = "$dir;$env:PATH"
    }
}

# ── 1. Rust ──────────────────────────────────────────────────────────────────

Write-Step "Checking Rust installation"

if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $rustVersion = rustc --version
    Write-Host "   Found: $rustVersion" -ForegroundColor Green
} else {
    Write-Step "Installing Rust via rustup"
    $rustupInit = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit
    & $rustupInit -y --default-toolchain stable --default-host x86_64-pc-windows-gnu
    Remove-Item $rustupInit -ErrorAction SilentlyContinue
    Add-ToUserPath $CargoBin
    Write-Host "   Rust installed" -ForegroundColor Green
}

# ── 2. GNU target ────────────────────────────────────────────────────────────

Write-Step "Ensuring x86_64-pc-windows-gnu target is installed"

$targets = rustup target list --installed 2>&1
if ($targets -match "x86_64-pc-windows-gnu") {
    Write-Host "   Target already installed" -ForegroundColor Green
} else {
    rustup target add x86_64-pc-windows-gnu
    Write-Host "   Target added" -ForegroundColor Green
}

# ── 3. MinGW-w64 (WinLibs) ──────────────────────────────────────────────────

Write-Step "Checking MinGW-w64 (dlltool, gcc)"

if (Get-Command dlltool -ErrorAction SilentlyContinue) {
    $dlltoolPath = (Get-Command dlltool).Source
    Write-Host "   Found: $dlltoolPath" -ForegroundColor Green
} else {
    if (Test-Path "$MinGWBin\dlltool.exe") {
        Write-Host "   MinGW exists at $MinGWDir but not in PATH" -ForegroundColor Yellow
    } else {
        Write-Step "Downloading MinGW-w64 (WinLibs) — ~250 MB"
        $zipPath = "$env:TEMP\$WinLibsZip"

        if (-not (Test-Path $zipPath)) {
            Invoke-WebRequest -Uri $WinLibsUrl -OutFile $zipPath
        }

        Write-Step "Extracting to $MinGWDir"
        Expand-Archive -Path $zipPath -DestinationPath "C:\" -Force
        Remove-Item $zipPath -ErrorAction SilentlyContinue
        Write-Host "   Extracted" -ForegroundColor Green
    }

    Add-ToUserPath $MinGWBin
}

# ── 4. Verify ────────────────────────────────────────────────────────────────

Write-Step "Verifying toolchain"

$checks = @(
    @{ Name = "rustc";   Cmd = "rustc --version" },
    @{ Name = "cargo";   Cmd = "cargo --version" },
    @{ Name = "dlltool"; Cmd = "dlltool --version" },
    @{ Name = "gcc";     Cmd = "gcc --version" }
)

$allOk = $true
foreach ($check in $checks) {
    try {
        $out = Invoke-Expression $check.Cmd 2>&1 | Select-Object -First 1
        Write-Host "   [OK] $($check.Name): $out" -ForegroundColor Green
    } catch {
        Write-Host "   [FAIL] $($check.Name) not found" -ForegroundColor Red
        $allOk = $false
    }
}

Write-Host ""
if ($allOk) {
    Write-Host "All prerequisites installed. You can now build:" -ForegroundColor Green
    Write-Host "   cd rust" -ForegroundColor White
    Write-Host "   cargo build" -ForegroundColor White
} else {
    Write-Host "Some tools are missing. Restart your terminal and re-run this script." -ForegroundColor Yellow
}
