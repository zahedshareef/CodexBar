#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 3 ]]; then
  echo "Usage: $0 <launch-profile> [date-stamp] [proof-name]" >&2
  exit 1
fi

provider="$1"
date_stamp="${2:-$(date +%Y%m%d)}"
proof_name="${3:-$provider}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
proof_dir="/tmp/win-codexbar-settings"
lock_dir="/tmp/win-codexbar-proof.lock"
min_capture_mean="0.20"
min_lower_half_mean="0.08"
mkdir -p "$proof_dir"

case "$provider" in
  claude)
    ;;
  cursor)
    ;;
  codex)
    ;;
  kiro)
    ;;
  opencode)
    ;;
  copilot)
    ;;
  factory)
    ;;
  kimi)
    ;;
  gemini)
    ;;
  minimax)
    ;;
  antigravity)
    ;;
  vertexai)
    ;;
  augment)
    ;;
  zai)
    ;;
  kimik2)
    ;;
  amp)
    ;;
  openrouter)
    ;;
  warp)
    ;;
  jetbrains)
    ;;
  alibaba)
    ;;
  ollama)
    ;;
  synthetic)
    ;;
  nanogpt)
    ;;
  infini)
    ;;
  *)
    echo "Unsupported provider: $provider" >&2
    exit 1
    ;;
esac

sync_repo() {
  rsync -az --delete \
    --exclude '.git' \
    --exclude 'rust/target' \
    "$repo_root/" \
    mac@imac-ca-mac:/Users/mac/codexbar-share/repo/
}

sync_share_root_scripts() {
  local provider_script="$repo_root/scripts/vm/provider_osclick_proof_unc.ps1"
  if [[ -f "$provider_script" ]]; then
    scp "$provider_script" "mac@imac-ca-mac:/Users/mac/codexbar-share/tmp-provider-osclick-proof-unc.ps1" >/dev/null
  fi
}

clear_guest_proof_artifacts() {
  ssh mac@imac-ca-mac \
    '"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl" exec "Windows 11" powershell -NoProfile -Command "Remove-Item -ErrorAction SilentlyContinue C:\\Users\\mac\\Desktop\\'"${proof_name}"'-ready.txt,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-state.json,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-interactive-full.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-preferences-proof.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-osclick-proof.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-window-capture.log"' \
    >/dev/null 2>&1 || true
}

run_guest_proof() {
  (cd "$repo_root" && bash scripts/vm/run_provider_proof_remote.sh "$provider" "$proof_name") || true
}

guest_ready_marker_exists() {
  ssh mac@imac-ca-mac \
    '"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl" exec "Windows 11" cmd /c type C:\\Users\\mac\\Desktop\\'"${proof_name}"'-ready.txt' \
    >/dev/null 2>&1
}

wait_for_guest_ready_marker() {
  local _attempt
  for _attempt in $(seq 1 120); do
    if guest_ready_marker_exists; then
      return 0
    fi
    sleep 1
  done
  return 1
}

capture_host_batch() {
  local i out remote_out
  for i in 1 2 3 4 5 6; do
    out="${proof_dir}/${proof_name}-proof-host-${date_stamp}-auto${i}.png"
    remote_out="/Users/mac/codexbar-share/proofs/${proof_name}-proof-host-${date_stamp}-auto${i}.png"
    ssh mac@imac-ca-mac \
      '"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl" capture "Windows 11" --file '"$remote_out"'' \
      >/dev/null
    scp "mac@imac-ca-mac:${remote_out}" "$out" >/dev/null
    echo "$out"
    sleep 3
  done
}

capture_mean() {
  local candidate="$1"
  convert "$candidate" -colorspace Gray -format '%[fx:mean]\n' info:
}

capture_lower_half_mean() {
  local candidate="$1"
  convert "$candidate" -colorspace Gray -gravity south -crop 100%x50%+0+0 +repage -format '%[fx:mean]\n' info:
}

pick_best_capture() {
  local best=""
  local best_size=0
  local best_mean="0"
  local best_lower_half_mean="0"
  local candidate size mean lower_half_mean
  while IFS= read -r candidate; do
    [[ -n "$candidate" ]] || continue
    size="$(stat -c '%s' "$candidate")"
    mean="$(capture_mean "$candidate" 2>/dev/null || printf '0')"
    lower_half_mean="$(capture_lower_half_mean "$candidate" 2>/dev/null || printf '0')"

    if awk -v mean="$mean" -v min="$min_capture_mean" -v lower="$lower_half_mean" -v lower_min="$min_lower_half_mean" 'BEGIN { exit !(mean >= min && lower >= lower_min) }'; then
      if [[ -z "$best" ]] \
        || awk -v mean="$mean" -v best="$best_mean" 'BEGIN { exit !(mean > best) }' \
        || { awk -v mean="$mean" -v best="$best_mean" 'BEGIN { exit !(mean == best) }' && awk -v lower="$lower_half_mean" -v best_lower="$best_lower_half_mean" 'BEGIN { exit !(lower > best_lower) }'; } \
        || { awk -v mean="$mean" -v best="$best_mean" 'BEGIN { exit !(mean == best) }' && awk -v lower="$lower_half_mean" -v best_lower="$best_lower_half_mean" 'BEGIN { exit !(lower == best_lower) }' && [[ "$size" -gt "$best_size" ]]; }; then
        best="$candidate"
        best_size="$size"
        best_mean="$mean"
        best_lower_half_mean="$lower_half_mean"
      fi
    elif [[ -z "$best" && "$size" -gt "$best_size" ]]; then
      best="$candidate"
      best_size="$size"
      best_mean="$mean"
      best_lower_half_mean="$lower_half_mean"
    fi
  done
  [[ -n "$best" ]] || return 1
  printf '%s\n' "$best"
}

while ! mkdir "$lock_dir" 2>/dev/null; do
  sleep 1
done
trap 'rmdir "$lock_dir"' EXIT

sync_repo
sync_share_root_scripts
clear_guest_proof_artifacts
run_guest_proof &
guest_pid=$!
wait_for_guest_ready_marker || true
sleep 2
captures="$(capture_host_batch)"
wait "$guest_pid" || true

best_capture="$(printf '%s\n' "$captures" | pick_best_capture)"

state_json="${proof_dir}/${proof_name}-state-${date_stamp}.json"
cropped_png="${proof_dir}/${proof_name}-preferences-crop-${date_stamp}.png"
"$repo_root/scripts/fetch_vm_preferences_proof.sh" \
  "$proof_name" \
  "$best_capture" \
  "$state_json" \
  "$cropped_png" >/dev/null

echo "best_capture=$best_capture"
echo "state_json=$state_json"
echo "preferences_crop=$cropped_png"
