param(
  [Parameter(Mandatory = $true)]
  [string]$LaunchProfile,
  [Parameter(Mandatory = $true)]
  [string]$ProofName,
  [Parameter(Mandatory = $true)]
  [string]$SelectedProvider,
  [ValidateSet('provider', 'tab', 'menu')]
  [string]$CaptureMode = 'provider',
  [string]$MenuSelectedTab = '',
  [string]$PreferencesTab = 'providers',
  [switch]$CleanBuild,
  [switch]$SkipMirror,
  [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class CodexBarWindowCapture {
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

function Stop-StaleCodexBarProofTasks {
  try {
    Get-ScheduledTask -TaskName 'CodexBarProof-*' -ErrorAction SilentlyContinue | ForEach-Object {
      try {
        Stop-ScheduledTask -TaskName $_.TaskName -ErrorAction SilentlyContinue | Out-Null
      } catch {}
      try {
        Unregister-ScheduledTask -TaskName $_.TaskName -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
      } catch {}
    }
  } catch {}
}

function Stop-CodexBarProcesses {
  Get-Process codexbar,cargo,rustc -ErrorAction SilentlyContinue | Stop-Process -Force
  for ($i = 0; $i -lt 20; $i++) {
    if (-not (Get-Process codexbar,cargo,rustc -ErrorAction SilentlyContinue)) {
      return
    }
    Start-Sleep -Milliseconds 300
  }
}

function Remove-StaleBuiltCodexBarArtifacts {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot
  )

  $debugDirs = @(
    (Join-Path $RepoRoot 'rust\target\debug'),
    (Join-Path $RepoRoot 'rust\target\x86_64-pc-windows-msvc\debug')
  )

  $paths = @()
  foreach ($debugDir in $debugDirs) {
    if (-not (Test-Path $debugDir)) {
      continue
    }

    $paths += @(
      (Join-Path $debugDir 'codexbar.exe'),
      (Join-Path $debugDir 'codexbar.pdb')
    )

    $depsDir = Join-Path $debugDir 'deps'
    if (Test-Path $depsDir) {
      $paths += Get-ChildItem $depsDir -Filter 'codexbar*.exe' -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName }
      $paths += Get-ChildItem $depsDir -Filter 'codexbar*.pdb' -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName }
    }
  }

  foreach ($path in ($paths | Where-Object { $_ } | Select-Object -Unique)) {
    for ($i = 0; $i -lt 10; $i++) {
      try {
        if (Test-Path $path) {
          Remove-Item $path -Force -ErrorAction Stop
        }
        break
      } catch {
        Start-Sleep -Milliseconds 400
      }
    }
  }
}

function Resolve-CodexBarDebugExe {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot
  )

  $candidates = @(
    (Join-Path $RepoRoot 'rust\target\debug\codexbar.exe'),
    (Join-Path $RepoRoot 'rust\target\x86_64-pc-windows-msvc\debug\codexbar.exe')
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  throw ("Built CodexBar executable not found. Checked: " + ($candidates -join ', '))
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

function Invoke-InteractiveCommandTask {
  param(
    [Parameter(Mandatory = $true)]
    [string]$TaskName,
    [Parameter(Mandatory = $true)]
    [string]$Execute,
    [Parameter(Mandatory = $true)]
    [string]$Arguments
  )

  Start-Process -FilePath $Execute -ArgumentList $Arguments -WorkingDirectory (Split-Path $Execute) | Out-Null
}

function Find-VisibleWindowHandleByTitlePrefix {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Prefix
  )

  $script:matchedWindow = [IntPtr]::Zero
  $callback = [CodexBarWindowCapture+EnumWindowsProc]{
    param($hWnd, $lParam)

    if (-not [CodexBarWindowCapture]::IsWindowVisible($hWnd)) {
      return $true
    }

    $titleBuilder = New-Object System.Text.StringBuilder 512
    [void][CodexBarWindowCapture]::GetWindowText($hWnd, $titleBuilder, $titleBuilder.Capacity)
    $title = $titleBuilder.ToString()
    if ($title.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
      $script:matchedWindow = $hWnd
      return $false
    }

    return $true
  }

  [void][CodexBarWindowCapture]::EnumWindows($callback, [IntPtr]::Zero)
  return $script:matchedWindow
}

