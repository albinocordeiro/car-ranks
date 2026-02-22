# Car Ranks Backend (Rust MVP Thin Slice)

Current API surface:
- `GET /health`
- `GET /v1/config/sampling`
- `POST /v1/telemetry/batches`
- `GET /v1/telemetry/raw`
- `GET /v1/kpis/me`
- `GET /v1/kpis/charging`
- `GET /v1/kpis/readiness`
- `GET /v1/kpis/temperature-impact`
- `GET /v1/rankings`
- `POST /internal/jobs/recompute-kpis`
- `POST /internal/jobs/build-ranking-snapshots`
- `GET /internal/jobs/latest`

Auth scope (MVP):
- Public vehicle-bound APIs require `x-user-id: <uuid>` request header.
- Vehicle data is user-scoped via `user_vehicle_access`.

## Stack
- Rust
- `axum` HTTP server
- `sqlx` with Postgres

## Run

```bash
cd /Users/albinocordeiro/Code/car_ranks/backend
cargo run
```

Defaults:
- `BIND_ADDR=127.0.0.1:8080`
- `DATABASE_URL` must be set to a Postgres URL
- `RUST_LOG=info,sqlx=warn,tower_http=info`
- `DEFAULT_USABLE_BATTERY_KWH=75`
- `CAR_RANKS_TEMP_GATE_MIN_COLD_DISTANCE_KM=20`
- `CAR_RANKS_TEMP_GATE_MIN_MILD_DISTANCE_KM=20`
- `CAR_RANKS_TEMP_GATE_MIN_COLD_CHARGE_SESSIONS=1`
- `CAR_RANKS_TEMP_GATE_MIN_MILD_CHARGE_SESSIONS=1`
- `CAR_RANKS_TEMP_GATE_MIN_SENSITIVITY_POINTS=6`

Optional overrides:

```bash
export BIND_ADDR=127.0.0.1:8080
export DATABASE_URL=postgres://<user>:<pass>@<host>:5432/<db>
export RUST_LOG=info,sqlx=warn,tower_http=info
export DEFAULT_USABLE_BATTERY_KWH=75
export CAR_RANKS_TEMP_GATE_MIN_COLD_DISTANCE_KM=20
export CAR_RANKS_TEMP_GATE_MIN_MILD_DISTANCE_KM=20
export CAR_RANKS_TEMP_GATE_MIN_COLD_CHARGE_SESSIONS=1
export CAR_RANKS_TEMP_GATE_MIN_MILD_CHARGE_SESSIONS=1
export CAR_RANKS_TEMP_GATE_MIN_SENSITIVITY_POINTS=6
```

## Smoke flow

1. Ingest telemetry:

```bash
curl "http://127.0.0.1:8080/v1/config/sampling"

curl -X POST http://127.0.0.1:8080/v1/telemetry/batches \
  -H 'x-user-id: <user_uuid>' \
  -H 'content-type: application/json' \
  --data @/Users/albinocordeiro/Code/car_ranks/docs/contracts/examples/telemetry-batch-request.json

curl -H 'x-user-id: <user_uuid>' \
  "http://127.0.0.1:8080/v1/telemetry/raw?vehicle_uid=<vehicle_uuid>&limit=50"

curl -H 'x-user-id: <user_uuid>' \
  "http://127.0.0.1:8080/v1/telemetry/raw?vehicle_uid=<vehicle_uuid>&session_id=<session_uuid>&include_session_events=true&limit=200"

curl -H 'x-user-id: <user_uuid>' \
  "http://127.0.0.1:8080/v1/telemetry/raw?vehicle_uid=<vehicle_uuid>&limit=200&cursor_observed_at=<rfc3339>&cursor_observation_id=<observation_uuid>"
```

2. Recompute KPIs/rankings:

```bash
curl -X POST http://127.0.0.1:8080/internal/jobs/recompute-kpis
curl "http://127.0.0.1:8080/internal/jobs/latest?job_kind=recompute_kpis"
```

3. Query KPI families:

```bash
curl "http://127.0.0.1:8080/v1/kpis/me?vehicle_uid=<vehicle_uuid>&timeframe=90d"
curl "http://127.0.0.1:8080/v1/kpis/charging?vehicle_uid=<vehicle_uuid>&timeframe=90d&temperature_bin=all"
curl "http://127.0.0.1:8080/v1/kpis/readiness?vehicle_uid=<vehicle_uuid>&timeframe=90d"
curl "http://127.0.0.1:8080/v1/kpis/temperature-impact?vehicle_uid=<vehicle_uuid>&timeframe=90d&baseline_temperature_bin=mild&compare_temperature_bin=cold"
```

With auth header:

```bash
curl -H 'x-user-id: <user_uuid>' "http://127.0.0.1:8080/v1/kpis/me?vehicle_uid=<vehicle_uuid>&timeframe=90d"
curl -H 'x-user-id: <user_uuid>' "http://127.0.0.1:8080/v1/kpis/charging?vehicle_uid=<vehicle_uuid>&timeframe=90d&temperature_bin=all"
curl -H 'x-user-id: <user_uuid>' "http://127.0.0.1:8080/v1/kpis/readiness?vehicle_uid=<vehicle_uuid>&timeframe=90d"
curl -H 'x-user-id: <user_uuid>' "http://127.0.0.1:8080/v1/kpis/temperature-impact?vehicle_uid=<vehicle_uuid>&timeframe=90d&baseline_temperature_bin=mild&compare_temperature_bin=cold"
```

