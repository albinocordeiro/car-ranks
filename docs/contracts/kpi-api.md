# KPI APIs

Status: Draft v2 (thin-slice aligned)  
Scope: Read-only EV KPI endpoints for MVP Rust backend.

## `GET /v1/kpis/me`

Returns range/efficiency KPI snapshot (`ranking_type=ev_range_efficiency`).

### Query Parameters
## Required
- `vehicle_uid`: UUID

## Optional
- `timeframe`: `30d`, `90d`, `180d` (default `30d`)
- `temperature_bin`: `all` only in thin slice (default `all`)

### Response (200)

```json
{
  "vehicle_uid": "uuid",
  "generated_at": "2026-02-17T11:00:00Z",
  "timeframe": "90d",
  "temperature_bin": "all",
  "ranking_type": "ev_range_efficiency",
  "kpis": [
    {
      "kpi_key": "ev_estimated_practical_range",
      "value": 125.0,
      "unit": "km",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 4
    },
    {
      "kpi_key": "soc_depletion_rate_per_100km",
      "value": 48.0,
      "unit": "%_per_100km",
      "direction": "lower_is_better",
      "confidence_level": "preview",
      "sample_count": 4
    },
    {
      "kpi_key": "ev_range_efficiency_score",
      "value": 42.4,
      "unit": "score",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 4
    }
  ]
}
```

## `GET /v1/kpis/charging`

Returns charging KPI snapshot (`ranking_type=ev_charging_performance`).

### Query Parameters
## Required
- `vehicle_uid`: UUID

## Optional
- `timeframe`: `30d`, `90d`, `180d` (default `30d`)
- `temperature_bin`: `all`, `cold` (default `all`)
- `charger_type`: accepted (`all`, `ac`, `dc`) but currently no-op in thin slice

### Response (200)

```json
{
  "vehicle_uid": "uuid",
  "generated_at": "2026-02-17T11:00:00Z",
  "timeframe": "90d",
  "temperature_bin": "all",
  "ranking_type": "ev_charging_performance",
  "kpis": [
    {
      "kpi_key": "temp_adjusted_charge_acceptance_score",
      "value": 81.6,
      "unit": "score",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 2
    },
    {
      "kpi_key": "cold_weather_charge_speed_retention",
      "value": 63.3,
      "unit": "%",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 1
    },
    {
      "kpi_key": "charging_performance_score",
      "value": 74.3,
      "unit": "score",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 2
    }
  ]
}
```

## `GET /v1/kpis/temperature-impact`

Returns cold-climate delta metrics (`ranking_type=ev_temperature_impact`).

### Query Parameters
## Required
- `vehicle_uid`: UUID

## Optional
- `timeframe`: `30d`, `90d`, `180d` (default `90d`)
- `baseline_temperature_bin`: default `mild`
- `compare_temperature_bin`: default `cold`

Notes:
- Thin slice currently computes against the `cold` KPI slice to avoid mixed-temperature duplicates.
- Current generated metric set includes `cold_weather_range_retention` and `cold_weather_charge_speed_retention`.

### Response (200)

```json
{
  "vehicle_uid": "uuid",
  "generated_at": "2026-02-17T11:00:00Z",
  "baseline_temperature_bin": "mild",
  "compare_temperature_bin": "cold",
  "metrics": [
    {
      "kpi_key": "cold_weather_range_retention",
      "value": 60.0,
      "unit": "%",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 2
    },
    {
      "kpi_key": "cold_weather_charge_speed_retention",
      "value": 63.3,
      "unit": "%",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 1
    }
  ],
  "cohort_benchmark": {
    "cohort_size": 1,
    "percentiles": {
      "cold_weather_range_retention": 100,
      "cold_weather_charge_speed_retention": 100
    }
  }
}
```

## Common Error Responses
- `404` no KPI snapshot available for the requested vehicle/filter
- `422` unsupported filter combination
- `500` query failure