function Get-VisibleWindowTitles {
  $titles = New-Object System.Collections.Generic.List[string]
  $callback = [CodexBarWindowCapture+EnumWindowsProc]{
    param($hWnd, $lParam)

    if (-not [CodexBarWindowCapture]::IsWindowVisible($hWnd)) {
      return $true
    }

    $titleBuilder = New-Object System.Text.StringBuilder 512
    [void][CodexBarWindowCapture]::GetWindowText($hWnd, $titleBuilder, $titleBuilder.Capacity)
    $title = $titleBuilder.ToString().Trim()
    if (-not [string]::IsNullOrWhiteSpace($title)) {
      $titles.Add($title) | Out-Null
    }

    return $true
  }

  [void][CodexBarWindowCapture]::EnumWindows($callback, [IntPtr]::Zero)
  return $titles.ToArray()
}

function Find-VisibleWindowHandleNearRect {
  param(
    [Parameter(Mandatory = $true)]
    [int]$Left,
    [Parameter(Mandatory = $true)]
    [int]$Top,
    [Parameter(Mandatory = $true)]
    [int]$Right,
    [Parameter(Mandatory = $true)]
    [int]$Bottom
  )

  $targetWidth = $Right - $Left
  $targetHeight = $Bottom - $Top
  $script:bestWindow = [IntPtr]::Zero
  $script:bestWindowScore = [double]::PositiveInfinity

  $callback = [CodexBarWindowCapture+EnumWindowsProc]{
    param($hWnd, $lParam)

    if (-not [CodexBarWindowCapture]::IsWindowVisible($hWnd)) {
      return $true
    }

    $rect = New-Object CodexBarWindowCapture+RECT
    if (-not [CodexBarWindowCapture]::GetWindowRect($hWnd, [ref]$rect)) {
      return $true
    }

    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) {
      return $true
    }

    $score =
      [math]::Abs($rect.Left - $Left) +
      [math]::Abs($rect.Top - $Top) +
      [math]::Abs($rect.Right - $Right) +
      [math]::Abs($rect.Bottom - $Bottom) +
      [math]::Abs($width - $targetWidth) +
      [math]::Abs($height - $targetHeight)

    if ($score -lt $script:bestWindowScore) {
      $script:bestWindowScore = $score
      $script:bestWindow = $hWnd
    }

    return $true
  }

  [void][CodexBarWindowCapture]::EnumWindows($callback, [IntPtr]::Zero)
  return $script:bestWindow
}

function Get-PreferencesRectFromState {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  if (-not (Test-Path $Path)) {
    return $null
  }

  try {
    $json = Get-Content $Path -Raw | ConvertFrom-Json
    $rect = $json.preferences_viewport_outer_rect
    if ($null -eq $rect) {
      return $null
    }

    return @{
      Left = [int][math]::Round([double]$rect.min_x)
      Top = [int][math]::Round([double]$rect.min_y)
      Right = [int][math]::Round([double]$rect.max_x)
      Bottom = [int][math]::Round([double]$rect.max_y)
    }
  } catch {
    Log-WindowCapture ("state_rect_parse_failed=" + $_.Exception.Message)
    return $null
  }
}

function Get-TargetCenterFromState {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Name
  )

  if (-not (Test-Path $Path)) {
    return $null
  }

  try {
    $json = Get-Content $Path -Raw | ConvertFrom-Json
    $viewportRect = $json.viewport_outer_rect
    $offsetX = 0.0
    $offsetY = 0.0
    if ($null -ne $viewportRect) {
      if ($null -ne $viewportRect.min_x) {
        $offsetX = [double]$viewportRect.min_x
      }
      if ($null -ne $viewportRect.min_y) {
        $offsetY = [double]$viewportRect.min_y
      }
    }
    foreach ($target in @($json.tab_targets) + @($json.preferences_tab_targets)) {
      if ($null -ne $target -and $target.name -eq $Name -and $null -ne $target.rect) {
        return [pscustomobject]@{
          X = $offsetX + [double]$target.rect.center_x
          Y = $offsetY + [double]$target.rect.center_y
        }
      }
    }
  } catch {}

  return $null
}

function Get-ViewportOriginFromState {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  if (-not (Test-Path $Path)) {
    return $null
  }

  try {
    $json = Get-Content $Path -Raw | ConvertFrom-Json
    $viewportRect = $json.viewport_outer_rect
    if ($null -ne $viewportRect -and $null -ne $viewportRect.min_x -and $null -ne $viewportRect.min_y) {
      return [pscustomobject]@{
        X = [double]$viewportRect.min_x
        Y = [double]$viewportRect.min_y
      }
    }
  } catch {}

  return $null
}

