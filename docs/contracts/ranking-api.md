# `GET /v1/rankings`

Status: Draft v4 (locked KPI mapping + temperature gates)  
Date: 2026-02-20  
Purpose: Return EV ranking snapshots with cohort filters and temperature slicing.

## Query Parameters

## Required
- `ranking_type`:
  - `ev_range_efficiency`
  - `ev_charging_performance`
  - `ev_composite`
  - `ev_temperature_impact`

## Optional
- `timeframe`: `30d`, `90d`, `180d` (default `30d`)
- `temperature_bin`: `all` by default
  - for `ev_temperature_impact`, use `cold` (thin-slice data is materialized for this filter)
  - for non-temperature ranking types, `temperature_bin` must remain `all`
- `powertrain_class`: `bev`, `phev_electric_mode` (default `bev`)
- `make`
- `model`
- `trim`
- `year_band` (accepted, currently no-op in thin slice)
- `region` (accepted, currently no-op in thin slice)
- `limit` (default `25`, max `100`)
- `offset` (default `0`)

## Request Example

`GET /v1/rankings?ranking_type=ev_temperature_impact&temperature_bin=cold&timeframe=90d&limit=10`

## Response (200)

```json
{
  "generated_at": "2026-02-20T11:05:00Z",
  "ranking_type": "ev_temperature_impact",
  "timeframe": "90d",
  "temperature_bin": "cold",
  "filters": {
    "powertrain_class": "bev",
    "make": null,
    "model": null,
    "trim": null,
    "year_band": null,
    "region": null
  },
  "cohort": {
    "cohort_key": "bev|unknown|unknown|unknown|unknown",
    "cohort_size": 1,
    "sample_gate_passed": false
  },
  "rows": [
    {
      "rank": 1,
      "vehicle_uid": "uuid",
      "score": 49.1,
      "confidence_level": "medium",
      "kpis": {
        "cold_weather_range_retention": 60.0,
        "range_temperature_sensitivity_index": 12.5,
        "cold_weather_charge_speed_retention": 63.3
      }
    }
  ],
  "page": {
    "limit": 10,
    "offset": 0,
    "has_more": false
  }
}
```

## Ranking Type KPI Mapping
- `ev_range_efficiency`:
  - `ev_net_energy_efficiency`
  - `ev_estimated_practical_range`
  - `ev_urban_efficiency` (when speed signal coverage allows)
  - `ev_highway_efficiency` (when speed signal coverage allows)
  - `regeneration_recovery_ratio` (when regen/traction signal coverage allows)
  - `soc_depletion_rate_per_100km`
  - `ev_range_efficiency_score`
- `ev_charging_performance`:
  - `temp_adjusted_charge_acceptance_score`
  - `cold_weather_charge_speed_retention`
  - `charging_performance_score`
- `ev_composite`:
  - `ev_composite_base_score`
  - `ev_health_modifier_penalty`
  - `ev_composite_score`
- `ev_temperature_impact`:
  - `cold_weather_range_retention`
  - `range_temperature_sensitivity_index`
  - `cold_weather_charge_speed_retention`

## Confidence and Sample Gates
- Low sample sizes return data with `confidence_level=preview`.
- For `ev_temperature_impact`, a vehicle is ranked only when both gated retention metrics are present:
  - `cold_weather_range_retention`
  - `cold_weather_charge_speed_retention`
- If minimum cohort sample gates fail:
  - endpoint can return `sample_gate_passed=false`
  - rows may be empty
  - fallback cohort broadening is allowed (and reflected in `cohort_key`)

## Error Responses
- `400` invalid parameters
- `404` no ranking snapshot available for this filter
- `422` unsupported combination (example: `temperature_bin=cold` with `ev_range_efficiency`)
- `500` ranking query failure
