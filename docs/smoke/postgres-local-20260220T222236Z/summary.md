# Postgres Smoke Summary

## Run Metadata

- Executed at: `2026-02-20T22:22:39Z`
- API base: `http://127.0.0.1:18080`
- Vehicle uid: `5bf1cb17-a0c6-404b-81f6-c407b80ea3b4`
- User id: `54e9b082-226f-4b7b-b98b-703832c5dfb2`
- Runtime DB: ephemeral local Postgres (`postgres://postgres:***@127.0.0.1:55435/car_ranks_smoke`)

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

- Internal recompute job succeeded.
- Job output counts:
  - `charging_sessions_upserted=2`
  - `kpi_rows_upserted=42`
  - `ranking_rows_upserted=15`
  - `recomputed_vehicles=1`
- Public KPI and ranking reads all returned successful payloads (`200`) for the seeded vehicle.

## Captured Artifacts

- Requests:
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/telemetry-cold.json`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/telemetry-mild.json`
- Endpoint responses:
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/01-health.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/02-config-sampling.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/03-ingest-cold.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/04-ingest-mild.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/05-recompute-kpis.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/06-latest-job.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/07-kpis-me.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/08-kpis-charging.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/09-kpis-readiness.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/10-kpis-temperature-impact.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/11-rankings-range.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/12-rankings-charging.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/13-rankings-composite.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/14-rankings-temperature.txt`
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T222236Z/run-meta.txt`
