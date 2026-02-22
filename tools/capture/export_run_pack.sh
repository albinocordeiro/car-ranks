#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  tools/capture/export_run_pack.sh \
    --api-base <url> \
    --user-id <uuid> \
    --vehicle-uid <uuid> \
    (--batch-id <uuid> | --session-id <uuid>) \
    --output-dir <path>

Description:
  Pulls paged raw telemetry for one capture scope (batch or session), then writes
  a reusable capture pack containing raw dumps, merged data, fixture outputs, and summaries.
USAGE
}

API_BASE=""
USER_ID=""
VEHICLE_UID=""
BATCH_ID=""
SESSION_ID=""
OUTPUT_ROOT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --api-base)
      API_BASE="${2:-}"
      shift 2
      ;;
    --user-id)
      USER_ID="${2:-}"
      shift 2
      ;;
    --vehicle-uid)
      VEHICLE_UID="${2:-}"
      shift 2
      ;;
    --batch-id)
      BATCH_ID="${2:-}"
      shift 2
      ;;
    --session-id)
      SESSION_ID="${2:-}"
      shift 2
      ;;
    --output-dir)
      OUTPUT_ROOT="${2:-}"
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

if [[ -z "$API_BASE" || -z "$USER_ID" || -z "$VEHICLE_UID" || -z "$OUTPUT_ROOT" ]]; then
  echo "Missing required arguments." >&2
  usage
  exit 1
fi

if [[ -n "$BATCH_ID" && -n "$SESSION_ID" ]]; then
  echo "Use either --batch-id or --session-id, not both." >&2
  exit 1
fi

if [[ -z "$BATCH_ID" && -z "$SESSION_ID" ]]; then
  echo "One scope selector is required: --batch-id or --session-id." >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required." >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required." >&2
  exit 1
fi

validate_uuid() {
  local value="$1"
  local label="$2"
  local uuid_regex='^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
  if [[ ! "$value" =~ $uuid_regex ]]; then
    echo "Invalid $label: $value" >&2
    exit 1
  fi
}

validate_json_query() {
  local file_path="$1"
  local query="$2"
  local description="$3"

  if ! jq -e "$query" "$file_path" >/dev/null 2>&1; then
    echo "Validation failed for $description in $file_path" >&2
    exit 1
  fi
}

