#!/usr/bin/env bash
set -euo pipefail

# This script captures a complete local Postgres smoke baseline under docs/smoke.
# It is intentionally explicit so each stage is easy to review and debug.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SMOKE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$SMOKE_DIR/../.." && pwd)"
BACKEND_DIR="$REPO_ROOT/backend"
TEMPLATE_DIR="$SMOKE_DIR/templates"

ENV_FILE="$BACKEND_DIR/.env.staging"
API_BASE="http://127.0.0.1:18080"
OUTPUT_ROOT="$SMOKE_DIR"
KEEP_BACKEND_LOG="0"

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Options:
  --env-file <path>      Env file that defines DATABASE_URL (default: backend/.env.staging)
  --api-base <url>       Backend base URL (default: http://127.0.0.1:18080)
  --output-root <path>   Directory where run snapshots are written (default: docs/smoke)
  --keep-backend-log     Keep backend.log in the run folder
  --help                 Show this help message
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --env-file)
      ENV_FILE="$2"
      shift 2
      ;;
    --api-base)
      API_BASE="$2"
      shift 2
      ;;
    --output-root)
      OUTPUT_ROOT="$2"
      shift 2
      ;;
    --keep-backend-log)
      KEEP_BACKEND_LOG="1"
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_file() {
  if [ ! -f "$1" ]; then
    echo "Missing required file: $1" >&2
    exit 1
  fi
}

# Validate all external tools up front so failures happen early.
require_cmd cargo
require_cmd curl
require_cmd jq
require_cmd uuidgen

require_file "$ENV_FILE"
require_file "$TEMPLATE_DIR/telemetry-cold-template.json"
require_file "$TEMPLATE_DIR/telemetry-mild-template.json"

RUN_ID="postgres-local-$(date -u +%Y%m%dT%H%M%SZ)"
RUN_DIR="$OUTPUT_ROOT/$RUN_ID"
mkdir -p "$RUN_DIR"

# Import runtime DB configuration used by the backend process and run metadata.
set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

if [ -z "${DATABASE_URL:-}" ]; then
  echo "DATABASE_URL is required in $ENV_FILE" >&2
  exit 1
fi

# Map api base to bind addr so the server listens where the captures expect.
BIND_ADDR="$(printf '%s' "$API_BASE" | sed -E 's#^https?://([^/]+).*$#\1#')"
export BIND_ADDR

USER_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
VEHICLE_UID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
COLD_BATCH_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
MILD_BATCH_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
COLD_SESSION_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
MILD_SESSION_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"

materialize_payload() {
  local template_file="$1"
  local out_file="$2"
  local batch_id="$3"
  local vehicle_uid="$4"
  local session_id="$5"

  jq \
    --arg batch_id "$batch_id" \
    --arg vehicle_uid "$vehicle_uid" \
    --arg session_id "$session_id" \
    '
    .batch_id=$batch_id
    | .vehicle_uid=$vehicle_uid
    | .records |= map(if has("session_id") then .session_id=$session_id else . end)
    | .session_events |= map(.session_id=$session_id)
    ' \
    "$template_file" > "$out_file"
}

materialize_payload \
  "$TEMPLATE_DIR/telemetry-cold-template.json" \
  "$RUN_DIR/telemetry-cold.json" \
  "$COLD_BATCH_ID" \
  "$VEHICLE_UID" \
  "$COLD_SESSION_ID"

materialize_payload \
  "$TEMPLATE_DIR/telemetry-mild-template.json" \
  "$RUN_DIR/telemetry-mild.json" \
  "$MILD_BATCH_ID" \
  "$VEHICLE_UID" \
  "$MILD_SESSION_ID"

(
  cd "$BACKEND_DIR"
  cargo run > "$RUN_DIR/backend.log" 2>&1
) &
SERVER_PID=$!

