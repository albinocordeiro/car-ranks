# Car Ranks MVP Spec (v1)

Status: Draft v4 (EV + Temperature Focus)  
Date: 2026-02-17  
Scope: Product + technical specification for OBD-only iOS MVP.

## 1. Goals
- Deliver meaningful EV rankings quickly after onboarding.
- Focus MVP KPI set on EV range, EV efficiency, and charging behavior.
- Make temperature impact explicit in KPIs, with strong cold-climate insight.
- Prioritize BEV support, with optional PHEV electric-mode support when signals are sufficient.
- Build an EV-centric telemetry database in S3 for future KPI redesign and additional EV products.
- Keep parked diagnostics as immediate context and a small score modifier.

## 2. Target Users
- EV owners.
- EV buyers evaluating real-world model behavior.
- EV sellers/dealers who need cohort-backed EV performance context.

## 3. MVP Boundaries

## In scope
- iOS app + BLE OBD data capture.
- Read-only KPIs/rankings.
- Batch ingest (default 1 minute; configurable up to daily).
- Cohort ranking by make/model/trim/year where available.
- EV driving telemetry KPIs (primary track).
- EV charging telemetry KPIs (secondary but mandatory for EV value).
- Temperature-binned model comparisons (especially cold-weather behavior).
- Parked-car diagnostics available immediately and used as a health modifier.
- Optional PHEV electric-mode inclusion when EV signal completeness is adequate.

## Out of scope
- ICE-only and HEV-only ranking tracks for MVP.
- Remote vehicle commands.
- Android client.
- Smartcar integration (future phase).
- Full standalone reliability ranking track (deferred; health shown as modifier/card in MVP).

## 4. User Value Timing (Fast Time-to-Insight)
- Immediate value (no driving required): parked snapshot card (SoC where available + diagnostic health card).
- Early driving value: EV efficiency preview after short driving window.
- Charging value: charging performance preview after first qualifying charge session.
- Cold-climate value: model-vs-model temperature impact view after enough cold samples.
- Stable value: confidence-weighted rankings after sufficient trip and charging volume.

## Suggested thresholds
- `parked health card`: available immediately after first successful diagnostic read.
- `EV efficiency preview`: after at least 8 minutes driving or 5 km.
- `charging performance preview`: after at least one charging session with at least 10 minutes duration and at least 2 kWh delivered (or equivalent SoC delta threshold where kWh is unavailable).
- `cold-climate preview`: after at least 20 km in `cold` or `very_cold` bins and at least one qualifying charging session in those bins.
- `stable profile`: after at least 3 trips or 50 km and at least 3 qualifying charge sessions.

## 5. Ranking Scope and Cohorts

## Primary cohort key
- `powertrain_class + make + model + trim + year_band`

`powertrain_class` for MVP:
- `bev`
- `phev_electric_mode` (optional, gated by signal completeness)

## Fallback hierarchy (for sparse cohorts)
1. `powertrain_class + make + model + trim + year_band`
2. `powertrain_class + make + model + year_band`
3. `powertrain_class + make + model`
4. `powertrain_class + vehicle_segment`

Use the most specific cohort with enough population to avoid noisy percentiles.

## Temperature comparison lens (cross-model)
- Every EV ranking can be sliced by `temperature_bin`.
- Default bins for MVP:
  - `very_cold`: <= -5C
  - `cold`: > -5C and <= 5C
  - `cool`: > 5C and <= 15C
  - `mild`: > 15C and <= 25C
  - `hot`: > 25C
- Cold-climate analysis primarily uses `very_cold` + `cold` bins against `mild` baseline.

## Initial cohort minimums
- Preferred cohort minimum: 30 vehicles.
- Absolute minimum for display: 10 vehicles (show low-confidence badge).

## 6. Top 10 KPI Set (EV + Temperature Focus)

Note: KPIs 1-9 are EV telemetry metrics. KPI 10 is a mixed EV score with a parked-health modifier.