if [[ ! "$API_BASE" =~ ^https?:// ]]; then
  echo "--api-base must start with http:// or https://." >&2
  exit 1
fi

validate_uuid "$USER_ID" "--user-id"
validate_uuid "$VEHICLE_UID" "--vehicle-uid"
if [[ -n "$BATCH_ID" ]]; then
  validate_uuid "$BATCH_ID" "--batch-id"
fi
if [[ -n "$SESSION_ID" ]]; then
  validate_uuid "$SESSION_ID" "--session-id"
fi

url_encode() {
  jq -nr --arg v "$1" '$v|@uri'
}

selector_kind="batch"
selector_value="$BATCH_ID"
if [[ -n "$SESSION_ID" ]]; then
  selector_kind="session"
  selector_value="$SESSION_ID"
fi

timestamp="$(date -u +%Y%m%d-%H%M%S)"
pack_id="${timestamp}-${selector_kind}-${selector_value}"
pack_dir="${OUTPUT_ROOT%/}/${pack_id}"
raw_pages_dir="$pack_dir/raw-telemetry-pages"
mkdir -p "$raw_pages_dir"

build_query() {
  local cursor_observed_at="${1:-}"
  local cursor_observation_id="${2:-}"

  local query="vehicle_uid=$(url_encode "$VEHICLE_UID")"
  query+="&limit=500"
  query+="&include_session_events=true"

  if [[ -n "$BATCH_ID" ]]; then
    query+="&batch_id=$(url_encode "$BATCH_ID")"
  fi
  if [[ -n "$SESSION_ID" ]]; then
    query+="&session_id=$(url_encode "$SESSION_ID")"
  fi

  if [[ -n "$cursor_observed_at" || -n "$cursor_observation_id" ]]; then
    query+="&cursor_observed_at=$(url_encode "$cursor_observed_at")"
    query+="&cursor_observation_id=$(url_encode "$cursor_observation_id")"
  fi

  echo "$query"
}

page_files=()
page_number=1
cursor_observed_at=""
cursor_observation_id=""

while true; do
  query="$(build_query "$cursor_observed_at" "$cursor_observation_id")"
  url="${API_BASE%/}/v1/telemetry/raw?${query}"
  page_file="$(printf "%s/page-%04d.json" "$raw_pages_dir" "$page_number")"

  http_status="$(curl -sS -H "x-user-id: ${USER_ID}" -w "%{http_code}" "$url" -o "$page_file")"
  if [[ "$http_status" -lt 200 || "$http_status" -gt 299 ]]; then
    echo "Raw telemetry request failed (HTTP ${http_status})." >&2
    echo "URL: $url" >&2
    cat "$page_file" >&2
    exit 1
  fi

  page_files+=("$page_file")

  returned_count="$(jq -r '.returned_count // 0' "$page_file")"
  next_observed="$(jq -r '.next_cursor_observed_at // empty' "$page_file")"
  next_observation="$(jq -r '.next_cursor_observation_id // empty' "$page_file")"

  if [[ "$returned_count" -eq 0 ]]; then
    break
  fi

  if [[ -z "$next_observed" || -z "$next_observation" ]]; then
    break
  fi

  cursor_observed_at="$next_observed"
  cursor_observation_id="$next_observation"
  page_number=$((page_number + 1))

  # Hard stop to avoid infinite loops if a server bug returns repeating cursors.
  if [[ "$page_number" -gt 2000 ]]; then
    echo "Aborting after 2000 pages; cursor loop guard triggered." >&2
    exit 1
  fi
done

if [[ "${#page_files[@]}" -eq 0 ]]; then
  echo "No pages were fetched." >&2
  exit 1
fi

aggregated_path="$pack_dir/aggregated-raw-telemetry.json"

jq -s '
  def rows: map(.rows // []) | add;
  {
    generated_at: (now | todateiso8601),
    vehicle_uid: (.[0].vehicle_uid // null),
    batch_id: (.[0].batch_id // null),
    session_id: (.[0].session_id // null),
    include_session_events: (.[0].include_session_events // false),
    pages: length,
    returned_count: (rows | length),
    rows: rows
  }
' "${page_files[@]}" > "$aggregated_path"

row_count="$(jq -r '.rows | length' "$aggregated_path")"
page_count="${#page_files[@]}"
newest_observed_at="$(jq -r '.rows | if length == 0 then null else .[0].observed_at end' "$aggregated_path")"
oldest_observed_at="$(jq -r '.rows | if length == 0 then null else .[-1].observed_at end' "$aggregated_path")"

run_meta_path="$pack_dir/run-meta.json"

jq -n \
  --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg api_base "${API_BASE%/}" \
  --arg user_id "$USER_ID" \
  --arg vehicle_uid "$VEHICLE_UID" \
  --arg selector_kind "$selector_kind" \
  --arg selector_value "$selector_value" \
  --arg pack_id "$pack_id" \
  --arg newest_observed_at "$newest_observed_at" \
  --arg oldest_observed_at "$oldest_observed_at" \
  --argjson page_count "$page_count" \
  --argjson row_count "$row_count" \
  '{
    generated_at: $generated_at,
    api_base: $api_base,
    user_id: $user_id,
    vehicle_uid: $vehicle_uid,
    selector: {
      kind: $selector_kind,
      value: $selector_value
    },
    pack_id: $pack_id,
    page_count: $page_count,
    row_count: $row_count,
    newest_observed_at: (if $newest_observed_at == "null" then null else $newest_observed_at end),
    oldest_observed_at: (if $oldest_observed_at == "null" then null else $oldest_observed_at end)
  }' > "$run_meta_path"

ios_fixture_path="$pack_dir/ios-command-fixture.json"

jq '
  {
    generated_at: (now | todateiso8601),
    vehicle_uid,
    batch_id,
    session_id,
    exchanges: [
      .rows[]
      | {
          observation_id,
          observed_at,
          signal_key,
          source_signal,
          status,
          raw_payload_ref,
          command: (try ((.raw_payload_ref // "") | capture("cmd=(?<cmd>[^ ]+)").cmd) catch null),
          response: (try ((.raw_payload_ref // "") | capture("resp=(?<resp>.*)$").resp) catch null)
        }
    ]
  }
' "$aggregated_path" > "$ios_fixture_path"

backend_fixture_path="$pack_dir/backend-ingest-fixture.json"

jq '
  def as_signal_record:
    select((.signal_key | startswith("session.")) | not)
    | {
        observed_at,
        signal_key,
        value_number,
        value_string,
        value_bool,
        value_json,
        unit: null,
        status,
        confidence: null,
        source_signal,
        session_id,
        raw_payload_ref
      };

  def as_session_event:
    select(.signal_key | startswith("session."))
    | {
        event_type: .status,
        observed_at,
        session_id,
        raw_payload_ref
      };

  {
    batch_id: (.batch_id // "00000000-0000-0000-0000-000000000000"),
    schema_version: "0.2",
    vehicle_uid,
    source: "OBD",
    client: {
      platform: "ios",
      app_version: "fixture-export",
      adapter_fingerprint: "fixture-export"
    },
    capture_window: {
      started_at: (.rows | if length == 0 then null else .[-1].observed_at end),
      ended_at: (.rows | if length == 0 then null else .[0].observed_at end),
      sample_interval_seconds: 60
    },
    records: [.rows[] | as_signal_record],
    session_events: [.rows[] | as_session_event],
    diagnostics: []
  }
' "$aggregated_path" > "$backend_fixture_path"

summary_path="$pack_dir/signal-summary.md"

signal_row_count="$(jq -r '[.rows[] | select((.signal_key | startswith("session.")) | not)] | length' "$aggregated_path")"
session_row_count="$(jq -r '[.rows[] | select(.signal_key | startswith("session."))] | length' "$aggregated_path")"
no_data_count="$(jq -r '[.rows[] | select(((.raw_payload_ref // "") | ascii_upcase | contains("NO DATA")) or .status == "unavailable")] | length' "$aggregated_path")"
error_count="$(jq -r '[.rows[] | select(.status == "error")] | length' "$aggregated_path")"
signal_ok_count="$(jq -r '[.rows[] | select((.signal_key | startswith("session.")) | not) | select(.status == "ok")] | length' "$aggregated_path")"

if [[ "$row_count" -gt 0 ]]; then
  no_data_ratio="$(awk -v n="$no_data_count" -v d="$row_count" 'BEGIN { printf "%.2f", (n/d)*100 }')"
else
  no_data_ratio="0.00"
fi

if [[ "$signal_row_count" -gt 0 ]]; then
  signal_availability_ratio="$(awk -v n="$signal_ok_count" -v d="$signal_row_count" 'BEGIN { printf "%.2f", (n/d)*100 }')"
else
  signal_availability_ratio="0.00"
fi

{
  cat <<HEADER
# Signal Summary

- Generated (UTC): $(date -u +%Y-%m-%dT%H:%M:%SZ)
- Total rows: $row_count
- Signal rows: $signal_row_count
- Session-event rows: $session_row_count
- Signal availability ratio (status=ok): $signal_ok_count/$signal_row_count (${signal_availability_ratio}%)
- Rows with NO DATA/unavailable: $no_data_count (${no_data_ratio}%)
- Rows with status=error: $error_count

## Top Signal Keys
HEADER

  jq -r '
    [.rows[] | .signal_key]
    | group_by(.)
    | map({key: .[0], count: length})
    | sort_by(-.count, .key)
    | .[0:20]
    | .[]
    | "- \(.key): \(.count)"
  ' "$aggregated_path"

  echo
  echo "## Top Command Cluster Hints"
  jq -r '
    [.rows[]
      | (try ((.raw_payload_ref // "") | capture("cmd=(?<cmd>[^ ]+)").cmd) catch "unknown")
    ]
    | group_by(.)
    | map({cmd: .[0], count: length})
    | sort_by(-.count, .cmd)
    | .[0:20]
    | .[]
    | "- \(.cmd): \(.count)"
  ' "$aggregated_path"

  echo
  echo "## NO DATA Command Clusters"
  jq -r '
    [.rows[]
      | select(((.raw_payload_ref // "") | ascii_upcase | contains("NO DATA")) or .status == "unavailable")
      | (try ((.raw_payload_ref // "") | capture("cmd=(?<cmd>[^ ]+)").cmd) catch "unknown")
    ]
    | group_by(.)
    | map({cmd: .[0], count: length})
    | sort_by(-.count, .cmd)
    | .[0:20]
    | if length == 0 then ["- none"] else map("- \(.cmd): \(.count)") end
    | .[]
  ' "$aggregated_path"

  echo
  echo "## Error Command Clusters"
  jq -r '
    [.rows[]
      | select(.status == "error")
      | (try ((.raw_payload_ref // "") | capture("cmd=(?<cmd>[^ ]+)").cmd) catch "unknown")
    ]
    | group_by(.)
    | map({cmd: .[0], count: length})
    | sort_by(-.count, .cmd)
    | .[0:20]
    | if length == 0 then ["- none"] else map("- \(.cmd): \(.count)") end
    | .[]
  ' "$aggregated_path"
} > "$summary_path"

manifest_path="$pack_dir/manifest.md"

cat > "$manifest_path" <<MANIFEST
# Capture Run Pack Manifest

- Pack ID: $pack_id
- Generated (UTC): $(date -u +%Y-%m-%dT%H:%M:%SZ)
- API Base: ${API_BASE%/}
- Vehicle: $VEHICLE_UID
- Selector: $selector_kind=$selector_value
- Pages fetched: $page_count
- Rows aggregated: $row_count

## Artifacts

1. run-meta.json
2. aggregated-raw-telemetry.json
3. raw-telemetry-pages/
4. ios-command-fixture.json
5. backend-ingest-fixture.json
6. signal-summary.md

## Notes

- Cursor pagination was followed until no next cursor remained.
- `ios-command-fixture.json` keeps command/response timeline hints for parser replay.
- `backend-ingest-fixture.json` is a normalized candidate payload for backend test seeding.
MANIFEST

validate_json_query "$run_meta_path" '
  has("generated_at")
  and has("selector")
  and has("pack_id")
  and has("page_count")
  and has("row_count")
' "run metadata structure"

validate_json_query "$aggregated_path" '
  has("vehicle_uid")
  and has("rows")
  and (.rows | type == "array")
  and has("returned_count")
' "aggregated raw telemetry structure"

validate_json_query "$ios_fixture_path" '
  has("exchanges")
  and (.exchanges | type == "array")
  and (.exchanges | all(has("command")))
' "iOS command fixture structure"

validate_json_query "$backend_fixture_path" '
  has("schema_version")
  and has("records")
  and has("session_events")
  and (.records | type == "array")
' "backend ingest fixture structure"

if [[ ! -s "$manifest_path" || ! -s "$summary_path" ]]; then
  echo "Validation failed: manifest.md or signal-summary.md is empty." >&2
  exit 1
fi

echo "Capture pack written to: $pack_dir"