4. Query rankings:

```bash
curl "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_range_efficiency&timeframe=90d&limit=10"
curl "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_charging_performance&timeframe=90d&limit=10"
curl "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_composite&timeframe=90d&limit=10"
curl "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_temperature_impact&timeframe=90d&temperature_bin=cold&limit=10"
```

With auth header:

```bash
curl -H 'x-user-id: <user_uuid>' "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_range_efficiency&timeframe=90d&limit=10"
curl -H 'x-user-id: <user_uuid>' "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_charging_performance&timeframe=90d&limit=10"
curl -H 'x-user-id: <user_uuid>' "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_composite&timeframe=90d&limit=10"
curl -H 'x-user-id: <user_uuid>' "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_temperature_impact&timeframe=90d&temperature_bin=cold&limit=10"
```

## Thin-slice constraints
- OBD-only ingest (`source=OBD`).
- Ingest payload schema is locked to `schema_version=0.2`.
- Duplicate `batch_id` replays are accepted only when payload envelope matches; mismatched envelopes return `409 conflict`.
- Public vehicle-bound APIs require `x-user-id` and enforce user-to-vehicle ownership scope.
- Timeframes currently materialized by jobs: `30d`, `90d`, `180d`.
- `temperature_bin` filters on rankings are supported only for `ev_temperature_impact`.
- Temperature impact KPI reads the `cold` slice to avoid duplicate metrics across temperature variants.
- Temperature KPI gates are enforced before persistence:
  - range/sensitivity require cold + mild distance coverage (defaults: `20 km` each)
  - cold charging retention requires cold + mild charging sessions (defaults: `1` each)
- Temperature-impact rankings include only vehicles that pass both gated retention metrics (`cold_weather_range_retention` and `cold_weather_charge_speed_retention`).
- KPI persistence is restricted by a locked KPI catalog in `src/main.rs`; unknown KPI keys are rejected at write time.
- Internal job endpoints enforce a per-`job_kind` lease lock (~10 minutes) and reject overlapping triggers with `409 conflict` including lock owner + expiry diagnostics.
- `GET /internal/jobs/latest` includes active lock owner token + lease expiry when a matching job lock is currently held.
- Triggering a new internal job automatically marks stale prior `running` rows as `failed` when their lease window has elapsed.
- Internal recompute runs fully natively in Postgres (charging sessions, KPI families, and ranking families).

## Migrations
- Startup requires `DATABASE_URL` with a Postgres scheme (`postgres://` or `postgresql://`).
- Startup applies Postgres migrations from `/Users/albinocordeiro/Code/car_ranks/backend/migrations/postgres/`.
- Applied migration ids are tracked in `schema_migration` to prevent duplicate execution.
- Postgres-ready bootstrap schema lives in `/Users/albinocordeiro/Code/car_ranks/backend/migrations/postgres/0001_init.sql`.
- Ownership/auth additive migrations live in `/Users/albinocordeiro/Code/car_ranks/backend/migrations/postgres/0002_auth_ownership.sql`.
- Internal job-run metadata migrations live in `/Users/albinocordeiro/Code/car_ranks/backend/migrations/postgres/0003_internal_job_runs.sql`.
- Internal job lock/lease migrations live in `/Users/albinocordeiro/Code/car_ranks/backend/migrations/postgres/0004_internal_job_locks.sql`.
- Current Postgres runtime endpoints:
  - `/health`
  - `/v1/config/sampling`
  - `/v1/telemetry/batches`
  - `/v1/telemetry/raw`
  - `/v1/kpis/me`
  - `/v1/kpis/charging`
  - `/v1/kpis/readiness`
  - `/v1/kpis/temperature-impact`
  - `/v1/rankings`
  - `/internal/jobs/recompute-kpis`
  - `/internal/jobs/build-ranking-snapshots`

## Dev checks

```bash
cargo fmt
cargo check
cargo test
```

Postgres integration checks (optional):

```bash
export POSTGRES_TEST_DATABASE_URL=postgres://<user>:<pass>@<host>:5432/<db>
cargo test postgres_bootstrap_migration_applies_when_env_set
cargo test postgres_kpi_fetch_and_charging_handler_work_when_env_set
cargo test postgres_ingest_enforces_idempotency_and_vehicle_ownership_when_env_set
cargo test postgres_rankings_and_temperature_impact_handlers_work_when_env_set
cargo test postgres_internal_job_handler_runs_native_pipeline_when_env_set
cargo test postgres_readiness_handler_returns_family_statuses_when_env_set
```

## References
- Postgres bootstrap schema: `/Users/albinocordeiro/Code/car_ranks/backend/migrations/postgres/0001_init.sql`.
- Signal validation uses `/Users/albinocordeiro/Code/car_ranks/research/schema/signal_registry_v0_2.json`.
