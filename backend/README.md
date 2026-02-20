# Car Ranks Backend (Rust MVP Thin Slice)

Current API surface:
- `GET /health`
- `GET /v1/config/sampling`
- `POST /v1/telemetry/batches`
- `GET /v1/kpis/me`
- `GET /v1/kpis/charging`
- `GET /v1/kpis/temperature-impact`
- `GET /v1/rankings`
- `POST /internal/jobs/recompute-kpis`
- `POST /internal/jobs/build-ranking-snapshots`

## Stack
- Rust
- `axum` HTTP server
- `sqlx` with SQLite

## Run

```bash
cd /Users/albinocordeiro/Code/car_ranks/backend
cargo run
```

Defaults:
- `BIND_ADDR=127.0.0.1:8080`
- `DATABASE_URL=sqlite://car_ranks.db`
- `RUST_LOG=info,sqlx=warn,tower_http=info`
- `DEFAULT_USABLE_BATTERY_KWH=75`

Optional overrides:

```bash
export BIND_ADDR=127.0.0.1:8080
export DATABASE_URL=sqlite:///tmp/car_ranks.db
export RUST_LOG=info,sqlx=warn,tower_http=info
export DEFAULT_USABLE_BATTERY_KWH=75
```

## Smoke flow

1. Ingest telemetry:

```bash
curl "http://127.0.0.1:8080/v1/config/sampling"

curl -X POST http://127.0.0.1:8080/v1/telemetry/batches \
  -H 'content-type: application/json' \
  --data @/Users/albinocordeiro/Code/car_ranks/docs/contracts/examples/telemetry-batch-request.json
```

2. Recompute KPIs/rankings:

```bash
curl -X POST http://127.0.0.1:8080/internal/jobs/recompute-kpis
```

3. Query KPI families:

```bash
curl "http://127.0.0.1:8080/v1/kpis/me?vehicle_uid=<vehicle_uuid>&timeframe=90d"
curl "http://127.0.0.1:8080/v1/kpis/charging?vehicle_uid=<vehicle_uuid>&timeframe=90d&temperature_bin=all"
curl "http://127.0.0.1:8080/v1/kpis/temperature-impact?vehicle_uid=<vehicle_uuid>&timeframe=90d&baseline_temperature_bin=mild&compare_temperature_bin=cold"
```

4. Query rankings:

```bash
curl "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_range_efficiency&timeframe=90d&limit=10"
curl "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_charging_performance&timeframe=90d&limit=10"
curl "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_composite&timeframe=90d&limit=10"
curl "http://127.0.0.1:8080/v1/rankings?ranking_type=ev_temperature_impact&timeframe=90d&temperature_bin=cold&limit=10"
```

## Thin-slice constraints
- OBD-only ingest (`source=OBD`).
- No auth/authorization yet (MVP backend slice only).
- Timeframes currently materialized by jobs: `30d`, `90d`, `180d`.
- `temperature_bin` filters on rankings are supported only for `ev_temperature_impact`.
- Temperature impact KPI reads the `cold` slice to avoid duplicate metrics across temperature variants.
- KPI persistence is restricted by a locked KPI catalog in `src/main.rs`; unknown KPI keys are rejected at write time.

## Dev checks

```bash
cargo fmt
cargo check
cargo test
```

## References
- Schema bootstrapped from `/Users/albinocordeiro/Code/car_ranks/backend/schema.sql`.
- Signal validation uses `/Users/albinocordeiro/Code/car_ranks/research/schema/signal_registry_v0_2.json`.
