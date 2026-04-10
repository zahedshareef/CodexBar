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

prlctl='"/Applications/Parallels Desktop.app/Contents/MacOS/prlctl"'

ssh mac@imac-ca-mac \
  "$prlctl exec \"Windows 11\" powershell -ExecutionPolicy Bypass -Command \"Copy-Item '\\\\\\\\Mac\\codexbarshare\\tmp-provider-osclick-proof-unc.ps1' 'C:\\Users\\mac\\provider-proof.ps1' -Force\""

timeout --foreground 420 \
  ssh mac@imac-ca-mac \
    "$prlctl exec \"Windows 11\" powershell -ExecutionPolicy Bypass -Command \"& 'C:\\Users\\mac\\provider-proof.ps1' -LaunchProfile '$launch_profile' -ProofName '$proof_name' -SelectedProvider '$selected_provider'$clean_build_arg\""
