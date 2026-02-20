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

## Capture A New Baseline

Run the one-command smoke runner:

```bash
/Users/albinocordeiro/Code/car_ranks/docs/smoke/scripts/run_local_postgres_smoke.sh
```

Optional flags:
- `--env-file /Users/albinocordeiro/Code/car_ranks/backend/.env.staging`
- `--api-base http://127.0.0.1:18080`
- `--output-root /Users/albinocordeiro/Code/car_ranks/docs/smoke`
- `--keep-backend-log`

The script writes a new `postgres-local-<timestamp>` folder containing:
- endpoint captures (`01-*.txt` through `14-*.txt`)
- request payloads (`telemetry-cold.json`, `telemetry-mild.json`)
- `run-meta.txt` and `summary.md`
