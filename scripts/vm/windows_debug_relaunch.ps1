param(
  [string]$RepoRoot = 'C:\Users\mac\src\Win-CodexBar',
  [switch]$CleanBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Stop-CodexBarProcesses {
  Get-Process codexbar, cargo, rustc -ErrorAction SilentlyContinue | Stop-Process -Force
  for ($i = 0; $i -lt 20; $i++) {
    if (-not (Get-Process codexbar, cargo, rustc -ErrorAction SilentlyContinue)) {
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

$repoRust = Join-Path $RepoRoot 'rust'
if (-not (Test-Path (Join-Path $repoRust 'Cargo.toml'))) {
  throw "Rust repo not found at $repoRust"
}

$vcvarsA = 'C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$vcvarsB = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$vcvars = if (Test-Path $vcvarsA) { $vcvarsA } elseif (Test-Path $vcvarsB) { $vcvarsB } else { throw 'vcvars64.bat not found' }

Stop-CodexBarProcesses
if ($CleanBuild) {
  Remove-StaleBuiltCodexBarArtifacts -RepoRootPath $RepoRoot
}

$cmd = 'call "' + $vcvars + '" && cd /d "' + $repoRust + '" && "C:\Users\mac\.cargo\bin\cargo.exe" build'
cmd.exe /c $cmd
if ($LASTEXITCODE -ne 0) {
  throw "cargo build failed with exit code $LASTEXITCODE"
}

$exePath = Resolve-CodexBarDebugExe -RepoRootPath $RepoRoot
Start-InteractiveCodexBar -ExePath $exePath
[Console]::WriteLine($exePath)
