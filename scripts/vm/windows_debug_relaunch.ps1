param(
  [string]$RepoRoot = 'C:\Users\mac\src\Win-CodexBar',
  [switch]$CleanBuild,
  [ValidateSet('tauri', 'egui')]
  [string]$Shell = 'tauri'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Stop-CodexBarProcesses {
  Get-Process codexbar, 'codexbar-desktop-tauri', cargo, rustc -ErrorAction SilentlyContinue | Stop-Process -Force
  for ($i = 0; $i -lt 20; $i++) {
    if (-not (Get-Process codexbar, 'codexbar-desktop-tauri', cargo, rustc -ErrorAction SilentlyContinue)) {
      return
    }
    Start-Sleep -Milliseconds 300
  }
}

function Remove-StaleBuiltCodexBarArtifacts {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRootPath
  )

  $debugDirs = @(
    (Join-Path $RepoRootPath 'rust\target\debug'),
    (Join-Path $RepoRootPath 'rust\target\x86_64-pc-windows-msvc\debug')
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
  }

  foreach ($path in ($paths | Where-Object { $_ } | Select-Object -Unique)) {
    if (Test-Path $path) {
      Remove-Item $path -Force -ErrorAction SilentlyContinue
    }
  }
}

function Resolve-CodexBarDebugExe {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRootPath
  )

  $candidates = @(
    (Join-Path $RepoRootPath 'rust\target\debug\codexbar.exe'),
    (Join-Path $RepoRootPath 'rust\target\x86_64-pc-windows-msvc\debug\codexbar.exe')
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  throw ("Built CodexBar executable not found. Checked: " + ($candidates -join ', '))
}

function Start-InteractiveCodexBar {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ExePath
  )

  Start-Process -FilePath $ExePath -ArgumentList 'menubar' -WorkingDirectory (Split-Path $ExePath) | Out-Null
  Start-Sleep -Seconds 2
  if (Get-Process codexbar -ErrorAction SilentlyContinue) {
    return
  }

  throw 'CodexBar did not stay running after Start-Process.'
}

$vcvarsA = 'C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$vcvarsB = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$vcvars = if (Test-Path $vcvarsA) { $vcvarsA } elseif (Test-Path $vcvarsB) { $vcvarsB } else { throw 'vcvars64.bat not found' }

Stop-CodexBarProcesses
if ($CleanBuild) {
  Remove-StaleBuiltCodexBarArtifacts -RepoRootPath $RepoRoot
}

if ($Shell -eq 'tauri') {
  # Build the Tauri frontend
  $tauriAppDir = Join-Path $RepoRoot 'apps\desktop-tauri'
  $npmExe = (Get-Command npm -ErrorAction SilentlyContinue).Source
  if (-not $npmExe) { throw 'npm not found — run setup-windows.ps1 first' }
  Push-Location $tauriAppDir
  try {
    & $npmExe install --prefer-offline 2>&1 | Out-Null
    & $npmExe run build
    if ($LASTEXITCODE -ne 0) { throw "vite build failed (exit $LASTEXITCODE)" }
  } finally { Pop-Location }

  # Build the Tauri backend (workspace member)
  $cmd = 'call "' + $vcvars + '" && cd /d "' + $RepoRoot + '" && "C:\Users\mac\.cargo\bin\cargo.exe" build -p codexbar-desktop-tauri'
  cmd.exe /c $cmd
  if ($LASTEXITCODE -ne 0) { throw "cargo build -p codexbar-desktop-tauri failed (exit $LASTEXITCODE)" }

  $exeCandidates = @(
    (Join-Path $RepoRoot 'target\debug\codexbar-desktop-tauri.exe'),
    (Join-Path $RepoRoot 'target\x86_64-pc-windows-msvc\debug\codexbar-desktop-tauri.exe')
  )
  $exePath = $null
  foreach ($c in $exeCandidates) { if (Test-Path $c) { $exePath = $c; break } }
  if (-not $exePath) { throw ("Tauri exe not found. Checked: " + ($exeCandidates -join ', ')) }

  Start-Process -FilePath $exePath -WorkingDirectory (Split-Path $exePath) | Out-Null
  Start-Sleep -Seconds 2
  if (Get-Process 'codexbar-desktop-tauri' -ErrorAction SilentlyContinue) {
    [Console]::WriteLine($exePath)
    return
  }
  throw 'codexbar-desktop-tauri did not stay running after Start-Process.'
} else {
  $repoRust = Join-Path $RepoRoot 'rust'
  if (-not (Test-Path (Join-Path $repoRust 'Cargo.toml'))) {
    throw "Rust repo not found at $repoRust"
  }

  $cmd = 'call "' + $vcvars + '" && cd /d "' + $repoRust + '" && "C:\Users\mac\.cargo\bin\cargo.exe" build'
  cmd.exe /c $cmd
  if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
  }

  $exePath = Resolve-CodexBarDebugExe -RepoRootPath $RepoRoot
  Start-InteractiveCodexBar -ExePath $exePath
  [Console]::WriteLine($exePath)
}
