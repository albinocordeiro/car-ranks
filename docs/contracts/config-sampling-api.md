# `GET /v1/config/sampling`

Status: Draft v1  
Purpose: Return app polling and upload cadence configuration for iOS OBD clients.

## Request

No parameters.

## Response (200)

```json
{
  "generated_at": "2026-02-17T11:00:00Z",
  "platform": "ios",
  "source": "obd",
  "read_only": true,
  "batch_upload": {
    "default_interval_seconds": 60,
    "min_interval_seconds": 60,
    "max_interval_seconds": 86400,
    "next_upload_after_seconds": 60
  },
  "sampling_profiles": [
    {
      "mode": "driving",
      "sample_interval_seconds": 5
    },
    {
      "mode": "charging",
      "sample_interval_seconds": 10
    },
    {
      "mode": "parked",
      "sample_interval_seconds": 60
    }
  ],
  "kpi_refresh": {
    "active_vehicle_interval_seconds": 300,
    "daily_rebuild_interval_seconds": 86400
  },
  "feature_flags": {
    "smartcar_enabled": false,
    "remote_commands_enabled": false
  }
}
```

## Behavioral Notes
- Upload cadence is constrained to the MVP bounds:
  - minimum interval: `60` seconds (1 minute)
  - maximum interval: `86400` seconds (daily)
- `default_interval_seconds` is clamped to the min/max bounds.
- `next_upload_after_seconds` is currently equal to `default_interval_seconds`.
- Endpoint is unauthenticated in the current thin slice.

## Environment Overrides (thin slice)
- `CAR_RANKS_UPLOAD_INTERVAL_SECONDS` (default `60`)
- `CAR_RANKS_UPLOAD_INTERVAL_MIN_SECONDS` (default `60`)
- `CAR_RANKS_UPLOAD_INTERVAL_MAX_SECONDS` (default `86400`)
- `CAR_RANKS_DRIVING_SAMPLE_INTERVAL_SECONDS` (default `5`)
- `CAR_RANKS_CHARGING_SAMPLE_INTERVAL_SECONDS` (default `10`)
- `CAR_RANKS_PARKED_SAMPLE_INTERVAL_SECONDS` (default `60`)
- `CAR_RANKS_ACTIVE_KPI_REFRESH_SECONDS` (default `300`)
- `CAR_RANKS_DAILY_REBUILD_SECONDS` (default `86400`)

All overrides must parse as positive integers; invalid values fall back to defaults.

## Error Responses
- `500` internal failure
