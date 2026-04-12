param(
  [Parameter(Mandatory = $true)]
  [string]$LocalDeviceId,
  [Parameter(Mandatory = $true)]
  [string]$LocalAddress,
  [string]$RepoPath = 'C:\Users\mac\src\Win-CodexBar',
  [string]$HomeDir = "$env:LOCALAPPDATA\WinCodexBarSyncthing",
  [string]$FolderId = 'win-codexbar',
  [string]$MarkerName = '.stfolder-win-codexbar',
  [string]$GuiAddress = '127.0.0.1:8385'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-SyncthingExe {
  $command = Get-Command syncthing -ErrorAction SilentlyContinue
  if ($command) {
    return $command.Source
  }

  $candidates = @(
    "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\Syncthing.Syncthing_*",
    "$env:LOCALAPPDATA\Programs\Syncthing\syncthing.exe",
    'C:\Program Files\Syncthing\syncthing.exe'
  )

  foreach ($candidate in $candidates) {
    if ($candidate -like '*_*') {
      $packageRoot = Get-ChildItem $candidate -Directory -ErrorAction SilentlyContinue | Select-Object -First 1
      if ($packageRoot) {
        $exe = Get-ChildItem $packageRoot.FullName -Filter 'syncthing.exe' -Recurse -File -ErrorAction SilentlyContinue |
          Select-Object -First 1 -ExpandProperty FullName
        if ($exe) {
          return $exe
        }
      }
      continue
    }

    if (Test-Path $candidate) {
      return $candidate
    }
  }

  return $null
}

function Ensure-SyncthingInstalled {
  $exe = Resolve-SyncthingExe
  if ($exe) {
    return $exe
  }

  winget install --id Syncthing.Syncthing -e --accept-package-agreements --accept-source-agreements --disable-interactivity
  $exe = Resolve-SyncthingExe
  if (-not $exe) {
    throw 'Syncthing installation completed, but syncthing.exe could not be located.'
  }

  return $exe
}

function Get-PrimaryIPv4 {
  $addresses = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object {
    $_.IPAddress -ne '127.0.0.1' -and
    $_.IPAddress -notlike '169.254*' -and
    $_.PrefixOrigin -ne 'WellKnown'
  }

  $selected = $addresses | Sort-Object InterfaceMetric, SkipAsSource | Select-Object -First 1
  if ($selected) {
    return $selected.IPAddress
  }

  throw 'Could not determine a usable Windows IPv4 address for Syncthing.'
}

function Ensure-TopDevice {
  param(
    [xml]$Config,
    [string]$DeviceId,
    [string]$Name,
    [string]$Address
  )

  $device = $Config.SelectNodes('/configuration/device') | Where-Object { $_.id -eq $DeviceId } | Select-Object -First 1
  if (-not $device) {
    $deviceTemplate = $Config.SelectSingleNode('/configuration/defaults/device')
    if (-not $deviceTemplate) {
      throw 'Syncthing config is missing defaults/device.'
    }
    $device = $deviceTemplate.CloneNode($true)
    $device.SetAttribute('id', $DeviceId)
    $Config.configuration.AppendChild($device) | Out-Null
  }

  $device.SetAttribute('name', $Name)
  $addressNode = $device.SelectSingleNode('address')
  if (-not $addressNode) {
    $addressNode = $Config.CreateElement('address')
    $device.AppendChild($addressNode) | Out-Null
  }
  $addressNode.InnerText = $Address
}

function Ensure-FolderConfig {
  param(
    [xml]$Config,
    [string]$SelfDeviceId,
    [string]$RemoteDeviceId
  )

  $folder = $Config.SelectNodes('/configuration/folder') | Where-Object { $_.id -eq $FolderId } | Select-Object -First 1
  if (-not $folder) {
    $folderTemplate = $Config.SelectSingleNode('/configuration/defaults/folder')
    if (-not $folderTemplate) {
      throw 'Syncthing config is missing defaults/folder.'
    }
    $folder = $folderTemplate.CloneNode($true)
    $Config.configuration.AppendChild($folder) | Out-Null
  }

  $folder.SetAttribute('id', $FolderId)
  $folder.SetAttribute('label', 'Win-CodexBar')
  $folder.SetAttribute('path', $RepoPath)
  $folder.SetAttribute('type', 'receiveonly')
  $folder.SetAttribute('markerName', $MarkerName)

  @($folder.SelectNodes('device')) | ForEach-Object { $folder.RemoveChild($_) | Out-Null }
  foreach ($deviceId in @($SelfDeviceId, $RemoteDeviceId)) {
    $deviceNode = $Config.CreateElement('device')
    $deviceNode.SetAttribute('id', $deviceId)
    $deviceNode.SetAttribute('introducedBy', '')
    $encryption = $Config.CreateElement('encryptionPassword')
    $deviceNode.AppendChild($encryption) | Out-Null
    $folder.AppendChild($deviceNode) | Out-Null
  }
}

function Restart-SyncthingProcess {
  param(
    [string]$ExePath
  )

  $existing = Get-CimInstance Win32_Process -Filter "Name = 'syncthing.exe'" | Where-Object {
    $_.CommandLine -like "*$HomeDir*"
  }

  foreach ($proc in $existing) {
    Stop-Process -Id $proc.ProcessId -Force -ErrorAction SilentlyContinue
  }

  Start-Process -FilePath $ExePath -ArgumentList @('serve', '--home', $HomeDir, '--no-browser', '--no-restart') -WindowStyle Hidden | Out-Null
}

$exePath = Ensure-SyncthingInstalled
New-Item -ItemType Directory -Force -Path $HomeDir | Out-Null
New-Item -ItemType Directory -Force -Path $RepoPath | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $RepoPath $MarkerName) | Out-Null

$configPath = Join-Path $HomeDir 'config.xml'
if (-not (Test-Path $configPath)) {
  & $exePath generate --home $HomeDir | Out-Null
}

$selfDeviceId = (& $exePath device-id --home $HomeDir).Trim()
[xml]$config = Get-Content $configPath

$config.configuration.gui.address = $GuiAddress
$config.configuration.options.startBrowser = 'false'

@($config.SelectNodes('/configuration/folder')) | Where-Object { $_.id -eq 'default' } | ForEach-Object {
  $config.configuration.RemoveChild($_) | Out-Null
}

Ensure-TopDevice -Config $config -DeviceId $LocalDeviceId -Name 'Linux Host' -Address $LocalAddress
Ensure-FolderConfig -Config $config -SelfDeviceId $selfDeviceId -RemoteDeviceId $LocalDeviceId

$config.Save($configPath)

Restart-SyncthingProcess -ExePath $exePath

$result = @{
  deviceId = $selfDeviceId
  address = "tcp://$(Get-PrimaryIPv4):22000"
  homeDir = $HomeDir
  repoPath = $RepoPath
}

[Console]::WriteLine(($result | ConvertTo-Json -Compress))
