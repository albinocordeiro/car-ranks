# Smoke Baselines

This folder stores captured API smoke baselines for release verification.

## Latest Baseline

- Run id: `postgres-local-20260220T231603Z`
- Environment: local backend + local staging Postgres
- Snapshot root: `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z`
- Summary: `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T231603Z/summary.md`

## Notes

- Each endpoint capture file ends with `__STATUS__:<http_code>`.
- Request payloads are included as `telemetry-cold.json` and `telemetry-mild.json`.
- `run-meta.txt` records the user id, vehicle id, and execution metadata for reproducibility.
