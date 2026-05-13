param(
    [string]$Version = "",
    [string]$Configuration = "release",
    [switch]$SkipNpmInstall,
    [switch]$SkipInstaller,
    [switch]$NoBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Write-Step {
    param([string]$Message)
    Write-Host "[release] $Message"
}

function Get-RepoRoot {
    $scriptDir = Split-Path -Parent $PSCommandPath
    (Resolve-Path (Join-Path $scriptDir "..")).Path
}

function Get-AppVersion {
    param([string]$RepoRoot)

    if ($Version) {
        return $Version
    }

    $cargoToml = Join-Path $RepoRoot "rust\Cargo.toml"
    $match = Select-String -LiteralPath $cargoToml -Pattern '^version = "([^"]+)"' | Select-Object -First 1
    if (-not $match) {
        throw "Could not read package version from $cargoToml"
    }
    return $match.Matches[0].Groups[1].Value
}

function Get-IsccPath {
    $default = Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"
    if (Test-Path -LiteralPath $default) {
        return $default
    }

    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    return $null
}

function Assert-MicrosoftSignature {
    param(
        [string]$Path,
        [string]$Label
    )

    $signature = Get-AuthenticodeSignature -FilePath $Path
    if ($signature.Status -ne "Valid") {
        throw "$Label signature is not valid. Status: $($signature.Status)"
    }

    $subject = $signature.SignerCertificate.Subject
    if ($subject -notlike "*Microsoft Corporation*") {
        throw "$Label signer is unexpected: $subject"
    }
}

function Save-Checksum {
    param([string]$Path)

    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    $fileName = Split-Path -Leaf $Path
    "$hash  $fileName" | Set-Content -Encoding ascii "$Path.sha256"
}

if (-not [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows
)) {
    throw "Windows release assets must be built on Windows."
}

$repoRoot = Get-RepoRoot
$appVersion = Get-AppVersion -RepoRoot $repoRoot
$desktopDir = Join-Path $repoRoot "apps\desktop-tauri"
$assetDir = Join-Path $repoRoot "rust\target\release-assets"
$installerDepsDir = Join-Path $repoRoot "rust\target\installer-deps"
$installerDir = Join-Path $repoRoot "rust\target\installer"

New-Item -ItemType Directory -Force -Path $assetDir | Out-Null

if (-not $NoBuild) {
    Push-Location $desktopDir
    try {
        if (-not $SkipNpmInstall) {
            if (Test-Path -LiteralPath "package-lock.json") {
                Write-Step "installing frontend dependencies"
                npm ci
            } else {
                Write-Step "installing frontend dependencies"
                npm install
            }
        }

        $tauriArgs = @("tauri:build")
        if ($Configuration -eq "debug") {
            $tauriArgs = @("tauri:build:debug")
        }

        Write-Step "building Tauri $Configuration executable"
        & npm run @tauriArgs
    } finally {
        Pop-Location
    }
}

$binDir = Join-Path $repoRoot "target\$Configuration"
$sourceExe = Join-Path $binDir "codexbar-desktop-tauri.exe"
if (-not (Test-Path -LiteralPath $sourceExe)) {
    throw "Missing expected Tauri executable: $sourceExe"
}

$portableExe = Join-Path $assetDir "CodexBar-$appVersion-portable.exe"
Copy-Item -LiteralPath $sourceExe -Destination $portableExe -Force
Save-Checksum -Path $portableExe
Write-Step "portable asset: $portableExe"

if (-not $SkipInstaller) {
    $iscc = Get-IsccPath
    if (-not $iscc) {
        throw "Inno Setup 6 compiler not found. Install Inno Setup or rerun with -SkipInstaller."
    }

    New-Item -ItemType Directory -Force -Path $installerDepsDir, $installerDir | Out-Null
    $vcRedistPath = Join-Path $installerDepsDir "vc_redist.x64.exe"
    $webView2BootstrapperPath = Join-Path $installerDepsDir "MicrosoftEdgeWebview2Setup.exe"

    if (-not (Test-Path -LiteralPath $vcRedistPath)) {
        Write-Step "downloading Microsoft Visual C++ runtime bootstrapper"
        Invoke-WebRequest -Uri "https://aka.ms/vc14/vc_redist.x64.exe" -OutFile $vcRedistPath
    }
    Assert-MicrosoftSignature -Path $vcRedistPath -Label "vc_redist.x64.exe"

    if (-not (Test-Path -LiteralPath $webView2BootstrapperPath)) {
        Write-Step "downloading Microsoft Edge WebView2 bootstrapper"
        Invoke-WebRequest -Uri "https://go.microsoft.com/fwlink/p/?LinkId=2124703" -OutFile $webView2BootstrapperPath
    }
    Assert-MicrosoftSignature -Path $webView2BootstrapperPath -Label "MicrosoftEdgeWebview2Setup.exe"

    $copiedExe = Join-Path $binDir "codexbar.exe"
    Copy-Item -LiteralPath $sourceExe -Destination $copiedExe -Force

    Write-Step "building Inno Setup installer"
    Push-Location (Join-Path $repoRoot "rust\installer")
    try {
        & $iscc `
            /Qp `
            "/DAppVersion=$appVersion" `
            "/DTargetBinDir=..\..\target\$Configuration" `
            "/DVCRedistPath=..\target\installer-deps\vc_redist.x64.exe" `
            "/DWebView2BootstrapperPath=..\target\installer-deps\MicrosoftEdgeWebview2Setup.exe" `
            "/DOutputDir=..\target\installer" `
            "/DOutputBaseFilename=CodexBar-$appVersion-Setup" `
            codexbar.iss
    } finally {
        Pop-Location
    }

    $installer = Join-Path $installerDir "CodexBar-$appVersion-Setup.exe"
    if (-not (Test-Path -LiteralPath $installer)) {
        throw "Expected installer was not created: $installer"
    }

    Copy-Item -LiteralPath $installer -Destination $assetDir -Force
    $assetInstaller = Join-Path $assetDir (Split-Path -Leaf $installer)
    Save-Checksum -Path $assetInstaller
    Write-Step "installer asset: $assetInstaller"
}

Write-Step "assets written to $assetDir"
