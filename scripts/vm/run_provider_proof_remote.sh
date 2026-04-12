#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: $0 <launch-profile> <proof-name> [selected-provider]" >&2
  exit 1
fi

launch_profile="$1"
proof_name="$2"
selected_provider="${3:-$proof_name}"
clean_build_arg=""
if [[ "${CODEXBAR_PROOF_CLEAN_BUILD:-0}" == "1" ]]; then
  clean_build_arg=" -CleanBuild"
fi
skip_build_arg=""
if [[ "${CODEXBAR_PROOF_SKIP_BUILD:-0}" == "1" ]]; then
  skip_build_arg=" -SkipBuild"
fi
skip_mirror_arg=""
if [[ "${CODEXBAR_PROOF_SKIP_MIRROR:-0}" == "1" ]]; then
  skip_mirror_arg=" -SkipMirror"
fi
capture_mode="${CODEXBAR_PROOF_CAPTURE_MODE:-provider}"
preferences_tab="${CODEXBAR_PROOF_PREFERENCES_TAB:-providers}"
menu_selected_tab="${CODEXBAR_PROOF_MENU_SELECTED_TAB:-}"
use_local_vm_repo="${CODEXBAR_PROOF_USE_LOCAL_VM_REPO:-0}"

prlctl='"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl"'

script_path='C:\Users\mac\provider-proof.ps1'
if [[ "$use_local_vm_repo" == "1" ]]; then
  script_path='C:\Users\mac\src\Win-CodexBar\scripts\vm\provider_osclick_proof_unc.ps1'
else
  ssh mac@imac-ca-mac \
    "$prlctl exec \"Windows 11\" powershell -ExecutionPolicy Bypass -Command \"Copy-Item '\\\\\\\\Mac\\codexbarshare\\tmp-provider-osclick-proof-unc.ps1' '$script_path' -Force\""
fi

ps_script="& '$script_path' -LaunchProfile '$launch_profile' -ProofName '$proof_name' -SelectedProvider '$selected_provider' -CaptureMode '$capture_mode' -MenuSelectedTab '$menu_selected_tab' -PreferencesTab '$preferences_tab'$clean_build_arg$skip_build_arg$skip_mirror_arg"
encoded_command="$(PS_SCRIPT="$ps_script" python3 - <<'PY'
import base64, os
print(base64.b64encode(os.environ["PS_SCRIPT"].encode("utf-16le")).decode())
PY
)"

timeout --foreground 420 \
  ssh mac@imac-ca-mac \
    "$prlctl exec \"Windows 11\" --current-user cmd /c powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand $encoded_command"
