#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 3 ]]; then
  echo "Usage: $0 <launch-profile> [date-stamp] [proof-name]" >&2
  echo "  Set CODEXBAR_PROOF_SHELL=tauri (default) or egui to select the shell." >&2
  exit 1
fi

provider="$1"
date_stamp="${2:-$(date +%Y%m%d)}"
proof_name="${3:-$provider}"

# Shell selection: "tauri" (default) or "egui" (legacy).
proof_shell="${CODEXBAR_PROOF_SHELL:-tauri}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
proof_dir="/tmp/win-codexbar-settings"
lock_dir="/tmp/win-codexbar-proof.lock"
min_capture_mean="0.20"
min_lower_half_mean="0.08"
capture_mode="${CODEXBAR_PROOF_CAPTURE_MODE:-provider}"
requested_tab="${CODEXBAR_PROOF_PREFERENCES_TAB:-providers}"
min_crop_mean="0.05"
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
  if [[ "${CODEXBAR_PROOF_SKIP_SYNC:-0}" == "1" ]]; then
    return 0
  fi
  rsync -az --delete \
    --exclude '.git' \
    --exclude 'rust/target' \
    --exclude 'target' \
    --exclude 'apps/desktop-tauri/node_modules' \
    --exclude 'apps/desktop-tauri/dist' \
    "$repo_root/" \
    mac@imac-ca-mac:/Users/mac/codexbar-share/repo/
}

sync_share_root_scripts() {
  if [[ "${CODEXBAR_PROOF_USE_LOCAL_VM_REPO:-0}" == "1" ]]; then
    return 0
  fi
  local provider_script="$repo_root/scripts/vm/provider_osclick_proof_unc.ps1"
  if [[ -f "$provider_script" ]]; then
    scp "$provider_script" "mac@imac-ca-mac:/Users/mac/codexbar-share/tmp-provider-osclick-proof-unc.ps1" >/dev/null
  fi
  local tauri_script="$repo_root/scripts/vm/tauri_proof.ps1"
  if [[ -f "$tauri_script" ]]; then
    scp "$tauri_script" "mac@imac-ca-mac:/Users/mac/codexbar-share/tmp-tauri-proof.ps1" >/dev/null
  fi
}

clear_guest_proof_artifacts() {
  ssh mac@imac-ca-mac \
    '"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl" exec "Windows 11" powershell -NoProfile -Command "Remove-Item -ErrorAction SilentlyContinue C:\\Users\\mac\\Desktop\\'"${proof_name}"'-ready.txt,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-state.json,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-interactive-full.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-menu-full.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-preferences-proof.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-osclick-proof.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-window-capture.log,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-desktop-full.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-window-capture.png,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-tauri-capture.log,C:\\Users\\mac\\Desktop\\'"${proof_name}"'-nircmd-full.png"' \
    >/dev/null 2>&1 || true
}

