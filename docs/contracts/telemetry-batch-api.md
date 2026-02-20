# `POST /v1/telemetry/batches`

Status: Draft v2 (locked schema + idempotency envelope checks)  
Date: 2026-02-20  
Purpose: Ingest iOS OBD telemetry in idempotent batches (default 60-second cadence).

## Request JSON

```json
{
  "batch_id": "uuid",
  "schema_version": "0.2",
  "vehicle_uid": "uuid",
  "source": "OBD",
  "client": {
    "platform": "ios",
    "app_version": "1.0.0",
    "adapter_fingerprint": "sha256_hex"
  },
  "capture_window": {
    "started_at": "2026-02-17T10:30:00Z",
    "ended_at": "2026-02-17T10:31:00Z",
    "sample_interval_seconds": 60
  },
  "records": [
    {
      "observed_at": "2026-02-17T10:30:05Z",
      "signal_key": "ev.soc_pct",
      "value_number": 71.2,
      "unit": "%",
      "status": "ok",
      "confidence": 0.96,
      "source_signal": "pid_or_oem_key"
    }
  ],
  "session_events": [
    {
      "event_type": "drive_session_start",
      "observed_at": "2026-02-17T10:30:00Z",
      "session_id": "uuid"
    }
  ],
  "diagnostics": [
    {
      "observed_at": "2026-02-17T10:30:10Z",
      "mil_on": false,
      "dtcs_active": []
    }
  ]
}
```

## Required Fields
- `batch_id`
- `schema_version`
- `vehicle_uid`
- `source`
- `capture_window.started_at`
- `capture_window.ended_at`
- `records` (can be empty only when sending session events/diagnostics)

## Locked Validation Rules
- `schema_version` must be `0.2`.
- `source` must be `OBD` in MVP.
- `client.platform` (when provided) must be `ios`.
- `capture_window.ended_at` must be after `capture_window.started_at`.
- `capture_window.sample_interval_seconds` (when provided) must be between configured min/max upload interval bounds.
- `capture_window` duration must not exceed configured max upload interval.
- `records[*].observed_at`, `session_events[*].observed_at`, and `diagnostics[*].observed_at` must be within capture window.
- `records[*].signal_key` must exist in active signal registry version.
- `records[*].status` enum: `ok`, `stale`, `unavailable`, `not_supported`, `permission_denied`, `error`.
- `records[*].confidence` range: `0.0` to `1.0`.
- Only one of `value_number`, `value_string`, `value_bool`, `value_json` may be set per record.
- For status `ok` or `stale`, exactly one value field must be present.
- `records[*].temperature_bin` (when provided) must be one of: `very_cold`, `cold`, `cool`, `mild`, `hot`.
- `session_events[*].event_type` allowed values:
  - `drive_session_start`
  - `drive_session_stop`
  - `charging_session_start`
  - `charging_session_stop`
- Maximum records per batch: 5,000.

## Idempotency Strategy
- `batch_id` is the idempotency key.
- If `batch_id` already exists and envelope fields match:
  - `vehicle_uid`
  - `schema_version`
  - `source`
  - `capture_window.started_at`
  - `capture_window.ended_at`
  then response is `200` with `duplicate=true`.
- If `batch_id` exists but envelope fields differ, response is `409 conflict`.

## Response (200)

```json
{
  "accepted": true,
  "batch_id": "uuid",
  "ingest_id": "uuid",
  "duplicate": false,
  "records_received": 120,
  "records_accepted": 118,
  "records_rejected": 2,
  "errors": [
    {
      "record_index": 27,
      "code": "unknown_signal_key",
      "message": "signal_key not present in registry"
    }
  ],
  "next_upload_after_seconds": 60
}
```

Duplicate replay example:
- `/Users/albinocordeiro/Code/car_ranks/docs/contracts/examples/telemetry-batch-duplicate-response.json`

## Error Responses
- `400` invalid payload
- `409` duplicate `batch_id` with mismatched payload envelope
- `413` payload too large
- `500` ingest failure

Conflict example:
- `/Users/albinocordeiro/Code/car_ranks/docs/contracts/examples/telemetry-batch-conflict-response.json`

## Storage Mapping
- `records` -> `vehicle_signal_observation`
- `diagnostics` -> `vehicle_diagnostic_event`
- session events -> `vehicle_session_event`
