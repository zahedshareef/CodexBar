<#
.SYNOPSIS
    Windows VM proof script for the Tauri desktop shell.

.DESCRIPTION
    Builds and launches the Tauri-based CodexBar Desktop app inside a Windows VM,
    using the CODEXBAR_PROOF_MODE env-var harness to programmatically show each
    UI surface (TrayPanel, PopOut, Settings, Settings tabs) and capture
    window-level and desktop screenshots as proof artifacts.

    Proof surfaces captured:
      - trayPanel   — the borderless tray-anchored panel
      - popOut      — the decorated pop-out dashboard
      - settings    — settings General tab
      - settings:apiKeys   — settings API Keys tab
      - settings:cookies   — settings Cookies tab
      - settings:about     — settings About tab
#>

param(
  [Parameter(Mandatory = $true)]
  [string]$ProofName,
  [switch]$CleanBuild,
  [switch]$SkipMirror,
  [switch]$SkipBuild,
  [switch]$SkipFrontend,
  [string]$ProofSurface = ''
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class TauriWindowCapture {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [DllImport("user32.dll")]
  public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
}
"@

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Log-Proof {
  param([Parameter(Mandatory)] [string]$Message)
  Add-Content -Path $captureLog -Value ("[{0}] {1}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $Message)
}

function Stop-TauriProcesses {
  Get-Process 'codexbar-desktop-tauri', 'cargo', 'node' -ErrorAction SilentlyContinue |
    Stop-Process -Force
  for ($i = 0; $i -lt 20; $i++) {
    if (-not (Get-Process 'codexbar-desktop-tauri' -ErrorAction SilentlyContinue)) {
      return
    }
    Start-Sleep -Milliseconds 300
  }
}

function Ensure-NirCmd {
  $installDir = 'C:\Users\mac\Tools\NirCmd'
  $exePath = Join-Path $installDir 'nircmd.exe'
  if (Test-Path $exePath) {
    return $exePath
  }
  New-Item -ItemType Directory -Force -Path $installDir | Out-Null
  $zipPath = Join-Path $env:TEMP 'nircmd-x64.zip'
  Invoke-WebRequest -Uri 'https://www.nirsoft.net/utils/nircmd-x64.zip' -OutFile $zipPath
  Expand-Archive -LiteralPath $zipPath -DestinationPath $installDir -Force
  if (-not (Test-Path $exePath)) {
    throw "Failed to provision NirCmd at $exePath"
  }
  return $exePath
}

function Find-VisibleWindowByTitle {
  param([Parameter(Mandatory)] [string]$Prefix)

  $script:matchedWindow = [IntPtr]::Zero
  $callback = [TauriWindowCapture+EnumWindowsProc]{
    param($hWnd, $lParam)
    if (-not [TauriWindowCapture]::IsWindowVisible($hWnd)) { return $true }
    $buf = New-Object System.Text.StringBuilder 512
    [void][TauriWindowCapture]::GetWindowText($hWnd, $buf, $buf.Capacity)
    if ($buf.ToString().StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
      $script:matchedWindow = $hWnd
      return $false
    }
    return $true
  }
  [void][TauriWindowCapture]::EnumWindows($callback, [IntPtr]::Zero)
  return $script:matchedWindow
}

function Save-WindowScreenshot {
  param(
    [Parameter(Mandatory)] [IntPtr]$Handle,
    [Parameter(Mandatory)] [string]$Path
  )
  if ($Handle -eq [IntPtr]::Zero) { return $false }

  $rect = New-Object TauriWindowCapture+RECT
  if (-not [TauriWindowCapture]::GetWindowRect($Handle, [ref]$rect)) { return $false }
  $w = $rect.Right - $rect.Left
  $h = $rect.Bottom - $rect.Top
  if ($w -le 0 -or $h -le 0) { return $false }

  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  try { $captured = [TauriWindowCapture]::PrintWindow($Handle, $hdc, 2) }
  finally { $g.ReleaseHdc($hdc); $g.Dispose() }

  if ($captured) { $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png) }
  $bmp.Dispose()
  return $captured
}

function Save-DesktopScreenshot {
  param([Parameter(Mandatory)] [string]$Path)
  try {
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
    return $true
  } catch {
    return $false
  }
}

# ---------------------------------------------------------------------------
# Launch the app in proof mode for a given surface, wait for visible window,
# capture window + desktop screenshots, then stop the process.
# ---------------------------------------------------------------------------

function Invoke-ProofCapture {
  param(
    [Parameter(Mandatory)] [string]$Surface,
    [Parameter(Mandatory)] [string]$TauriExePath,
    [Parameter(Mandatory)] [string]$Tag
  )
  Stop-TauriProcesses
  Start-Sleep -Milliseconds 500

  $env:CODEXBAR_PROOF_MODE = $Surface
  Log-Proof "proof_launch surface=$Surface tag=$Tag"
  Start-Process -FilePath $TauriExePath -WorkingDirectory (Split-Path $TauriExePath) | Out-Null

  # Wait for process to start
  $processUp = $false
  for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 500
    if (Get-Process 'codexbar-desktop-tauri' -ErrorAction SilentlyContinue) {
      $processUp = $true
      break
    }
  }
  if (-not $processUp) {
    Log-Proof "proof_process_not_found surface=$Surface"
    return
  }
  Log-Proof "proof_process_confirmed surface=$Surface"

  # Wait for the window to become visible (proof mode shows it immediately)
  $windowHandle = [IntPtr]::Zero
  for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 500
    $windowHandle = Find-VisibleWindowByTitle -Prefix 'CodexBar Desktop'
    if ($windowHandle -ne [IntPtr]::Zero) { break }
  }

  # Extra settle time for the webview to render content
  Start-Sleep -Seconds 3

  # Capture window screenshot
  $winShot = "C:\Users\mac\Desktop\$ProofName-$Tag-window.png"
  if ($windowHandle -ne [IntPtr]::Zero) {
    if (Save-WindowScreenshot -Handle $windowHandle -Path $winShot) {
      Log-Proof "proof_window_captured surface=$Surface path=$winShot"
      Get-Item $winShot | Select-Object FullName, Length, LastWriteTime
    } else {
      Log-Proof "proof_window_printwindow_failed surface=$Surface"
    }
  } else {
    Log-Proof "proof_window_not_visible surface=$Surface"
  }

  # Capture desktop screenshot
  $deskShot = "C:\Users\mac\Desktop\$ProofName-$Tag-desktop.png"
  if (Save-DesktopScreenshot -Path $deskShot) {
    Log-Proof "proof_desktop_captured surface=$Surface path=$deskShot"
    Get-Item $deskShot | Select-Object FullName, Length, LastWriteTime
  } else {
    Log-Proof "proof_desktop_capture_failed surface=$Surface"
  }

  Stop-TauriProcesses
  $env:CODEXBAR_PROOF_MODE = ''
  Log-Proof "proof_done surface=$Surface"
}

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