run_guest_proof() {
  (cd "$repo_root" && bash scripts/vm/run_provider_proof_remote.sh "$provider" "$proof_name" "$provider") || true
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

fetch_guest_file() {
  local guest_path="$1"
  local local_path="$2"
  local prlctl='"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl"'

  ssh mac@imac-ca-mac \
    "$prlctl exec \"Windows 11\" powershell -NoProfile -Command \"[Convert]::ToBase64String([IO.File]::ReadAllBytes('$guest_path'))\"" \
    | tr -d '\r\n' \
    | base64 -d >"$local_path"
}

validate_state_json() {
  local json_path="$1"
  if [[ ! -s "$json_path" ]]; then
    echo "PROOF REJECTED: state JSON is empty or missing: $json_path" >&2
    return 1
  fi
  if ! jq -e '.preferences_viewport_outer_rect' "$json_path" >/dev/null 2>&1; then
    echo "PROOF REJECTED: state JSON missing preferences_viewport_outer_rect" >&2
    return 1
  fi
}

validate_state_tab() {
  local json_path="$1"
  local expected_tab="$2"
  if [[ -z "$expected_tab" || "$expected_tab" == "providers" ]]; then
    return 0
  fi
  local actual_tab
  actual_tab="$(jq -r '.preferences_tab // empty' "$json_path" 2>/dev/null || true)"
  if [[ -z "$actual_tab" ]]; then
    echo "WARNING: state JSON has no preferences_tab field — cannot verify tab match" >&2
    return 0
  fi
  if [[ "$actual_tab" != "$expected_tab" ]]; then
    echo "PROOF REJECTED: tab mismatch — requested '$expected_tab' but state says '$actual_tab'" >&2
    return 1
  fi
}

validate_image_brightness() {
  local image_path="$1"
  local label="$2"
  local threshold="${3:-$min_crop_mean}"
  if [[ ! -s "$image_path" ]]; then
    echo "PROOF REJECTED: $label is empty or missing: $image_path" >&2
    return 1
  fi
  local mean
  mean="$(capture_mean "$image_path" 2>/dev/null || printf '0')"
  if awk -v mean="$mean" -v min="$threshold" 'BEGIN { exit !(mean < min) }'; then
    echo "PROOF REJECTED: $label is too dark (mean=$mean, threshold=$threshold)" >&2
    return 1
  fi
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
if ! wait_for_guest_ready_marker; then
  echo "WARNING: guest ready marker did not appear within 120s — captures may be stale" >&2
fi
sleep 2
captures="$(capture_host_batch)"
wait "$guest_pid" || true

# ── Tauri shell proof path ──────────────────────────────────────────────────
# The Tauri app uses the CODEXBAR_PROOF_MODE env-var harness to show each
# surface (trayPanel, popOut, settings, settings tabs) visibly for capture.
if [[ "$proof_shell" == "tauri" ]]; then
  best_capture="$(printf '%s\n' "$captures" | pick_best_capture || true)"

  # Collect per-surface proof artifacts produced inside the guest.
  proof_surfaces=("trayPanel" "popOut" "settings-general" "settings-apiKeys" "settings-cookies" "settings-about")
  for surf in "${proof_surfaces[@]}"; do
    win_png="${proof_dir}/${proof_name}-${surf}-window-${date_stamp}.png"
    fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-${surf}-window.png" "$win_png" 2>/dev/null || true
    desk_png="${proof_dir}/${proof_name}-${surf}-desktop-${date_stamp}.png"
    fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-${surf}-desktop.png" "$desk_png" 2>/dev/null || true
    if [[ -s "$win_png" ]]; then
      echo "${surf}_window=$win_png"
    fi
    if [[ -s "$desk_png" ]]; then
      echo "${surf}_desktop=$desk_png"
    fi
  done

  # Also fetch legacy single-capture files if present.
  window_png="${proof_dir}/${proof_name}-window-capture-${date_stamp}.png"
  fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-window-capture.png" "$window_png" 2>/dev/null || true
  desktop_png="${proof_dir}/${proof_name}-desktop-full-${date_stamp}.png"
  fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-desktop-full.png" "$desktop_png" 2>/dev/null || true

  echo "shell=tauri"
  echo "proof_harness=CODEXBAR_PROOF_MODE"
  if [[ -n "${best_capture:-}" ]]; then
    echo "best_host_capture=$best_capture"
  fi
  if [[ -s "$window_png" ]]; then
    echo "window_capture=$window_png"
  fi
  if [[ -s "$desktop_png" ]]; then
    echo "desktop_capture=$desktop_png"
  fi
  exit 0
fi

# ── Legacy egui proof path (unchanged) ──────────────────────────────────────

validate_state_json_menu() {
  local json_path="$1"
  if [[ ! -s "$json_path" ]]; then
    echo "PROOF REJECTED: state JSON is empty or missing: $json_path" >&2
    return 1
  fi
}

if [[ "$capture_mode" == "menu" ]]; then
  state_json="${proof_dir}/${proof_name}-state-${date_stamp}.json"
  menu_png="${proof_dir}/${proof_name}-menu-full-${date_stamp}.png"
  fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-state.json" "$state_json"
  fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-menu-full.png" "$menu_png"
  validate_state_json_menu "$state_json"
  validate_image_brightness "$menu_png" "menu capture" "$min_capture_mean"
  echo "state_json=$state_json"
  echo "menu_full=$menu_png"
  exit 0
fi

if [[ "$capture_mode" == "popout" ]]; then
  state_json="${proof_dir}/${proof_name}-state-${date_stamp}.json"
  popout_png="${proof_dir}/${proof_name}-interactive-full-${date_stamp}.png"
  popout_log="${proof_dir}/${proof_name}-window-capture-${date_stamp}.log"
  fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-state.json" "$state_json"
  fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-interactive-full.png" "$popout_png"
  fetch_guest_file "C:\\Users\\mac\\Desktop\\${proof_name}-window-capture.log" "$popout_log" 2>/dev/null || true
  validate_state_json_menu "$state_json"
  validate_image_brightness "$popout_png" "popout capture" "$min_capture_mean"
  echo "state_json=$state_json"
  echo "popout_full=$popout_png"
  if [[ -s "$popout_log" ]]; then
    echo "window_capture_log=$popout_log"
  fi
  exit 0
fi

best_capture="$(printf '%s\n' "$captures" | pick_best_capture)"

state_json="${proof_dir}/${proof_name}-state-${date_stamp}.json"
cropped_png="${proof_dir}/${proof_name}-preferences-crop-${date_stamp}.png"
"$repo_root/scripts/fetch_vm_preferences_proof.sh" \
  "$proof_name" \
  "$best_capture" \
  "$state_json" \
  "$cropped_png" >/dev/null

validate_state_json "$state_json"
validate_state_tab "$state_json" "$requested_tab"
validate_image_brightness "$cropped_png" "cropped preferences" "$min_crop_mean"

echo "best_capture=$best_capture"
echo "state_json=$state_json"
echo "preferences_crop=$cropped_png"
