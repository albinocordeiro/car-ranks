# API Contracts (MVP)

Status: Draft v3  
Date: 2026-02-20

This folder contains executable contract drafts for the EV-first MVP APIs.

## Files
- `telemetry-batch-api.md`: `POST /v1/telemetry/batches` request/response contract.
- `config-sampling-api.md`: `GET /v1/config/sampling` response contract for cadence/profile config.
- `kpi-api.md`: `GET /v1/kpis/me`, `GET /v1/kpis/charging`, and `GET /v1/kpis/temperature-impact`.
- `ranking-api.md`: `GET /v1/rankings` query and response contract.
- `examples/*.json`: copy-ready payload examples.
  - `config-sampling-response.json`
  - `kpis-me-response.json`
  - `kpis-charging-response.json`
  - `kpis-temperature-impact-response.json`
  - `rankings-response.json`
  - `telemetry-batch-request.json`
  - `telemetry-batch-response.json`

## Contract Conventions
- All timestamps are RFC3339 UTC strings.
- IDs are UUIDs unless noted.
- Numeric KPI values are stored in canonical units from the signal registry.
- Temperature-related keys align with signal registry v0.2.
- `confidence_level` enum: `preview`, `medium`, `stable`.
- `temperature_bin` enum: `all`, `very_cold`, `cold`, `cool`, `mild`, `hot`.
- This folder documents the current thin-slice backend behavior with locked KPI formulas and signal mappings.