$shareRoot        = '\\Mac\codexbarshare'
$repoSrc          = Join-Path $shareRoot 'repo'
$repoDst          = 'C:\Users\mac\src\Win-CodexBar'
$tauriAppDir      = Join-Path $repoDst 'apps\desktop-tauri'
$tauriSrcTauriDir = Join-Path $tauriAppDir 'src-tauri'

$readyMarkerPath  = "C:\Users\mac\Desktop\$ProofName-ready.txt"
$captureLog       = "C:\Users\mac\Desktop\$ProofName-tauri-capture.log"
$buildLog         = Join-Path $env:TEMP ("codexbar-tauri-{0}-robocopy.log" -f $ProofName)

New-Item -ItemType Directory -Force -Path $repoDst | Out-Null

# ---------------------------------------------------------------------------
# Mirror repo from host share
# ---------------------------------------------------------------------------

if (-not $SkipMirror -or -not $SkipBuild -or $CleanBuild) {
  Stop-TauriProcesses
  Start-Sleep -Seconds 1
}

if (-not $SkipMirror) {
  if (-not (Test-Path $repoSrc)) {
    throw "Host share repo path missing: $repoSrc"
  }
  $excludedDirs = @('.git', (Join-Path $repoSrc 'target'), (Join-Path $repoDst 'target'),
                     (Join-Path $repoSrc 'apps\desktop-tauri\node_modules'),
                     (Join-Path $repoDst 'apps\desktop-tauri\node_modules'))
  robocopy $repoSrc $repoDst /MIR /XF nul /XD $excludedDirs > $buildLog
  $rc = $LASTEXITCODE
  if ($rc -ge 8) {
    Get-Content $buildLog -Tail 40
    throw "robocopy mirror failed (exit $rc)"
  }
}

# ---------------------------------------------------------------------------
# Clean old proof artifacts
# ---------------------------------------------------------------------------

