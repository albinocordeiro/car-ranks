# Postgres Smoke Summary

## Run Metadata

- Executed at: `2026-02-20T23:30:28Z`
- API base: `http://127.0.0.1:18080`
- Vehicle uid: `a94c9edd-ebb7-4392-9f27-640236b02f8d`
- User id: `6dacf43f-bce5-4022-93b2-1d6ba131b4d1`
- Runtime DB: local staging Postgres (`postgres://car_ranks_staging:***@127.0.0.1:5432/car_ranks_staging`)

## Endpoint Status

- `01-health.txt`: `200`
- `02-config-sampling.txt`: `200`
- `03-ingest-cold.txt`: `200`
- `04-ingest-mild.txt`: `200`
- `05-recompute-kpis.txt`: `200`
- `06-latest-job.txt`: `200`
- `07-kpis-me.txt`: `200`
- `08-kpis-charging.txt`: `200`
- `09-kpis-readiness.txt`: `200`
- `10-kpis-temperature-impact.txt`: `200`
- `11-rankings-range.txt`: `200`
- `12-rankings-charging.txt`: `200`
- `13-rankings-composite.txt`: `200`
- `14-rankings-temperature.txt`: `200`

## Key Outcomes

- Internal recompute job response captured in `06-latest-job.txt`.
- Job output counts:
  - `charging_sessions_upserted=6`
  - `kpi_rows_upserted=126`
  - `ranking_rows_upserted=45`
  - `recomputed_vehicles=3`
- Public KPI and ranking reads were captured for the seeded vehicle/user scope.

## Captured Artifacts

- Requests:
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/telemetry-cold.json`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/telemetry-mild.json`
- Endpoint responses:
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/01-health.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/02-config-sampling.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/03-ingest-cold.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/04-ingest-mild.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/05-recompute-kpis.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/06-latest-job.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/07-kpis-me.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/08-kpis-charging.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/09-kpis-readiness.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/10-kpis-temperature-impact.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/11-rankings-range.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/12-rankings-charging.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/13-rankings-composite.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/14-rankings-temperature.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/run-meta.txt`
