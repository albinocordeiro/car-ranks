# `POST /v1/telemetry/batches`

Status: Draft v1  
Purpose: Ingest iOS OBD telemetry in idempotent batches (default 60-second cadence).

## Request JSON

```json
{
  "batch_id": "uuid",
  "schema_version": "1.0",
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
    "sample_interval_seconds": 5
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
    },
    {
      "event_type": "charging_session_stop",
      "observed_at": "2026-02-17T10:31:00Z",
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

## Validation Rules
- `source` must be `OBD` in MVP.
- `batch_id` is idempotency key. Duplicate `batch_id` returns success with `duplicate=true`.
- `records[*].signal_key` must exist in active signal registry version (v0.2 for EV temperature work).
- `records[*].status` enum: `ok`, `stale`, `unavailable`, `not_supported`, `permission_denied`, `error`.
- `records[*].confidence` range: `0.0` to `1.0`.
- `capture_window.ended_at` must be greater than `capture_window.started_at`.
- Maximum records per batch: 5,000 (reject with `payload_too_large`).

## EV/Temperature Signals Expected
- Driving: `speed.vehicle`, `distance.odometer`, `ev.soc_pct`, `power.battery_voltage` (or derived power)
- Charging: `ev.charging_state`, `ev.soc_pct`, charge power (derived or direct)
- Temperature: `environment.ambient_temp_c` and `ev.battery_temp_c` when available

## Session Event Types
- `drive_session_start`
- `drive_session_stop`
- `charging_session_start`
- `charging_session_stop`

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

## Error Responses
- `400` invalid payload
- `401` unauthorized
- `413` payload too large
- `429` rate limited
- `500` ingest failure

## Storage Mapping
- `records` -> `vehicle_signal_observation`
- `diagnostics` -> `vehicle_diagnostic_event`
- session events -> `vehicle_session_event` (v0.2 schema)