Remove-Item "C:\Users\mac\Desktop\$ProofName-*" -ErrorAction SilentlyContinue
Remove-Item $captureLog -ErrorAction SilentlyContinue

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

$vcvarsA = 'C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$vcvarsB = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$vcvars = if (Test-Path $vcvarsA) { $vcvarsA } elseif (Test-Path $vcvarsB) { $vcvarsB } else { throw 'vcvars64.bat not found' }

if (-not $SkipBuild) {
  # Frontend build (vite)
  if (-not $SkipFrontend) {
    $npmExe = (Get-Command npm -ErrorAction SilentlyContinue).Source
    if (-not $npmExe) { throw 'npm not found — run setup-windows.ps1 first' }
    Log-Proof "frontend_build_start dir=$tauriAppDir"
    Push-Location $tauriAppDir
    try {
      & $npmExe install --prefer-offline 2>&1 | Out-Null
      & $npmExe run build
      if ($LASTEXITCODE -ne 0) { throw "vite build failed (exit $LASTEXITCODE)" }
    } finally { Pop-Location }
    Log-Proof 'frontend_build_done'
  }

  # Rust/Tauri backend build (workspace member)
  $cargoExe = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
  $cmd = 'call "' + $vcvars + '" && cd /d "' + $repoDst + '" && "' + $cargoExe + '" build -p codexbar-desktop-tauri'
  Log-Proof "cargo_build_start"
  cmd.exe /c $cmd
  if ($LASTEXITCODE -ne 0) { throw "cargo build -p codexbar-desktop-tauri failed (exit $LASTEXITCODE)" }
  Log-Proof 'cargo_build_done'
}

# ---------------------------------------------------------------------------
# Resolve built executable
# ---------------------------------------------------------------------------

$exeCandidates = @(
  (Join-Path $repoDst 'target\debug\codexbar-desktop-tauri.exe'),
  (Join-Path $repoDst 'target\x86_64-pc-windows-msvc\debug\codexbar-desktop-tauri.exe')
)
$tauriExe = $null
foreach ($c in $exeCandidates) {
  if (Test-Path $c) { $tauriExe = $c; break }
}
if (-not $tauriExe) {
  throw ("Tauri exe not found. Checked: " + ($exeCandidates -join ', '))
}
Log-Proof "exe_resolved=$tauriExe"

# ---------------------------------------------------------------------------
# Proof captures: iterate over surfaces
# ---------------------------------------------------------------------------

# If a specific surface was requested, only capture that one.
# Otherwise capture the full proof set.
if ($ProofSurface -ne '') {
  $proofTargets = @(
    @{ Surface = $ProofSurface; Tag = ($ProofSurface -replace ':', '-') }
  )
} else {
  $proofTargets = @(
    @{ Surface = 'trayPanel';       Tag = 'trayPanel' },
    @{ Surface = 'popOut';          Tag = 'popOut' },
    @{ Surface = 'settings';        Tag = 'settings-general' },
    @{ Surface = 'settings:apiKeys';  Tag = 'settings-apiKeys' },
    @{ Surface = 'settings:cookies';  Tag = 'settings-cookies' },
    @{ Surface = 'settings:about';    Tag = 'settings-about' }
  )
}

foreach ($target in $proofTargets) {
  Invoke-ProofCapture -Surface $target.Surface -TauriExePath $tauriExe -Tag $target.Tag
}

# ---------------------------------------------------------------------------
# Signal ready for host-side captures
# ---------------------------------------------------------------------------

Set-Content -Path $readyMarkerPath -Encoding UTF8 -Value ("ready " + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))
Log-Proof 'ready_marker_written'

# ---------------------------------------------------------------------------
# Also try NirCmd for a final desktop capture (belt-and-suspenders)
# ---------------------------------------------------------------------------

try {
  $nircmd = Ensure-NirCmd
  $nircmdShot = "C:\Users\mac\Desktop\$ProofName-nircmd-full.png"
  Start-Process -FilePath $nircmd -ArgumentList ('savescreenshot "' + $nircmdShot + '"') -Wait -NoNewWindow
  if (Test-Path $nircmdShot) {
    Log-Proof 'nircmd_screenshot_saved'
    Get-Item $nircmdShot | Select-Object FullName, Length, LastWriteTime
  }
} catch {
  Log-Proof ("nircmd_failed=" + $_.Exception.Message)
}

Log-Proof 'proof_complete'
