# Canonical Signal Schema v0.1

Date: 2026-02-17  
Goal: normalize Smartcar and OBD-II BLE data into one source-agnostic model for ranking and analytics.

## 1) Design principles
- Use one canonical signal namespace, independent of provider.
- Store source metadata for every value (`source`, `source_field`, freshness, confidence).
- Keep capability gaps explicit (`not_supported`, `permission_denied`, `unavailable`).
- Separate identity, telemetry, diagnostics, and command capability.

## 2) Canonical entities

### `vehicle`
- `vehicle_uid` (internal UUID)
- `source_account_id` (Smartcar account id or OBD user/device id)
- `vin` (nullable)
- `make` (nullable)
- `model` (nullable)
- `model_year` (nullable)
- `powertrain` (`ICE|HEV|PHEV|BEV|UNKNOWN`)
- `created_at`, `updated_at`

### `vehicle_signal_observation`
- `observation_id` (UUID)
- `vehicle_uid`
- `signal_key` (from catalog below)
- `value_number` (nullable)
- `value_string` (nullable)
- `value_bool` (nullable)
- `unit` (nullable)
- `observed_at` (source timestamp when available)
- `ingested_at` (server timestamp)
- `source` (`SMARTCAR|OBD`)
- `source_signal` (endpoint/mode-pid identifier)
- `status` (`ok|stale|unavailable|not_supported|permission_denied|error`)
- `confidence` (`0.0-1.0`)
- `freshness_ttl_seconds`
- `raw_payload_ref` (pointer to raw blob)

### `vehicle_diagnostic_event`
- `event_id` (UUID)
- `vehicle_uid`
- `event_type` (`DTC_ACTIVE|DTC_CLEARED|MIL_ON|MIL_OFF|READINESS_CHANGED|SOURCE_ERROR`)
- `code` (e.g., DTC)
- `severity` (nullable)
- `description` (nullable)
- `observed_at`, `ingested_at`
- `source`, `source_event`
- `resolution_hint` (nullable)

## 3) Canonical signal catalog (v0.1)

| signal_key | Type | Unit | Smartcar mapping | OBD mapping | Priority | Notes |
|---|---|---|---|---|---|---|
| `vehicle.vin` | string | - | `GET /vehicles/{id}/vin` + `read_vin` | Mode 09 vehicle info (when supported) | Smartcar > OBD | VIN may be unavailable by source/vehicle. |
| `vehicle.make` | string | - | `GET /vehicles/{id}` + `read_vehicle_info` | Mode 09 vehicle info (when supported) | Smartcar > OBD | |
| `vehicle.model` | string | - | `GET /vehicles/{id}` + `read_vehicle_info` | Mode 09 vehicle info (when supported) | Smartcar > OBD | |
| `vehicle.model_year` | int | year | `GET /vehicles/{id}` + `read_vehicle_info` | Mode 09 vehicle info (when supported) | Smartcar > OBD | |
| `distance.odometer` | number | km (canonical) | `GET /vehicles/{id}/odometer` + `read_odometer` | Adapter/app-specific OBD support | Smartcar > OBD | Convert miles->km in normalization. |
| `position.latitude` | number | deg | `GET /vehicles/{id}/location` + `read_location` | Not standard OBD-II emission PID | Smartcar only | |
| `position.longitude` | number | deg | `GET /vehicles/{id}/location` + `read_location` | Not standard OBD-II emission PID | Smartcar only | |
| `fuel.level_pct` | number | % | `GET /vehicles/{id}/fuel` + `read_fuel` | Mode 01 current data (vehicle dependent) | Smartcar > OBD | |
| `ev.soc_pct` | number | % | `GET /vehicles/{id}/battery` + `read_battery` | EV OBD support varies | Smartcar > OBD | |
| `ev.charging_state` | enum | - | `GET /vehicles/{id}/charge` + `read_charge` | EV OBD support varies | Smartcar > OBD | |
| `speed.vehicle` | number | km/h | `read_speedometer` (if available) | Mode 01 current data | OBD > Smartcar | Use highest-frequency source for live KPIs. |
| `engine.rpm` | number | rpm | (brand/signal dependent) | Mode 01 current data | OBD | |
| `power.battery_voltage` | number | V | (brand/signal dependent) | Mode 01 current data / adapter-derived | OBD | |
| `diag.mil_on` | bool | - | `read_diagnostics` / webhook errors/events when available | Mode 01 PID 01 MIL bit | OBD > Smartcar | Regulatory MIL logic grounded in SAE J1979/CFR flow. |
| `diag.readiness_summary` | json | - | limited | Mode 01 PID 01 readiness | OBD | Track supported vs complete monitors. |
| `diag.dtcs_active` | array<string> | - | `GET /vehicles/{id}/diagnostics/dtcs` + `read_diagnostics` | Mode 03 DTCs | Smartcar + OBD merge | Preserve source-specific code metadata. |
| `diag.system_status` | json | - | `GET /vehicles/{id}/diagnostics/system_status` + `read_diagnostics` | adapter/manufacturer dependent | Smartcar > OBD | |
| `security.locked` | bool | - | `GET /vehicles/{id}/security` + `read_security` | Not standard OBD-II | Smartcar | |
| `tires.pressure` | json | kPa (canonical) | `GET /vehicles/{id}/tires/pressure` + `read_tires` | manufacturer/adapter dependent | Smartcar > OBD | Normalize PSI->kPa if needed. |