| # | KPI | Category | Unit | Direction | Applicability | Why it matters |
|---|---|---|---|---|---|---|
| 1 | EV Net Energy Efficiency | EV Efficiency | Wh/km | lower is better | BEV, PHEV electric mode | Core EV efficiency metric. |
| 2 | EV Estimated Practical Range | EV Range | km | higher is better | BEV, PHEV electric mode | User-understandable real-world range outcome. |
| 3 | Cold-Weather Range Retention | Temperature vs Range | % | higher is better | BEV, PHEV electric mode | Directly answers range loss in cold climates. |
| 4 | Range Temperature Sensitivity Index | Temperature vs Range | % loss per 10C drop | lower is better | BEV, PHEV electric mode | Quantifies how sharply range degrades as temperature drops. |
| 5 | Regeneration Recovery Ratio | EV Efficiency | % | higher is better | BEV, PHEV electric mode (if supported) | Captures recaptured energy effectiveness. |
| 6 | EV Urban Efficiency | EV Efficiency | Wh/km | lower is better | BEV, PHEV electric mode | City-driving efficiency profile. |
| 7 | EV Highway Efficiency | EV Efficiency | Wh/km | lower is better | BEV, PHEV electric mode | Highway-driving efficiency profile. |
| 8 | Temp-Adjusted Charge Acceptance Score | Charging | 0-100 | higher is better | BEV, PHEV electric mode (if charging telemetry supported) | Measures observed charge power vs expected for same SoC and temperature conditions. |
| 9 | Cold-Weather Charge Speed Retention | Temperature vs Charging | % | higher is better | BEV, PHEV electric mode (if charging telemetry supported) | Directly answers charging speed loss in cold climates. |
| 10 | EV Composite Score (with Health Modifier) | Mixed EV | 0-100 | higher is better | BEV, PHEV electric mode | Unified EV score combining range, efficiency, temperature, and charging behavior. |

## 7. KPI Definitions (What / How)

## KPI 1: EV Net Energy Efficiency
- Definition: `energy_used_Wh / distance_km`.
- Inputs:
  - preferred: battery power integration over trip window.
  - fallback: SoC delta * estimated usable capacity.

## KPI 2: EV Estimated Practical Range
- Definition: `remaining_usable_energy_Wh / rolling_Wh_per_km`.
- Inputs: current SoC, estimated usable battery capacity, rolling efficiency.
- Behavior: if battery capacity estimate is uncertain, show lower confidence.

## KPI 3: Cold-Weather Range Retention
- Definition: `100 * (median_practical_range_cold / median_practical_range_mild)`.
- Temperature windows:
  - `cold_set`: `very_cold + cold`
  - `mild_set`: `mild`
- Inputs:
  - practical range observations
  - temperature bins
- Interpretation: `85%` means cold-weather range is about 85% of mild-weather range.

## KPI 4: Range Temperature Sensitivity Index
- Definition: robust slope of practical range against ambient temperature, normalized to `% range loss per 10C drop`.
- Inputs:
  - practical range observations
  - ambient temperature (or estimated ambient)
- Interpretation: lower value means better cold-weather resilience.

## KPI 5: Regeneration Recovery Ratio
- Definition: `regen_recovered_Wh / (regen_recovered_Wh + traction_consumed_Wh)`.
- Inputs: signed power/current where available.
- Fallback: mark `not_supported` if EV signal set cannot separate regen from traction.

## KPI 6: EV Urban Efficiency
- Definition: Wh/km on segments where avg speed < 45 km/h.
- Inputs: speed + EV energy estimate.

## KPI 7: EV Highway Efficiency
- Definition: Wh/km on segments where avg speed >= 80 km/h.
- Inputs: speed + EV energy estimate.

## KPI 8: Temp-Adjusted Charge Acceptance Score
- Definition: score from observed charging power against expected charging power for matched `SoC bin + battery_temp bin + charger_type`.
- Inputs:
  - SoC
  - battery temperature
  - battery voltage/current or derived charging power
  - charge state and charger type (AC/DC when detectable)
- Example normalization: `score = clamp(100 * median(observed_kw / expected_kw), 0, 100)`.
- Fallback: if battery temperature is missing, compute unadjusted acceptance score and mark lower confidence.

## KPI 9: Cold-Weather Charge Speed Retention
- Definition: `100 * (median_charge_power_cold / median_charge_power_mild)` with matching by `SoC band + charger_type`.
- Temperature windows:
  - `cold_set`: `very_cold + cold`
  - `mild_set`: `mild`
- Inputs:
  - charging power time series
  - SoC during charging
  - battery temperature (preferred) and ambient temperature
  - charger type
- Interpretation: `70%` means charging in cold is ~30% slower vs mild conditions for equivalent context.

## KPI 10: EV Composite Score (with Health Modifier)
- Base score: weighted blend of KPIs 1-9.
- Health modifier (parked diagnostics):
  - small penalty cap (example: max -10 points) based on MIL state + active DTC burden.
