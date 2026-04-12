#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <launch-profile> <tab> [tab...]" >&2
  exit 1
fi

launch_profile="$1"
shift

date_stamp="${CODEXBAR_BATCH_DATE:-$(date +%Y%m%d)}"
proof_prefix="${CODEXBAR_BATCH_PREFIX:-fast-${launch_profile}}"
first=1

for tab in "$@"; do
  proof_name="${proof_prefix}-${tab}"
  skip_build=1
  if [[ "$first" == "1" ]]; then
    skip_build=0
    first=0
  fi

  CODEXBAR_PROOF_USE_LOCAL_VM_REPO=1 \
  CODEXBAR_PROOF_SKIP_SYNC=1 \
  CODEXBAR_PROOF_SKIP_MIRROR=1 \
  CODEXBAR_PROOF_SKIP_BUILD="$skip_build" \
  CODEXBAR_PROOF_CAPTURE_MODE=tab \
  CODEXBAR_PROOF_PREFERENCES_TAB="$tab" \
  bash "$repo_root/scripts/run_vm_provider_proof.sh" "$launch_profile" "$date_stamp" "$proof_name"
done