function Get-DeterministicSettingsClickPoint {
  $workingArea = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
  $targetWidth = 360.0
  $targetHeight = 500.0
  $windowX = $workingArea.Left + ($workingArea.Width * 0.22) - ($targetWidth * 0.5)
  $windowY = $workingArea.Top + (($workingArea.Height - $targetHeight) * 0.5)

  return [pscustomobject]@{
    X = $windowX + 314.0
    Y = $windowY + 190.0
  }
}

function Save-WindowScreenshot {
  param(
    [Parameter(Mandatory = $true)]
    [IntPtr]$Handle,
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  if ($Handle -eq [IntPtr]::Zero) {
    return $false
  }

  $rect = New-Object CodexBarWindowCapture+RECT
  if (-not [CodexBarWindowCapture]::GetWindowRect($Handle, [ref]$rect)) {
    return $false
  }

  $width = $rect.Right - $rect.Left
  $height = $rect.Bottom - $rect.Top
  if ($width -le 0 -or $height -le 0) {
    return $false
  }

  $bitmap = New-Object System.Drawing.Bitmap $width, $height
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $hdc = $graphics.GetHdc()
  try {
    $captured = [CodexBarWindowCapture]::PrintWindow($Handle, $hdc, 2)
  } finally {
    $graphics.ReleaseHdc($hdc)
    $graphics.Dispose()
  }

  if ($captured) {
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
  }
  $bitmap.Dispose()
  return $captured
}

function Log-WindowCapture {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Message
  )

  Add-Content -Path $windowCaptureLog -Value ("[{0}] {1}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $Message)
}

$shareRoot = '\\Mac\codexbarshare'
$repoSrc = Join-Path $shareRoot 'repo'
$repoDst = 'C:\Users\mac\src\Win-CodexBar'
$desktopShot = "C:\Users\mac\Desktop\$ProofName-osclick-proof.png"
$interactiveDesktopShot = "C:\Users\mac\Desktop\$ProofName-interactive-full.png"
$menuDesktopShot = "C:\Users\mac\Desktop\$ProofName-menu-full.png"
$preferencesShot = "C:\Users\mac\Desktop\$ProofName-preferences-proof.png"
$statePath = "C:\Users\mac\Desktop\$ProofName-state.json"
$readyMarkerPath = "C:\Users\mac\Desktop\$ProofName-ready.txt"
$windowCaptureLog = "C:\Users\mac\Desktop\$ProofName-window-capture.log"
$buildLog = Join-Path $env:TEMP ("codexbar-{0}-robocopy.log" -f $ProofName)

New-Item -ItemType Directory -Force -Path $repoDst | Out-Null

if (-not $SkipMirror -or -not $SkipBuild -or $CleanBuild) {
  Stop-StaleCodexBarProofTasks
  Stop-CodexBarProcesses
  if ($CleanBuild) {
    Remove-StaleBuiltCodexBarArtifacts -RepoRoot $repoDst
  }
  Start-Sleep -Seconds 1
}

if (-not $SkipMirror) {
  if (!(Test-Path $repoSrc)) {
    throw "Host share repo path missing: $repoSrc"
  }

  $excludedMirrorDirs = @(
    '.git',
    (Join-Path $repoSrc 'rust\target'),
    (Join-Path $repoDst 'rust\target')
  )
  robocopy $repoSrc $repoDst /MIR /XF nul /XD $excludedMirrorDirs > $buildLog
  $robocopyExit = $LASTEXITCODE
  if ($robocopyExit -ge 8) {
    Get-Content $buildLog -Tail 40
    throw "robocopy failed with exit code $robocopyExit"
  }
}

Remove-Item $desktopShot -ErrorAction SilentlyContinue
Remove-Item $interactiveDesktopShot -ErrorAction SilentlyContinue
Remove-Item $menuDesktopShot -ErrorAction SilentlyContinue
Remove-Item $desktopShot -ErrorAction SilentlyContinue
Remove-Item $preferencesShot -ErrorAction SilentlyContinue
Remove-Item $statePath -ErrorAction SilentlyContinue
Remove-Item $readyMarkerPath -ErrorAction SilentlyContinue
Remove-Item $windowCaptureLog -ErrorAction SilentlyContinue
Remove-Item $preferencesShot -ErrorAction SilentlyContinue
Remove-Item $statePath -ErrorAction SilentlyContinue
Remove-Item $readyMarkerPath -ErrorAction SilentlyContinue
Remove-Item $windowCaptureLog -ErrorAction SilentlyContinue

$repo = 'C:\Users\mac\src\Win-CodexBar\rust'
$vcvarsA = 'C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$vcvarsB = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$vcvars = if (Test-Path $vcvarsA) { $vcvarsA } elseif (Test-Path $vcvarsB) { $vcvarsB } else { throw 'vcvars64.bat not found' }
if (-not $SkipBuild) {
  $cmd = 'call "' + $vcvars + '" && cd /d "' + $repo + '" && "C:\Users\mac\.cargo\bin\cargo.exe" build'
  cmd.exe /c $cmd
  if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
  }
}