- Rationale: include immediate parked signal value without dominating EV efficiency and charging ranking.

## 8. Ranking Views

## EV Range & Efficiency Ranking (Primary)
- Uses KPIs: 1,2,3,4,5,6,7.
- Cohorts: `bev` and optionally `phev_electric_mode`.
- Display:
  - percentile within cohort
  - confidence badge (`preview`/`medium`/`stable`)
  - temperature filter (`very_cold`, `cold`, `cool`, `mild`, `hot`, `all`)

## EV Charging Performance Ranking
- Uses KPIs: 8,9.
- Cohorts: same as EV ranking.
- Display:
  - percentile within cohort
  - charger context (AC/DC when available)
  - temperature filter
  - confidence badge and telemetry completeness caveat

## EV Temperature Impact Comparison (Model vs Model)
- Uses KPIs: 3,4,9.
- Purpose: compare models across temperature bins, emphasizing cold-climate behavior.
- Display:
  - side-by-side model comparison for cold vs mild delta
  - rank deltas in `very_cold` and `cold` bins
  - sample-size and confidence flags

## EV Composite Ranking
- Uses KPI 10.
- Display:
  - overall percentile
  - drive-score and charging-score breakdown
  - temperature-impact summary
  - health modifier note

## Parked Health Card (Context)
- Inputs: MIL + active DTCs + readiness where available.
- Purpose: immediate pre-drive insight and composite modifier context.

## 9. Signal Requirements (OBD)

## Core required
- Speed
- Distance/odometer progression
- Timestamped samples

## EV driving required
- SoC
- Battery voltage/current or derived power (preferred)
- Charge/discharge state indicators where available
- Ambient temperature (preferred for temperature KPIs)
- Battery temperature (preferred for temperature + charging KPIs)

## EV charging required
- Charge state (charging/not charging)
- SoC during charging
- Battery voltage/current or derived charging power
- Battery temperature (preferred for KPI 8/9 normalization)
- Charger type context when available
- Charging-session boundaries (start/stop markers)

## Temperature estimation fallback
- If ambient temperature is unavailable from OBD, derive a coarse ambient estimate from timestamp + user-approved coarse location context (city-level), then mark lower confidence.
- If battery temperature is missing, temperature-sensitive charging KPIs remain available with downgraded confidence and explicit caveat.

## Diagnostics for health modifier/context
- Mode 01 PID 01 (MIL + readiness)
- Mode 03 DTC list

If a signal is unavailable, dependent KPI remains optional with explicit confidence/status handling.

## 10. Data Quality and Confidence
- Every KPI carries `confidence_level`: `preview`, `medium`, `stable`.
- Confidence increases with:
  - sample volume
  - number of trips and qualifying charge sessions
  - coverage across temperature bins (especially `cold` and `very_cold`)
  - signal completeness
  - direct signal path availability (vs fallback estimation)
- Rankings must show confidence to avoid over-trusting early sparse data.

## 11. Privacy and Retention Alignment
- Persist salted VIN hash only.
- No precise GPS required for MVP KPI set.
- Retain most raw telemetry in S3 for future KPI redesign/backtesting.
- Keep serving layer in normalized Postgres for low-latency ranking APIs.

## 12. Acceptance Criteria (MVP)
1. EV user sees EV efficiency/range ranking preview after first qualifying drive window.
2. EV user sees charging performance preview after first qualifying charge session.
3. EV user can view temperature-filtered results (including `cold` and `very_cold`) once minimum sample thresholds are met.
4. EV user can compare model cold-vs-mild range and charging deltas through temperature impact view.
5. New user sees parked health card immediately if diagnostic read succeeds.
6. EV composite score includes a visible health modifier note.
7. Missing optional signals do not break ranking; confidence/caveats are shown.
8. Raw telemetry is archived to S3 and retrievable for offline recomputation.
9. ICE-only and HEV-only vehicles are not shown in MVP ranking flows.

## 13. Open Items to Finalize
1. Exact temperature-bin thresholds and whether they are global or region-adjusted.
2. Battery capacity estimation strategy for KPI 2 (per-vehicle learned vs cohort prior).
3. Baseline strategy for KPI 3 and KPI 9 (`cold` vs `mild`, model prior vs global prior).
4. Range sensitivity modeling choice for KPI 4 (robust linear vs piecewise).
5. Composite weights for KPI 10 (range/efficiency vs charging vs health modifier).
6. DTC/MIL penalty mapping for health modifier.
7. UI copy for confidence levels and temperature caveats.
