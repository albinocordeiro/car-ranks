# Postgres Smoke Summary

## Run Metadata

- Executed at: `2026-02-20T23:16:05Z`
- API base: `http://127.0.0.1:18080`
- Vehicle uid: `e11889bf-504c-4238-9583-bc8840f20e19`
- User id: `06b3fff1-bfcc-4cda-840b-d512963bc239`
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
  - `charging_sessions_upserted=2`
  - `kpi_rows_upserted=42`
  - `ranking_rows_upserted=15`
  - `recomputed_vehicles=1`
- Public KPI and ranking reads were captured for the seeded vehicle/user scope.

## Captured Artifacts

- Requests:
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/telemetry-cold.json`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/telemetry-mild.json`
- Endpoint responses:
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/01-health.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/02-config-sampling.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/03-ingest-cold.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/04-ingest-mild.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/05-recompute-kpis.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/06-latest-job.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/07-kpis-me.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/08-kpis-charging.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/09-kpis-readiness.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/10-kpis-temperature-impact.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/11-rankings-range.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/12-rankings-charging.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/13-rankings-composite.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/14-rankings-temperature.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/run-meta.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/backend.log`
