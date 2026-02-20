# KPI APIs

Status: Draft v3 (locked EV-first formulas)  
Date: 2026-02-20  
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
  "generated_at": "2026-02-20T11:00:00Z",
  "timeframe": "90d",
  "temperature_bin": "all",
  "ranking_type": "ev_range_efficiency",
  "kpis": [
    {
      "kpi_key": "ev_net_energy_efficiency",
      "value": 157.8,
      "unit": "Wh_per_km",
      "direction": "lower_is_better",
      "confidence_level": "preview",
      "sample_count": 8
    },
    {
      "kpi_key": "ev_estimated_practical_range",
      "value": 289.2,
      "unit": "km",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 8
    },
    {
      "kpi_key": "ev_urban_efficiency",
      "value": 151.3,
      "unit": "Wh_per_km",
      "direction": "lower_is_better",
      "confidence_level": "preview",
      "sample_count": 4
    },
    {
      "kpi_key": "ev_highway_efficiency",
      "value": 173.4,
      "unit": "Wh_per_km",
      "direction": "lower_is_better",
      "confidence_level": "preview",
      "sample_count": 3
    },
    {
      "kpi_key": "regeneration_recovery_ratio",
      "value": 14.9,
      "unit": "%",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 6
    },
    {
      "kpi_key": "soc_depletion_rate_per_100km",
      "value": 21.0,
      "unit": "%_per_100km",
      "direction": "lower_is_better",
      "confidence_level": "preview",
      "sample_count": 8
    },
    {
      "kpi_key": "ev_range_efficiency_score",
      "value": 58.2,
      "unit": "score",
      "direction": "higher_is_better",
      "confidence_level": "preview",
      "sample_count": 8
    }
  ]
}
```

Notes:
- `ev_urban_efficiency` is emitted only when `speed.vehicle` is available for at least one valid segment below 45 km/h.
- `ev_highway_efficiency` is emitted only when `speed.vehicle` is available for at least one valid segment at or above 80 km/h.
- `regeneration_recovery_ratio` is emitted only when both `ev.regen_power_kw` and `ev.traction_power_kw` exist with valid windows.

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
  "generated_at": "2026-02-20T11:00:00Z",
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

### Response (200)

```json
{
  "vehicle_uid": "uuid",
  "generated_at": "2026-02-20T11:00:00Z",
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
      "kpi_key": "range_temperature_sensitivity_index",
      "value": 12.5,
      "unit": "%_loss_per_10C_drop",
      "direction": "lower_is_better",
      "confidence_level": "preview",
      "sample_count": 8
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
      "range_temperature_sensitivity_index": 100,
      "cold_weather_charge_speed_retention": 100
    }
  }
}
```

## Locked KPI Formula and Signal Contract

Backend persists only KPI keys listed in its locked catalog. Snapshot writes fail for non-catalog keys.

| Ranking Type | KPI Key | Formula (locked) | Required Signals |
|---|---|---|---|
| `ev_range_efficiency` | `ev_net_energy_efficiency` | `median(((delta_soc_pct/100) * DEFAULT_USABLE_BATTERY_KWH * 1000) / delta_km)` | `distance.odometer`, `ev.soc_pct` |
| `ev_range_efficiency` | `ev_estimated_practical_range` | `latest_soc_pct * median(delta_km / delta_soc_pct)` | `distance.odometer`, `ev.soc_pct` |
| `ev_range_efficiency` | `ev_urban_efficiency` | median net efficiency where speed `< 45 km/h` | `distance.odometer`, `ev.soc_pct`, `speed.vehicle` |
| `ev_range_efficiency` | `ev_highway_efficiency` | median net efficiency where speed `>= 80 km/h` | `distance.odometer`, `ev.soc_pct`, `speed.vehicle` |
| `ev_range_efficiency` | `regeneration_recovery_ratio` | `100 * regen_wh / (regen_wh + traction_wh)` | `ev.regen_power_kw`, `ev.traction_power_kw` |
| `ev_range_efficiency` | `ev_range_efficiency_score` | blend of normalized efficiency and estimated range | `distance.odometer`, `ev.soc_pct` |
| `ev_charging_performance` | `temp_adjusted_charge_acceptance_score` | `clamp(100 * median(all_charge_kw)/median(mild_charge_kw), 0, 120)` | `ev.charging_state`, `ev.charge_power_kw`, `ev.soc_pct` |
| `ev_charging_performance` | `cold_weather_charge_speed_retention` | `100 * median(cold_charge_kw)/median(mild_charge_kw)` | `ev.charging_state`, `ev.charge_power_kw`, `ev.soc_pct` |
| `ev_charging_performance` | `charging_performance_score` | `0.6 * acceptance + 0.4 * cold_retention` | `ev.charging_state`, `ev.charge_power_kw`, `ev.soc_pct` |
| `ev_temperature_impact` | `cold_weather_range_retention` | `100 * median(cold_km_per_soc)/median(mild_km_per_soc)` | `distance.odometer`, `ev.soc_pct`, `environment.ambient_temp_c` |
| `ev_temperature_impact` | `range_temperature_sensitivity_index` | normalized `% loss per 10C drop` from slope | `distance.odometer`, `ev.soc_pct`, `environment.ambient_temp_c` |
| `ev_temperature_impact` | `cold_weather_charge_speed_retention` | `100 * median(cold_charge_kw)/median(mild_charge_kw)` | `ev.charging_state`, `ev.charge_power_kw`, `ev.soc_pct` |
| `ev_composite` | `ev_composite_base_score` | `0.6 * ev_range_efficiency_score + 0.4 * charging_performance_score` | derived from KPI snapshots |
| `ev_composite` | `ev_health_modifier_penalty` | `min(10, (MIL_ON ? 6 : 0) + min(4, 0.5 * distinct_active_dtc_count))` | `diag.mil_on`, `diag.dtcs_active` |
| `ev_composite` | `ev_composite_score` | `clamp(base_score - health_penalty, 0, 100)` | derived from KPI snapshots + diagnostics |

## Common Error Responses
- `404` no KPI snapshot available for the requested vehicle/filter
- `422` unsupported filter combination
- `500` query failure
