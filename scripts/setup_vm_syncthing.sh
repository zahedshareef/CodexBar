#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
folder_id="${CODEXBAR_SYNC_FOLDER_ID:-win-codexbar}"
marker_name="${CODEXBAR_SYNC_MARKER:-.stfolder-win-codexbar}"
local_home="${CODEXBAR_SYNC_LOCAL_HOME:-${XDG_STATE_HOME:-$HOME/.local/state}/win-codexbar-syncthing}"
local_gui="${CODEXBAR_SYNC_LOCAL_GUI:-127.0.0.1:8386}"
windows_home="${CODEXBAR_SYNC_WINDOWS_HOME:-C:\Users\mac\AppData\Local\WinCodexBarSyncthing}"
windows_repo="${CODEXBAR_SYNC_WINDOWS_REPO:-C:\Users\mac\src\Win-CodexBar}"
mac_host="${CODEXBAR_VM_HOST:-mac@imac-ca-mac}"
mac_stage_script="/Users/mac/codexbar-share/tmp-setup-syncthing-windows.ps1"
windows_stage_script='C:\Users\mac\setup-syncthing-windows.ps1'

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

log() {
  printf '==> %s\n' "$*"
}

ps_encode() {
  PS_SCRIPT="$1" python3 - <<'PY'
import base64, os
print(base64.b64encode(os.environ["PS_SCRIPT"].encode("utf-16le")).decode())
PY
}

windows_ps() {
  local script="$1"
  local encoded
  encoded="$(ps_encode "$script")"
  ssh -o BatchMode=yes "$mac_host" \
    "/usr/local/bin/prlctl exec 'Windows 11' --current-user cmd /c powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand $encoded"
}

windows_json() {
  windows_ps "$1" | tr -d '\r' | grep -Eo '\{.*\}' | tail -n 1
}

detect_local_ip() {
  python3 - <<'PY'
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
try:
    s.connect(("8.8.8.8", 80))
    print(s.getsockname()[0])
finally:
    s.close()
PY
}

configure_local_config() {
  local remote_device_id="$1"
  local remote_address="$2"
  export LOCAL_HOME="$local_home"
  export LOCAL_GUI="$local_gui"
  export REPO_ROOT="$repo_root"
  export FOLDER_ID="$folder_id"
  export MARKER_NAME="$marker_name"
  export REMOTE_DEVICE_ID="$remote_device_id"
  export REMOTE_ADDRESS="$remote_address"
  export LOCAL_DEVICE_ID="$local_device_id"
  python3 - <<'PY'
import os
import xml.etree.ElementTree as ET

config_path = os.path.join(os.environ["LOCAL_HOME"], "config.xml")
tree = ET.parse(config_path)
root = tree.getroot()

local_device_id = os.environ["LOCAL_DEVICE_ID"]
remote_device_id = os.environ["REMOTE_DEVICE_ID"]
remote_address = os.environ["REMOTE_ADDRESS"]
repo_root = os.environ["REPO_ROOT"]
folder_id = os.environ["FOLDER_ID"]
marker_name = os.environ["MARKER_NAME"]
local_gui = os.environ["LOCAL_GUI"]

def find_top_device(device_id):
    for node in root.findall("device"):
        if node.get("id") == device_id:
            return node
    return None

def ensure_address(node, value):
    addresses = node.findall("address")
    if addresses:
        addresses[0].text = value
        for extra in addresses[1:]:
            node.remove(extra)
    else:
        addr = ET.SubElement(node, "address")
        addr.text = value

gui = root.find("gui")
if gui is not None:
    address = gui.find("address")
    if address is not None:
        address.text = local_gui

options = root.find("options")
if options is not None:
    start_browser = options.find("startBrowser")
    if start_browser is not None:
        start_browser.text = "false"

for folder in list(root.findall("folder")):
    if folder.get("id") == "default":
        root.remove(folder)

remote_top = find_top_device(remote_device_id)
if remote_top is None:
    remote_top = ET.Element("device", {
        "id": remote_device_id,
        "name": "Windows VM",
        "compression": "metadata",
        "introducer": "false",
        "skipIntroductionRemovals": "false",
        "introducedBy": "",
    })
    ensure_address(remote_top, remote_address)
    for tag, value in [
        ("paused", "false"),
        ("autoAcceptFolders", "false"),
        ("maxSendKbps", "0"),
        ("maxRecvKbps", "0"),
        ("maxRequestKiB", "0"),
        ("untrusted", "false"),
        ("remoteGUIPort", "0"),
        ("numConnections", "0"),
    ]:
        child = ET.SubElement(remote_top, tag)
        child.text = value
    root.insert(len(root.findall("device")) + len(root.findall("folder")), remote_top)
else:
    remote_top.set("name", "Windows VM")
    ensure_address(remote_top, remote_address)

folder = None
for node in root.findall("folder"):
    if node.get("id") == folder_id:
        folder = node
        break
if folder is None:
    defaults_folder = root.find("./defaults/folder")
    folder = ET.fromstring(ET.tostring(defaults_folder))
    root.insert(0, folder)

folder.set("id", folder_id)
folder.set("label", "Win-CodexBar")
folder.set("path", repo_root)
folder.set("type", "sendonly")
folder.set("markerName", marker_name)

for device in list(folder.findall("device")):
    folder.remove(device)

for device_id in (local_device_id, remote_device_id):
    device = ET.SubElement(folder, "device", {"id": device_id, "introducedBy": ""})
    encryption = ET.SubElement(device, "encryptionPassword")
    encryption.text = ""

ET.indent(tree, space="    ")
tree.write(config_path, encoding="utf-8", xml_declaration=False)
PY
}

