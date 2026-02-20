# Internal Job APIs

Status: Draft v1  
Date: 2026-02-20  
Scope: Internal operational job execution and status endpoints.

## `POST /internal/jobs/recompute-kpis`

Triggers end-to-end internal KPI/ranking recompute for the active backend.

### Response (200)

```json
{
  "ok": true,
  "job_id": "uuid",
  "charging_sessions_upserted": 12,
  "kpi_rows_upserted": 96,
  "ranking_rows_upserted": 48,
  "recomputed_vehicles": 16
}
```

## `POST /internal/jobs/build-ranking-snapshots`

Triggers the internal ranking build job kind.

### Response (200)

Same payload shape as `POST /internal/jobs/recompute-kpis`.

## `GET /internal/jobs/latest`

Returns latest persisted status for one internal job kind.

### Query Parameters

## Optional
- `job_kind`: `recompute_kpis` or `build_rankings` (default `recompute_kpis`)

### Response (200)

```json
{
  "job_run_id": "uuid",
  "job_kind": "recompute_kpis",
  "backend": "postgres",
  "status": "succeeded",
  "started_at": "2026-02-20T11:05:00Z",
  "finished_at": "2026-02-20T11:05:01Z",
  "error_message": null,
  "response_job_id": "uuid",
  "charging_sessions_upserted": 12,
  "kpi_rows_upserted": 96,
  "ranking_rows_upserted": 48,
  "recomputed_vehicles": 16,
  "active_lock_owner_token": null,
  "active_lock_expires_at": null
}
```

Notes:
- `status` enum: `running`, `succeeded`, `failed`.
- `active_lock_owner_token` and `active_lock_expires_at` are populated only while a valid lock lease exists for the requested `job_kind`.
- When a previous run is stuck in `running` and its lease window is stale, a new trigger auto-marks that stale row as `failed` before writing the next `running` row.

## Error Responses

- `404` no internal job run exists for requested `job_kind`.
- `409` active lease lock already exists for requested `job_kind`.
- `422` unsupported `job_kind`.
- `500` job execution or status lookup failed.