$needsCodexSettings = ($LaunchProfile -eq 'codex' -or $SelectedProvider -eq 'codex')
if ($needsCodexSettings) {
  $settingsDir = 'C:\Users\mac\AppData\Roaming\CodexBar'
  $settingsPath = Join-Path $settingsDir 'settings.json'
  New-Item -ItemType Directory -Force -Path $settingsDir | Out-Null
@'
{
  "enabled_providers": ["claude", "codex"],
  "refresh_interval_secs": 300,
  "reset_time_relative": true,
  "menu_bar_display_mode": "detailed",
  "show_as_used": true,
  "show_credits_extra_usage": true,
  "codex_usage_source": "auto",
  "codex_cookie_source": "auto",
  "codex_historical_tracking": false,
  "codex_openai_web_extras": true
}
'@ | Set-Content -Encoding UTF8 $settingsPath
}

Stop-StaleCodexBarProofTasks
Stop-CodexBarProcesses
Start-Sleep -Seconds 1

$taskName = ('CodexBarProof-' + ($ProofName -replace '[^A-Za-z0-9_-]', '-'))
try {
  Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
} catch {}

$codexBarExe = Resolve-CodexBarDebugExe -RepoRoot $repoDst
Start-Process -FilePath $codexBarExe -ArgumentList 'menubar' -WorkingDirectory (Split-Path $codexBarExe) | Out-Null

$portReady = $false
for ($i = 0; $i -lt 40; $i++) {
  Start-Sleep -Milliseconds 500
  try {
    $probe = Test-NetConnection 127.0.0.1 -Port 19400 -WarningAction SilentlyContinue
    if ($probe.TcpTestSucceeded) {
      $portReady = $true
      break
    }
  } catch {}
}

if (-not $portReady) {
  Get-Process codexbar -ErrorAction SilentlyContinue | Select-Object Id, ProcessName, MainWindowTitle, Responding, Path
  throw 'CodexBar UI test server did not come up on 127.0.0.1:19400'
}

$commands = @(
  '{"type":"simulate_tray_left_click"}'
)

function Send-TestCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Line
  )

  $client = New-Object System.Net.Sockets.TcpClient('127.0.0.1', 19400)
  $stream = $client.GetStream()
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($Line + "`n")
  $stream.Write($bytes, 0, $bytes.Length)
  $stream.Flush()
  $stream.Close()
  $client.Close()
  Start-Sleep -Milliseconds 420
}

foreach ($line in $commands) {
  Send-TestCommand -Line $line
}

Start-Sleep -Milliseconds 2600

