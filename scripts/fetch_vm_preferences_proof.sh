#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "Usage: $0 <provider> <host-capture-png> <state-json-out> <cropped-png-out>" >&2
  exit 1
fi

provider="$1"
host_capture_png="$2"
state_json_out="$3"
cropped_png_out="$4"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crop_script="$script_dir/crop_vm_preferences_proof.sh"

if [[ ! -x "$crop_script" ]]; then
  echo "Crop script is missing or not executable: $crop_script" >&2
  exit 1
fi

mkdir -p "$(dirname "$state_json_out")"
mkdir -p "$(dirname "$cropped_png_out")"

guest_state_path="C:\\\\Users\\\\mac\\\\Desktop\\\\${provider}-state.json"
guest_interactive_capture_path="C:\\\\Users\\\\mac\\\\Desktop\\\\${provider}-interactive-full.png"
state_fetched=0
guest_capture_tmp="$(mktemp "${TMPDIR:-/tmp}/${provider}-interactive-full-XXXXXX.png")"
cleanup() {
  rm -f "$guest_capture_tmp"
}
trap cleanup EXIT

image_mean() {
  local image_path="$1"
  convert "$image_path" -colorspace Gray -format '%[fx:mean]\n' info:
}

build_encoded_ps_command() {
  local ps_command="$1"
  python3 - "$ps_command" <<'PY'
import base64
import sys

print(base64.b64encode(sys.argv[1].encode("utf-16le")).decode())
PY
}

fetch_guest_binary_file() {
  local guest_path="$1"
  local local_path="$2"
  local encoded
  encoded="$(build_encoded_ps_command "[Convert]::ToBase64String([IO.File]::ReadAllBytes('$guest_path'))")"
  if ssh mac@imac-ca-mac \
    '"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl" exec "Windows 11" powershell -EncodedCommand '"$encoded"'' \
    2>/dev/null | tr -d '\r\n' | base64 -d > "$local_path"; then
    [[ -s "$local_path" ]]
    return
  fi
  return 1
}

for _ in $(seq 1 30); do
  if ssh mac@imac-ca-mac \
    '"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl" exec "Windows 11" cmd /c type '"$guest_state_path"'' \
    > "$state_json_out" 2>/dev/null; then
    state_fetched=1
    break
  fi
  sleep 1
done

if [[ "$state_fetched" -ne 1 ]]; then
  echo "Guest state file did not appear in time: $guest_state_path" >&2
  exit 1
fi

crop_source_png="$host_capture_png"
if fetch_guest_binary_file "$guest_interactive_capture_path" "$guest_capture_tmp"; then
  guest_capture_mean="$(image_mean "$guest_capture_tmp")"
  if awk -v mean="$guest_capture_mean" 'BEGIN { exit !(mean > 0.01) }'; then
    crop_source_png="$guest_capture_tmp"
  fi
fi

"$crop_script" "$state_json_out" "$crop_source_png" "$cropped_png_out" >/dev/null

echo "$cropped_png_out"