## 4) Capability schema

### `vehicle_capability`
- `vehicle_uid`
- `capability_key` (e.g., `read_location`, `control_security`, `obd_mode03`)
- `source` (`SMARTCAR|OBD`)
- `status` (`supported|not_supported|unknown|permission_denied`)
- `last_checked_at`
- `check_method` (`compatibility_api|connect_grant|runtime_probe|manual`)

Rules:
- Smartcar capabilities are derived from granted permissions and runtime responses.
- OBD capabilities are derived from adapter handshake + successful mode requests.

## 5) Source precedence and merge rules
- Identity (`vin/make/model/year`): prefer Smartcar; fallback OBD mode 09.
- Diagnostics (`mil_on/readiness/dtcs`):
  - use OBD as primary for direct in-vehicle inspection semantics;
  - merge Smartcar diagnostics as supplemental stream.
- Real-time driving signals (`speed`, short-interval telemetry): prefer OBD.
- Cloud-state signals (`location`, remote lock/charge, service history): Smartcar primary.

Conflict resolution:
1. Choose source by signal-level priority.
2. If same priority, choose newest `observed_at`.
3. If timestamps missing, choose highest confidence.
4. Keep losing observation (not discarded) for audit/reconciliation.

## 6) Freshness policy (initial)
- `position.*`: TTL 10 min (Smartcar webhook-driven preferred).
- `distance.odometer`: TTL 24h (unless fresh webhook/event).
- `speed.vehicle`, `engine.rpm`: TTL 5s (OBD live stream).
- `diag.*`: TTL 24h, but event updates processed immediately.

## 7) Normalization conventions
- Distance canonical unit: `km`.
- Speed canonical unit: `km/h`.
- Pressure canonical unit: `kPa`.
- Temperature canonical unit: `C`.
- Percent values normalized to `0..100`.
- Missing/unsupported values must be represented with `status != ok` rather than null-only semantics.

## 8) Minimal ingestion contract per source

### Smartcar adapter emits
- permissions snapshot (`read_*`, `control_*`)
- vehicle identity
- telemetry/dx observations from REST + webhooks
- source errors mapped to normalized `status` and `resolution_hint`

### OBD adapter emits
- adapter identity + protocol negotiation result
- mode 01 current data samples
- mode 01 PID 01 readiness + MIL
- mode 03 DTC payload
- explicit action event if mode 04 (clear) is sent

## 9) Implementation checklist
1. Create DB tables for `vehicle`, `vehicle_signal_observation`, `vehicle_diagnostic_event`, `vehicle_capability`.
2. Implement canonical signal registry (hardcoded YAML/JSON in code for v0.1).
3. Implement two mappers (`smartcar_to_canonical`, `obd_to_canonical`).
4. Add deterministic merge service using precedence rules above.
5. Add KPI reader API that only reads canonical signals.

## 10) Source references used for this schema
- Smartcar Compatibility API overview and scope: `research/sources/smartcar/llms-full.txt:670`, `research/sources/smartcar/llms-full.txt:713`, `research/sources/smartcar/llms-full.txt:905`
- Smartcar Vehicles API behavior/versioning: `research/sources/smartcar/llms-full.txt:8712`, `research/sources/smartcar/llms-full.txt:8716`, `research/sources/smartcar/llms-full.txt:8746`
- Smartcar webhook constraints: `research/sources/smartcar/llms-full.txt:8874`, `research/sources/smartcar/llms-full.txt:8894`, `research/sources/smartcar/llms-full.txt:8901`, `research/sources/smartcar/llms-full.txt:8904`
- Smartcar permissions taxonomy: `research/sources/smartcar/llms-full.txt:3638`, `research/sources/smartcar/llms-full.txt:3667`
- Smartcar endpoint references in docs dump (selected): `research/sources/smartcar/llms-full.txt:1142`, `research/sources/smartcar/llms-full.txt:1242`, `research/sources/smartcar/llms-full.txt:1990`, `research/sources/smartcar/llms-full.txt:2062`, `research/sources/smartcar/llms-full.txt:2106`, `research/sources/smartcar/llms-full.txt:2424`, `research/sources/smartcar/llms-full.txt:2770`, `research/sources/smartcar/llms-full.txt:2824`
- OBD/ELM327 protocol and command model: `research/sources/obd/elm327-datasheet.txt:10`, `research/sources/obd/elm327-datasheet.txt:14`, `research/sources/obd/elm327-datasheet.txt:522`, `research/sources/obd/elm327-datasheet.txt:528`, `research/sources/obd/elm327-datasheet.txt:2540`
- OBD readiness/MIL/DTC inspection flow grounding: `research/sources/obd/cfr-40-sec85-2222.txt:136`, `research/sources/obd/cfr-40-sec85-2222.txt:137`, `research/sources/obd/cfr-40-sec85-2222.txt:208`