if ($CaptureMode -eq 'menu') {
  if (-not [string]::IsNullOrWhiteSpace($MenuSelectedTab)) {
    Send-TestCommand -Line ('{"type":"select_tab","tab":"' + $MenuSelectedTab + '"}')
    Start-Sleep -Milliseconds 700
  }
  Send-TestCommand -Line ('{"type":"save_state","path":"' + ($statePath -replace '\\', '\\\\') + '"}')
  Start-Sleep -Milliseconds 700
  Set-Content -Path $readyMarkerPath -Encoding UTF8 -Value ("ready " + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))

  try {
    $nircmd = Ensure-NirCmd
    $captureTask = ('CodexBarProof-' + ($ProofName -replace '[^A-Za-z0-9_-]', '-') + '-menu-capture')
    Invoke-InteractiveCommandTask `
      -TaskName $captureTask `
      -Execute $nircmd `
      -Arguments ('savescreenshot "' + $menuDesktopShot + '"')
    Start-Sleep -Seconds 2
    if (Test-Path $menuDesktopShot) {
      Log-WindowCapture 'nircmd_interactive_saved_menu_full_desktop_shot'
      Get-Item $menuDesktopShot | Select-Object FullName, Length, LastWriteTime
    }
  } catch {
    Log-WindowCapture ("menu_nircmd_failed=" + $_.Exception.Message)
  }

  Start-Sleep -Seconds 2
  exit 0
}

Send-TestCommand -Line ('{"type":"save_state","path":"' + ($statePath -replace '\\', '\\\\') + '"}')
Start-Sleep -Milliseconds 700

$settingsButtonTarget = $null
for ($i = 0; $i -lt 20; $i++) {
  $settingsButtonTarget = Get-TargetCenterFromState -Path $statePath -Name 'menu:settings'
  if ($null -ne $settingsButtonTarget) {
    break
  }

  Log-WindowCapture ("menu_settings_target_missing attempt=" + ($i + 1))
  Send-TestCommand -Line ('{"type":"save_state","path":"' + ($statePath -replace '\\', '\\\\') + '"}')
  Start-Sleep -Milliseconds 700
}

if ($null -ne $settingsButtonTarget) {
  $clickCommand = [string]::Format(
    [System.Globalization.CultureInfo]::InvariantCulture,
    '{{"type":"click","x":{0},"y":{1}}}',
    $settingsButtonTarget.X,
    $settingsButtonTarget.Y
  )
  Log-WindowCapture (
    [string]::Format(
      [System.Globalization.CultureInfo]::InvariantCulture,
      'click_menu_settings x={0} y={1}',
      $settingsButtonTarget.X,
      $settingsButtonTarget.Y
    )
  )
  Send-TestCommand -Line $clickCommand
  Start-Sleep -Milliseconds 1200
} else {
  $viewportOrigin = Get-ViewportOriginFromState -Path $statePath
  if ($null -eq $viewportOrigin) {
    for ($i = 0; $i -lt 20; $i++) {
      Log-WindowCapture ("viewport_origin_missing attempt=" + ($i + 1))
      Send-TestCommand -Line ('{"type":"save_state","path":"' + ($statePath -replace '\\', '\\\\') + '"}')
      Start-Sleep -Milliseconds 700
      $viewportOrigin = Get-ViewportOriginFromState -Path $statePath
      if ($null -ne $viewportOrigin) {
        break
      }
    }
  }

  if ($null -ne $viewportOrigin) {
    $fallbackX = $viewportOrigin.X + 314.0
    $fallbackY = $viewportOrigin.Y + 190.0
    $fallbackClickCommand = [string]::Format(
      [System.Globalization.CultureInfo]::InvariantCulture,
      '{{"type":"click","x":{0},"y":{1}}}',
      $fallbackX,
      $fallbackY
    )
    Log-WindowCapture (
      [string]::Format(
        [System.Globalization.CultureInfo]::InvariantCulture,
        'click_menu_settings_fallback x={0} y={1}',
        $fallbackX,
        $fallbackY
      )
    )
    Send-TestCommand -Line $fallbackClickCommand
    Start-Sleep -Milliseconds 1200
  } else {
    $deterministicPoint = Get-DeterministicSettingsClickPoint
    $deterministicClickCommand = [string]::Format(
      [System.Globalization.CultureInfo]::InvariantCulture,
      '{{"type":"click","x":{0},"y":{1}}}',
      $deterministicPoint.X,
      $deterministicPoint.Y
    )
    Log-WindowCapture (
      [string]::Format(
        [System.Globalization.CultureInfo]::InvariantCulture,
        'click_menu_settings_deterministic x={0} y={1}',
        $deterministicPoint.X,
        $deterministicPoint.Y
      )
    )
    Send-TestCommand -Line $deterministicClickCommand
    Start-Sleep -Milliseconds 1200
  }
}

Send-TestCommand -Line ('{"type":"select_preferences_tab","tab":"' + $PreferencesTab + '"}')

if ($CaptureMode -eq 'provider' -and $needsCodexSettings) {
  Send-TestCommand -Line '{"type":"set_provider_enabled","provider":"codex","enabled":true}'
}

if ($CaptureMode -eq 'provider' -and $SelectedProvider -eq 'kiro') {
  Send-TestCommand -Line '{"type":"set_runtime_provider_state","provider":"kiro","state":"error","error":"kiro-cli: No such file or directory"}'
}

if ($CaptureMode -eq 'provider') {
  Send-TestCommand -Line '{"type":"select_preferences_tab","tab":"providers"}'
  Send-TestCommand -Line ('{"type":"select_preferences_provider","provider":"' + $SelectedProvider + '"}')
}

$stateReady = $false
for ($i = 0; $i -lt 12; $i++) {
  Send-TestCommand -Line ('{"type":"save_state","path":"' + ($statePath -replace '\\', '\\\\') + '"}')
  Start-Sleep -Milliseconds 500
  if ((Test-Path $statePath) -and (Get-Item $statePath).Length -gt 0) {
    $stateReady = $true
    break
  }
}

$preferencesRect = $null
if ($stateReady) {
  for ($i = 0; $i -lt 8; $i++) {
    $preferencesRect = Get-PreferencesRectFromState -Path $statePath
    if ($null -ne $preferencesRect) {
      Log-WindowCapture (
        "state_preferences_rect={0},{1},{2},{3}" -f
          $preferencesRect.Left,
          $preferencesRect.Top,
          $preferencesRect.Right,
          $preferencesRect.Bottom
      )
      break
    }

    Log-WindowCapture ("state_preferences_rect_missing attempt=" + ($i + 1))
    Send-TestCommand -Line ('{"type":"save_state","path":"' + ($statePath -replace '\\', '\\\\') + '"}')
    Start-Sleep -Milliseconds 700
  }
}

Set-Content -Path $readyMarkerPath -Encoding UTF8 -Value ("ready " + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))

Send-TestCommand -Line ('{"type":"save_preferences_screenshot","path":"' + ($preferencesShot -replace '\\', '\\\\') + '"}')

$preferencesShotReady = $false
$interactiveDesktopShotReady = $false
for ($i = 0; $i -lt 10; $i++) {
  Start-Sleep -Milliseconds 500
  if (Test-Path $preferencesShot) {
    $preferencesShotReady = $true
    break
  }
}

if ($preferencesShotReady) {
  Get-Item $preferencesShot | Select-Object FullName, Length, LastWriteTime
} else {
  try {
    $nircmd = Ensure-NirCmd
    $captureTask = ('CodexBarProof-' + ($ProofName -replace '[^A-Za-z0-9_-]', '-') + '-capture')
    Invoke-InteractiveCommandTask `
      -TaskName $captureTask `
      -Execute $nircmd `
      -Arguments ('savescreenshot "' + $interactiveDesktopShot + '"')
    Start-Sleep -Seconds 2
    if (Test-Path $interactiveDesktopShot) {
      Log-WindowCapture 'nircmd_interactive_saved_full_desktop_shot'
      $interactiveDesktopShotReady = $true
      Get-Item $interactiveDesktopShot | Select-Object FullName, Length, LastWriteTime
    }
  } catch {
    Log-WindowCapture ("nircmd_failed=" + $_.Exception.Message)
  }
}

