#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$RUST_DIR/.." && pwd)"
VERSION="${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$RUST_DIR/Cargo.toml" | head -n 1)}"
TARGET_TRIPLE="${INSTALLER_TARGET_TRIPLE:-x86_64-pc-windows-gnu}"
TARGET_BIN_DIR="$RUST_DIR/target/$TARGET_TRIPLE/release"
OUTPUT_DIR="$RUST_DIR/target/installer"
INSTALLER_DEPS_DIR="$RUST_DIR/target/installer-deps"
INSTALLER_PATH="$OUTPUT_DIR/CodexBar-${VERSION}-Setup.exe"
INNO_IMAGE="${INNO_SETUP_IMAGE:-amake/innosetup}"
CONTAINER_NAME="codexbar-inno-${VERSION//./-}-$$"
VC_REDIST_URL="${VC_REDIST_URL:-https://aka.ms/vc14/vc_redist.x64.exe}"
VC_REDIST_PATH="$INSTALLER_DEPS_DIR/vc_redist.x64.exe"

for required_file in "$TARGET_BIN_DIR/codexbar.exe" "$RUST_DIR/icons/icon.ico"; do
  if [[ ! -f "$required_file" ]]; then
    echo "Missing required build artifact: $required_file" >&2
    echo "Build the Windows target first, then rerun this script." >&2
    exit 1
  fi
done

mkdir -p "$OUTPUT_DIR"
mkdir -p "$INSTALLER_DEPS_DIR"

curl -L "$VC_REDIST_URL" -o "$VC_REDIST_PATH"

cleanup() {
  docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker create \
  --name "$CONTAINER_NAME" \
  -v "$REPO_ROOT:/work" \
  -w /work/rust/installer \
  --entrypoint /bin/sh \
  "$INNO_IMAGE" \
  -lc "/opt/bin/iscc /Qp /DAppVersion=$VERSION /DTargetBinDir=..\\\\target\\\\$TARGET_TRIPLE\\\\release /DVCRedistPath=..\\\\target\\\\installer-deps\\\\vc_redist.x64.exe /DOutputDir=C:\\\\inno-out /DOutputBaseFilename=CodexBar-$VERSION-Setup codexbar.iss" \
  >/dev/null

docker start -a "$CONTAINER_NAME" >/dev/null
docker cp \
  "$CONTAINER_NAME:/home/xclient/.wine/drive_c/inno-out/CodexBar-$VERSION-Setup.exe" \
  "$INSTALLER_PATH"

if [[ ! -f "$INSTALLER_PATH" ]]; then
  echo "Expected installer was not created: $INSTALLER_PATH" >&2
  exit 1
fi

printf '%s\n' "$INSTALLER_PATH"
