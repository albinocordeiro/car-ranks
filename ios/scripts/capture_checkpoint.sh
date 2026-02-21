#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  ios/scripts/capture_checkpoint.sh --checkpoint <name> --device <simulator-name> --mode <mock|live>

Options:
  --checkpoint   Required checkpoint label (example: batch-a-initial)
  --device       Optional simulator name (default: iPhone 16 Pro)
  --mode         Optional data source mode: mock or live (default: mock)
USAGE
}

CHECKPOINT=""
DEVICE="iPhone 16 Pro"
MODE="mock"
LIVE_CAPTURE_OVERRIDE_MODE="none"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --checkpoint)
      CHECKPOINT="${2:-}"
      shift 2
      ;;
    --device)
      DEVICE="${2:-}"
      shift 2
      ;;
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$CHECKPOINT" ]]; then
  echo "Missing required --checkpoint." >&2
  usage
  exit 1
fi

if [[ "$MODE" != "mock" && "$MODE" != "live" ]]; then
  echo "Invalid --mode '$MODE'. Use mock or live." >&2
  exit 1
fi

if [[ "$MODE" == "live" ]]; then
  LIVE_CAPTURE_OVERRIDE_MODE="force-states"
fi

if ! command -v xcodegen >/dev/null 2>&1; then
  echo "xcodegen is required. Install with: brew install xcodegen" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IOS_DIR="$ROOT_DIR/ios"
REVIEWS_DIR="$ROOT_DIR/docs/ios-ui-reviews"
DERIVED_DATA_DIR="$IOS_DIR/.derivedData"
PROJECT_PATH="$IOS_DIR/CarRanksApp.xcodeproj"
BUNDLE_ID="com.albinocordeiro.carranks"
TIMESTAMP="$(date -u +%Y%m%d-%H%M%S)"
OUTPUT_DIR="$REVIEWS_DIR/${TIMESTAMP}-${CHECKPOINT}"

mkdir -p "$OUTPUT_DIR"