restart_local_syncthing() {
  local pids
  pids="$(ps -eo pid=,args= | awk -v home="$local_home" '$0 ~ /syncthing serve/ && index($0, home) { print $1 }')"
  if [[ -n "$pids" ]]; then
    while IFS= read -r pid; do
      [[ -n "$pid" ]] || continue
      kill "$pid" 2>/dev/null || true
    done <<<"$pids"
    sleep 1
  fi
  mkdir -p "$local_home"
  nohup syncthing serve --home "$local_home" --no-browser --no-restart \
    >"$local_home/syncthing.log" 2>&1 < /dev/null &
}

validate_sync() {
  local probe_file probe_name remote_probe
  probe_name=".sync-validation-$(date +%s)"
  probe_file="$repo_root/$probe_name"
  remote_probe="${windows_repo}\\${probe_name}"
  printf '%s\n' "$(date --iso-8601=seconds)" >"$probe_file"

  local found=0
  for _ in $(seq 1 60); do
    if [[ "$(windows_ps "Write-Output \$([int](Test-Path '$remote_probe'))" | tr -d '\r' | tail -n 1)" == "1" ]]; then
      found=1
      break
    fi
    sleep 1
  done

  rm -f "$probe_file"

  if [[ "$found" != "1" ]]; then
    echo "Syncthing did not propagate the probe file to Windows within 60 seconds." >&2
    exit 1
  fi
}

need_cmd syncthing
need_cmd ssh
need_cmd scp
need_cmd python3

mkdir -p "$local_home" "$repo_root/$marker_name"

if [[ ! -f "$local_home/config.xml" ]]; then
  log "Generating local Syncthing home at $local_home"
  syncthing generate --home "$local_home" >/dev/null
fi

local_device_id="$(syncthing serve --home "$local_home" --device-id | tr -d '\r\n')"
local_address="${CODEXBAR_SYNC_LOCAL_ADDRESS:-dynamic}"
windows_address="${CODEXBAR_SYNC_WINDOWS_ADDRESS:-dynamic}"
log "Local device ID: $local_device_id"

log "Copying Windows Syncthing helper"
scp "$repo_root/scripts/vm/setup_syncthing_windows.ps1" "$mac_host:$mac_stage_script" >/dev/null
windows_ps "Copy-Item '\\\\Mac\\codexbarshare\\tmp-setup-syncthing-windows.ps1' '$windows_stage_script' -Force" >/dev/null

log "Configuring Windows Syncthing instance"
windows_result="$(windows_json "& '$windows_stage_script' -LocalDeviceId '$local_device_id' -LocalAddress '$local_address' -RepoPath '$windows_repo' -HomeDir '$windows_home' -FolderId '$folder_id' -MarkerName '$marker_name'")"
if [[ -z "$windows_result" ]]; then
  echo "Failed to capture Windows Syncthing setup result." >&2
  exit 1
fi

remote_device_id="$(WINDOWS_JSON="$windows_result" python3 - <<'PY'
import json, os
print(json.loads(os.environ["WINDOWS_JSON"])["deviceId"])
PY
)"
log "Configuring local Syncthing instance"
configure_local_config "$remote_device_id" "$windows_address"

log "Restarting local Syncthing"
restart_local_syncthing

log "Validating end-to-end sync"
validate_sync

printf '\n'
printf 'Local Syncthing home: %s\n' "$local_home"
printf 'Windows repo path: %s\n' "$windows_repo"
printf 'Remote device ID: %s\n' "$remote_device_id"
printf 'Status: ready\n'