if (-not $preferencesShotReady) {
  $settingsWindow = [IntPtr]::Zero
  if ($null -ne $preferencesRect) {
    $settingsWindow = Find-VisibleWindowHandleNearRect `
      -Left $preferencesRect.Left `
      -Top $preferencesRect.Top `
      -Right $preferencesRect.Right `
      -Bottom $preferencesRect.Bottom
    Log-WindowCapture ("rect_matched_handle=" + $settingsWindow)
  }
  if ($settingsWindow -eq [IntPtr]::Zero) {
    $settingsWindow = Find-VisibleWindowHandleByTitlePrefix -Prefix 'Settings'
  }
  Log-WindowCapture ("settings_handle=" + $settingsWindow)
  if ($settingsWindow -eq [IntPtr]::Zero) {
    $visibleTitles = Get-VisibleWindowTitles
    Log-WindowCapture ("visible_window_count=" + $visibleTitles.Count)
    if ($visibleTitles.Count -gt 0) {
      Log-WindowCapture ("visible_windows=" + ($visibleTitles -join ' | '))
    }
    Log-WindowCapture 'settings_window_not_found'
  } elseif (Save-WindowScreenshot -Handle $settingsWindow -Path $preferencesShot) {
    Log-WindowCapture 'printwindow_saved_preferences_shot'
    $preferencesShotReady = $true
    Get-Item $preferencesShot | Select-Object FullName, Length, LastWriteTime
  } else {
    Log-WindowCapture 'printwindow_failed'
  }
}

Log-WindowCapture 'post_capture_settle'
Start-Sleep -Seconds 6

try {
  $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
  $bmp.Save($desktopShot, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose()
  $bmp.Dispose()

  Get-Item $desktopShot | Select-Object FullName, Length, LastWriteTime
} catch {
  Write-Warning ("Desktop screenshot capture failed: " + $_.Exception.Message)
}
