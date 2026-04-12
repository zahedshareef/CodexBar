#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
windows_repo='C:\Users\mac\src\Win-CodexBar'
clean_arg=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean)
      clean_arg=" -CleanBuild"
      shift
      ;;
    *)
      echo "Usage: $0 [--clean]" >&2
      exit 1
      ;;
  esac
done

ps_script="& '$windows_repo\\scripts\\vm\\windows_debug_relaunch.ps1' -RepoRoot '$windows_repo'$clean_arg"
encoded="$(PS_SCRIPT="$ps_script" python3 - <<'PY'
import base64, os
print(base64.b64encode(os.environ["PS_SCRIPT"].encode("utf-16le")).decode())
PY
)"

ssh -o BatchMode=yes mac@imac-ca-mac \
  "/usr/local/bin/prlctl exec 'Windows 11' --current-user cmd /c powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand $encoded"