cleanup() {
  if kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# Allow the server process to boot before captures start.
READY=0
for _ in $(seq 1 90); do
  http_code="$(curl -s -o /dev/null -w "%{http_code}" "$API_BASE/health" || true)"
  if [ "$http_code" = "200" ]; then
    READY=1
    break
  fi
  sleep 1
done

if [ "$READY" != "1" ]; then
  echo "Backend failed to become ready; inspect $RUN_DIR/backend.log" >&2
  exit 1
fi

capture() {
  local output_file="$1"
  shift
  local tmp_body
  tmp_body="$(mktemp)"
  local http_code
  http_code="$(curl -sS -o "$tmp_body" -w "%{http_code}" "$@")"
  cat "$tmp_body" > "$output_file"
  printf "\n__STATUS__:%s\n" "$http_code" >> "$output_file"
  rm -f "$tmp_body"
}

# Capture all baseline endpoints in a deterministic order.
capture "$RUN_DIR/01-health.txt" "$API_BASE/health"
capture "$RUN_DIR/02-config-sampling.txt" "$API_BASE/v1/config/sampling"
capture "$RUN_DIR/03-ingest-cold.txt" \
  -X POST "$API_BASE/v1/telemetry/batches" \
  -H "x-user-id: $USER_ID" \
  -H "content-type: application/json" \
  --data "@$RUN_DIR/telemetry-cold.json"
capture "$RUN_DIR/04-ingest-mild.txt" \
  -X POST "$API_BASE/v1/telemetry/batches" \
  -H "x-user-id: $USER_ID" \
  -H "content-type: application/json" \
  --data "@$RUN_DIR/telemetry-mild.json"
capture "$RUN_DIR/05-recompute-kpis.txt" -X POST "$API_BASE/internal/jobs/recompute-kpis"
capture "$RUN_DIR/06-latest-job.txt" "$API_BASE/internal/jobs/latest?job_kind=recompute_kpis"
capture "$RUN_DIR/07-kpis-me.txt" \
  -H "x-user-id: $USER_ID" \
  "$API_BASE/v1/kpis/me?vehicle_uid=$VEHICLE_UID&timeframe=90d"
capture "$RUN_DIR/08-kpis-charging.txt" \
  -H "x-user-id: $USER_ID" \
  "$API_BASE/v1/kpis/charging?vehicle_uid=$VEHICLE_UID&timeframe=90d&temperature_bin=all"
capture "$RUN_DIR/09-kpis-readiness.txt" \
  -H "x-user-id: $USER_ID" \
  "$API_BASE/v1/kpis/readiness?vehicle_uid=$VEHICLE_UID&timeframe=90d"
capture "$RUN_DIR/10-kpis-temperature-impact.txt" \
  -H "x-user-id: $USER_ID" \
  "$API_BASE/v1/kpis/temperature-impact?vehicle_uid=$VEHICLE_UID&timeframe=90d&baseline_temperature_bin=mild&compare_temperature_bin=cold"
capture "$RUN_DIR/11-rankings-range.txt" \
  -H "x-user-id: $USER_ID" \
  "$API_BASE/v1/rankings?ranking_type=ev_range_efficiency&timeframe=90d&limit=10"
capture "$RUN_DIR/12-rankings-charging.txt" \
  -H "x-user-id: $USER_ID" \
  "$API_BASE/v1/rankings?ranking_type=ev_charging_performance&timeframe=90d&limit=10"
capture "$RUN_DIR/13-rankings-composite.txt" \
  -H "x-user-id: $USER_ID" \
  "$API_BASE/v1/rankings?ranking_type=ev_composite&timeframe=90d&limit=10"
capture "$RUN_DIR/14-rankings-temperature.txt" \
  -H "x-user-id: $USER_ID" \
  "$API_BASE/v1/rankings?ranking_type=ev_temperature_impact&timeframe=90d&temperature_bin=cold&limit=10"

mask_database_url() {
  printf '%s' "$1" | sed -E 's#(postgres(ql)?://[^:]+:)[^@]+#\1***#'
}

MASKED_DATABASE_URL="$(mask_database_url "$DATABASE_URL")"
CREATED_AT_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

cat > "$RUN_DIR/run-meta.txt" <<META
snapshot_dir=$RUN_DIR
user_id=$USER_ID
vehicle_uid=$VEHICLE_UID
database_url=$MASKED_DATABASE_URL
api_base=$API_BASE
created_at_utc=$CREATED_AT_UTC
META

status_line() {
  local file="$1"
  local code
  code="$(tail -n 1 "$RUN_DIR/$file" | cut -d: -f2)"
  printf -- "- \`%s\`: \`%s\`\n" "$file" "$code"
}

LATEST_JOB_JSON="$(head -n 1 "$RUN_DIR/06-latest-job.txt")"
CHARGING_UPSERTED="$(printf '%s' "$LATEST_JOB_JSON" | jq -r '.charging_sessions_upserted // "n/a"')"
KPI_UPSERTED="$(printf '%s' "$LATEST_JOB_JSON" | jq -r '.kpi_rows_upserted // "n/a"')"
RANKING_UPSERTED="$(printf '%s' "$LATEST_JOB_JSON" | jq -r '.ranking_rows_upserted // "n/a"')"
RECOMPUTED_VEHICLES="$(printf '%s' "$LATEST_JOB_JSON" | jq -r '.recomputed_vehicles // "n/a"')"

{
  echo "# Postgres Smoke Summary"
  echo
  echo "## Run Metadata"
  echo
  echo "- Executed at: \`$CREATED_AT_UTC\`"
  echo "- API base: \`$API_BASE\`"
  echo "- Vehicle uid: \`$VEHICLE_UID\`"
  echo "- User id: \`$USER_ID\`"
  echo "- Runtime DB: local staging Postgres (\`$MASKED_DATABASE_URL\`)"
  echo
  echo "## Endpoint Status"
  echo
  status_line "01-health.txt"
  status_line "02-config-sampling.txt"
  status_line "03-ingest-cold.txt"
  status_line "04-ingest-mild.txt"
  status_line "05-recompute-kpis.txt"
  status_line "06-latest-job.txt"
  status_line "07-kpis-me.txt"
  status_line "08-kpis-charging.txt"
  status_line "09-kpis-readiness.txt"
  status_line "10-kpis-temperature-impact.txt"
  status_line "11-rankings-range.txt"
  status_line "12-rankings-charging.txt"
  status_line "13-rankings-composite.txt"
  status_line "14-rankings-temperature.txt"
  echo
  echo "## Key Outcomes"
  echo
  echo "- Internal recompute job response captured in \`06-latest-job.txt\`."
  echo "- Job output counts:"
  echo "  - \`charging_sessions_upserted=$CHARGING_UPSERTED\`"
  echo "  - \`kpi_rows_upserted=$KPI_UPSERTED\`"
  echo "  - \`ranking_rows_upserted=$RANKING_UPSERTED\`"
  echo "  - \`recomputed_vehicles=$RECOMPUTED_VEHICLES\`"
  echo "- Public KPI and ranking reads were captured for the seeded vehicle/user scope."
  echo
  echo "## Captured Artifacts"
  echo
  echo "- Requests:"
  echo "  - \`$RUN_DIR/telemetry-cold.json\`"
  echo "  - \`$RUN_DIR/telemetry-mild.json\`"
  echo "- Endpoint responses:"
  for file in \
    01-health.txt \
    02-config-sampling.txt \
    03-ingest-cold.txt \
    04-ingest-mild.txt \
    05-recompute-kpis.txt \
    06-latest-job.txt \
    07-kpis-me.txt \
    08-kpis-charging.txt \
    09-kpis-readiness.txt \
    10-kpis-temperature-impact.txt \
    11-rankings-range.txt \
    12-rankings-charging.txt \
    13-rankings-composite.txt \
    14-rankings-temperature.txt \
    run-meta.txt; do
    echo "  - \`$RUN_DIR/$file\`"
  done
  if [ "$KEEP_BACKEND_LOG" = "1" ]; then
    echo "  - \`$RUN_DIR/backend.log\`"
  fi
} > "$RUN_DIR/summary.md"

if [ "$KEEP_BACKEND_LOG" != "1" ]; then
  rm -f "$RUN_DIR/backend.log"
fi

echo "Run complete"
echo "RUN_ID=$RUN_ID"
echo "RUN_DIR=$RUN_DIR"
