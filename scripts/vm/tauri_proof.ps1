<#
.SYNOPSIS
    Windows VM proof script for the Tauri desktop shell.

.DESCRIPTION
    Builds and launches the Tauri-based CodexBar Desktop app inside a Windows VM,
    captures desktop and window screenshots as proof artifacts.

    Unlike the egui-based provider_osclick_proof_unc.ps1, this script does NOT
    rely on a TCP test-command server.  The Tauri shell does not expose one.
    Proof collection is limited to:
      - verifying the process starts and stays running
      - verifying the tray icon / window appears
      - capturing full-desktop and per-window screenshots

    For interactive UI navigation proofs (tab switching, preferences drill-down),
    use the egui shell workflow until a Tauri-native test harness is added.
#>

param(
  [Parameter(Mandatory = $true)]
  [string]$ProofName,
  [switch]$CleanBuild,
  [switch]$SkipMirror,
  [switch]$SkipBuild,
  [switch]$SkipFrontend
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

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

$shareRoot        = '\\Mac\codexbarshare'
$repoSrc          = Join-Path $shareRoot 'repo'
$repoDst          = 'C:\Users\mac\src\Win-CodexBar'
$tauriAppDir      = Join-Path $repoDst 'apps\desktop-tauri'
$tauriSrcTauriDir = Join-Path $tauriAppDir 'src-tauri'

$desktopShot      = "C:\Users\mac\Desktop\$ProofName-desktop-full.png"
$windowShot       = "C:\Users\mac\Desktop\$ProofName-window-capture.png"
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
# Clean old artifacts
# ---------------------------------------------------------------------------

Remove-Item $desktopShot    -ErrorAction SilentlyContinue
Remove-Item $windowShot     -ErrorAction SilentlyContinue
Remove-Item $readyMarkerPath -ErrorAction SilentlyContinue
Remove-Item $captureLog     -ErrorAction SilentlyContinue

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
# Launch
# ---------------------------------------------------------------------------

Stop-TauriProcesses
Start-Sleep -Seconds 1

Start-Process -FilePath $tauriExe -WorkingDirectory (Split-Path $tauriExe) | Out-Null
Log-Proof 'tauri_launched'

# Wait for the process to stabilise
$processUp = $false
for ($i = 0; $i -lt 30; $i++) {
  Start-Sleep -Milliseconds 500
  if (Get-Process 'codexbar-desktop-tauri' -ErrorAction SilentlyContinue) {
    $processUp = $true
    break
  }
}
if (-not $processUp) {
  Log-Proof 'tauri_process_not_found'
  throw 'codexbar-desktop-tauri did not stay running after launch'
}
Log-Proof 'tauri_process_confirmed'

# Give the app time to initialise tray icon and webview
Start-Sleep -Seconds 4

# ---------------------------------------------------------------------------
# Signal ready for host-side captures
# ---------------------------------------------------------------------------

Set-Content -Path $readyMarkerPath -Encoding UTF8 -Value ("ready " + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))
Log-Proof 'ready_marker_written'

# ---------------------------------------------------------------------------
# Capture: window screenshot via PrintWindow
# ---------------------------------------------------------------------------

$windowHandle = Find-VisibleWindowByTitle -Prefix 'CodexBar Desktop'
if ($windowHandle -ne [IntPtr]::Zero) {
  if (Save-WindowScreenshot -Handle $windowHandle -Path $windowShot) {
    Log-Proof 'window_screenshot_saved'
    Get-Item $windowShot | Select-Object FullName, Length, LastWriteTime
  } else {
    Log-Proof 'window_printwindow_failed'
  }
} else {
  Log-Proof 'window_not_visible_for_capture'
  # The Tauri window starts hidden (visible: false in tauri.conf.json).
  # It only becomes visible after a tray left-click or shortcut toggle.
  # Without a TCP test harness, we cannot programmatically show it.
  # The host-side Parallels capture will still get the tray icon in the taskbar.
}

# ---------------------------------------------------------------------------
# Capture: full desktop screenshot
# ---------------------------------------------------------------------------

Start-Sleep -Seconds 2
try {
  $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
  $bmp.Save($desktopShot, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose()
  $bmp.Dispose()
  Log-Proof 'desktop_screenshot_saved'
  Get-Item $desktopShot | Select-Object FullName, Length, LastWriteTime
} catch {
  Log-Proof ("desktop_screenshot_failed=" + $_.Exception.Message)
}

# ---------------------------------------------------------------------------
# Also try NirCmd for a second desktop capture (belt-and-suspenders)
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