find_device_udid() {
  local device_name="$1"
  local line
  line="$(xcrun simctl list devices available | rg -m1 "^[[:space:]]*${device_name} \(" || true)"
  if [[ -z "$line" ]]; then
    return 1
  fi
  echo "$line" | sed -E 's/.*\(([0-9A-F-]+)\).*/\1/'
}

UDID="$(find_device_udid "$DEVICE")" || {
  echo "Could not find an available simulator named '$DEVICE'." >&2
  xcrun simctl list devices available >&2
  exit 1
}

echo "Generating iOS project..."
(
  cd "$IOS_DIR"
  xcodegen generate >/dev/null
)

echo "Booting simulator: $DEVICE ($UDID)"
xcrun simctl boot "$UDID" >/dev/null 2>&1 || true
xcrun simctl bootstatus "$UDID" -b >/dev/null

echo "Building app for simulator..."
xcodebuild \
  -project "$PROJECT_PATH" \
  -scheme CarRanksApp \
  -destination "id=$UDID" \
  -derivedDataPath "$DERIVED_DATA_DIR" \
  build >/tmp/car_ranks_ios_build.log

APP_PATH="$DERIVED_DATA_DIR/Build/Products/Debug-iphonesimulator/CarRanksApp.app"
if [[ ! -d "$APP_PATH" ]]; then
  echo "App build output not found at: $APP_PATH" >&2
  echo "See /tmp/car_ranks_ios_build.log for details." >&2
  exit 1
fi

xcrun simctl uninstall "$UDID" "$BUNDLE_ID" >/dev/null 2>&1 || true
xcrun simctl install "$UDID" "$APP_PATH" >/dev/null

resolved_filename() {
  local base_name="$1"
  if [[ "$MODE" == "live" ]]; then
    echo "${base_name%.png}-live.png"
  else
    echo "$base_name"
  fi
}

capture_screen() {
  local base_name="$1"
  local scenario="$2"
  local settle_seconds="$3"
  local output_name

  output_name="$(resolved_filename "$base_name")"

  SIMCTL_CHILD_CAPTURE_SCENARIO="$scenario" \
  SIMCTL_CHILD_DATA_SOURCE_MODE="$MODE" \
  SIMCTL_CHILD_LIVE_CAPTURE_OVERRIDE_MODE="$LIVE_CAPTURE_OVERRIDE_MODE" \
  xcrun simctl launch \
    --terminate-running-process \
    "$UDID" \
    "$BUNDLE_ID" \
    --capture-scenario "$scenario" \
    --data-source-mode "$MODE" \
    --live-capture-override "$LIVE_CAPTURE_OVERRIDE_MODE" >/dev/null

  sleep "$settle_seconds"
  xcrun simctl io "$UDID" screenshot "$OUTPUT_DIR/$output_name" >/dev/null
  echo "Captured $output_name"
}

declare -a CAPTURE_ITEMS=()
if [[ "$CHECKPOINT" == *"batch-c"* ]]; then
  CAPTURE_ITEMS=(
    "01-rankings-loading.png|rankings-loading|1|Rankings loading"
    "02-rankings-success.png|rankings-success|2|Rankings success"
    "03-rankings-empty.png|rankings-empty|2|Rankings empty"
    "04-rankings-error.png|rankings-error|2|Rankings error"
    "05-dev-session-panel.png|dev-session|1|Dev Session Panel"
  )
elif [[ "$CHECKPOINT" == *"batch-b"* ]]; then
  CAPTURE_ITEMS=(
    "01-kpi-charging-loading.png|kpi-charging-loading|1|KPI Charging loading"
    "02-kpi-charging-success.png|kpi-charging-success|2|KPI Charging success"
    "03-kpi-charging-empty.png|kpi-charging-empty|2|KPI Charging empty"
    "04-kpi-charging-error.png|kpi-charging-error|2|KPI Charging error"
    "05-kpi-readiness-loading.png|kpi-readiness-loading|1|KPI Readiness loading"
    "06-kpi-readiness-success.png|kpi-readiness-success|2|KPI Readiness success"
    "07-kpi-readiness-empty.png|kpi-readiness-empty|2|KPI Readiness empty"
    "08-kpi-readiness-error.png|kpi-readiness-error|2|KPI Readiness error"
    "09-kpi-temperature-impact-loading.png|kpi-temperature-impact-loading|1|KPI Temperature Impact loading"
    "10-kpi-temperature-impact-success.png|kpi-temperature-impact-success|2|KPI Temperature Impact success"
    "11-kpi-temperature-impact-empty.png|kpi-temperature-impact-empty|2|KPI Temperature Impact empty"
    "12-kpi-temperature-impact-error.png|kpi-temperature-impact-error|2|KPI Temperature Impact error"
    "13-dev-session-panel.png|dev-session|1|Dev Session Panel"
  )
else
  CAPTURE_ITEMS=(
    "01-kpi-me-loading.png|kpi-me-loading|1|KPI Me loading"
    "02-kpi-me-success.png|kpi-me-success|2|KPI Me success"
    "03-kpi-me-empty.png|kpi-me-empty|2|KPI Me empty"
    "04-kpi-me-error.png|kpi-me-error|2|KPI Me error"
    "05-dev-session-panel.png|dev-session|1|Dev Session Panel"
  )
fi

declare -a SCREEN_FILES=()
declare -a COVERAGE_LINES=()

for item in "${CAPTURE_ITEMS[@]}"; do
  IFS="|" read -r file_name scenario settle_seconds coverage_label <<<"$item"
  capture_screen "$file_name" "$scenario" "$settle_seconds"
  resolved_name="$(resolved_filename "$file_name")"
  SCREEN_FILES+=("$resolved_name")
  COVERAGE_LINES+=("$coverage_label: $resolved_name")
done

{
cat <<MANIFEST
# iOS UI Checkpoint Manifest

- Timestamp (UTC): $TIMESTAMP
- Checkpoint: $CHECKPOINT
- Device: $DEVICE
- Mode: $MODE
- Live Capture Override: $LIVE_CAPTURE_OVERRIDE_MODE

## Screen List
MANIFEST

index=1
for screen_file in "${SCREEN_FILES[@]}"; do
  echo "$index. $screen_file"
  index=$((index + 1))
done

echo
echo "## State Coverage"
for coverage_line in "${COVERAGE_LINES[@]}"; do
  echo "- $coverage_line"
done

cat <<'MANIFEST'

## Acceptance Notes
- Pending review.
- Record final visual decisions and unresolved issues in this section.
MANIFEST
} > "$OUTPUT_DIR/manifest.md"

echo "Checkpoint pack ready: $OUTPUT_DIR"
