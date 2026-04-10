#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: $0 <state-json> <full-screenshot> <output-png>" >&2
  exit 1
fi

state_json="$1"
full_screenshot="$2"
output_png="$3"

if [[ ! -f "$state_json" ]]; then
  echo "State JSON not found: $state_json" >&2
  exit 1
fi

if [[ ! -f "$full_screenshot" ]]; then
  echo "Screenshot not found: $full_screenshot" >&2
  exit 1
fi

for cmd in jq convert; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Required command missing: $cmd" >&2
    exit 1
  fi
done

min_x="$(jq -r '.preferences_viewport_outer_rect.min_x' "$state_json")"
min_y="$(jq -r '.preferences_viewport_outer_rect.min_y' "$state_json")"
max_x="$(jq -r '.preferences_viewport_outer_rect.max_x' "$state_json")"
max_y="$(jq -r '.preferences_viewport_outer_rect.max_y' "$state_json")"

for value in "$min_x" "$min_y" "$max_x" "$max_y"; do
  if [[ "$value" == "null" || -z "$value" ]]; then
    echo "Missing preferences_viewport_outer_rect in $state_json" >&2
    exit 1
  fi
done

crop_x="$(printf '%.0f' "$min_x")"
crop_y="$(printf '%.0f' "$min_y")"
crop_w="$(awk -v min="$min_x" -v max="$max_x" 'BEGIN { printf "%.0f", max - min }')"
crop_h="$(awk -v min="$min_y" -v max="$max_y" 'BEGIN { printf "%.0f", max - min }')"

if [[ "$crop_w" -le 0 || "$crop_h" -le 0 ]]; then
  echo "Invalid crop size: ${crop_w}x${crop_h}" >&2
  exit 1
fi

mkdir -p "$(dirname "$output_png")"
convert "$full_screenshot" -crop "${crop_w}x${crop_h}+${crop_x}+${crop_y}" +repage "$output_png"
echo "$output_png"
