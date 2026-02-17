# OBD-II + Smartcar Technical Brief

Date: 2026-02-17
Scope: technical due diligence on OBD-II, Smartcar, iOS integration APIs, and near-term implementation priorities.
Deferred: white-label hardware/vendor decisions.

## Executive summary
1. OBD-II gives broad standardized diagnostics through SAE J1979 modes/PIDs, but adapter/protocol behavior varies by hardware.
2. Smartcar gives OEM-cloud access to standardized vehicle signals and commands, but compatibility is region/vehicle dependent and some compatibility checks are Enterprise-only.
3. Smartcar REST should be treated as low-frequency; webhooks are the scalable path for fresher state.
4. iOS stack is clear: `ASWebAuthenticationSession` + backend for Smartcar OAuth/webhooks, and `CoreBluetooth` (or `ExternalAccessory` for MFi accessories) for dongle workflows.
5. Early cost anchor: Smartcar Build starts at `$1.99` per connected vehicle/month.

## OBD-II: what data you can reliably expect
- Regulatory/inspection flows explicitly rely on `Mode $01, PID $01` (readiness + MIL status) and `Mode $03` (stored DTCs) in SAE J1979 workflows.
- Readiness completion and MIL behavior determine pass/fail logic in the cited CFR procedure.
- ELM327 documentation confirms the common adapter model: OBD bus translation to serial, AT command channel, and support for multiple legacy/CAN protocols.

### Practical implications for your app
- You can build a robust baseline around: readiness, DTCs, MIL state, freeze-frame, and common PIDs.
- Normalize by protocol/ECU quirks in your ingestion layer; do not assume uniform support across vehicles/adapters.
- Treat mode 04 (clear codes) as privileged and explicit user action only.

## Smartcar: capabilities, constraints, and cost signals

### Capabilities and API model
- Vehicles API v3 base URL is `https://vehicle.api.smartcar.com/v3`; v2 base URL remains `https://api.smartcar.com/v2.0`.
- v2.0 is marked for deprecation by Q3 2026, but remote commands are still v2-only today.
- Permissions map cleanly into `read_*` and `control_*` scopes (e.g., `read_odometer`, `read_location`, `control_security`, `control_charge`).

### Data freshness and webhooks
- Smartcar explicitly frames REST usage as non-frequent retrieval, with typical updates around every 24 hours unless webhook-subscribed.
- Webhook receiver requirements are strict: respond in 15s; failures retry up to 3 times (4 total attempts) with exponential backoff (25s, 50s, 100s).
- After 4 failed attempts, that payload is dropped.

### Compatibility and regional boundaries
- Compatibility checks vary by model/year/trim and are non-guaranteed reference outputs.
- Compatibility API endpoints in docs are labeled Enterprise features.
- By-region/make supports `US`, `CA`, `EUROPE`.
- By-VIN capability checking is US-only for capabilities (not CA/EU capability checks).
- Product page capture states 37 brands, but also warns compatibility variability and recommends VIN-based checks for specificity.

### Pricing signals (captured)
- Free: `$0`, 1 connected vehicle.
- Build: starting at `$1.99` per connected vehicle/month; up to 100 connected vehicles; 100 command API calls/month in captured table/card.
- Custom: indicates custom pricing and a captured compare-table note of `500 minimum`.

## iOS APIs needed (and why)
- `ASWebAuthenticationSession`: user OAuth flow for Smartcar Connect from app.
- `URLSession`: token exchange (through backend), API calls, telemetry sync.
- `CoreBluetooth`: BLE communication with non-MFi OBD dongles.
- `ExternalAccessory`: accessory comms when using MFi-style accessories/protocols.
- `BackgroundTasks` and `UIBackgroundModes`: controlled background refresh/processing.
- `NSBluetoothAlwaysUsageDescription`: required permission rationale string for Bluetooth access.

## Suggested technical architecture (now)
1. Build a canonical vehicle schema in your backend (`vehicle_identity`, `trip/session`, `signals`, `diagnostics`).
2. Implement two ingestion adapters:
- OBD adapter: PID polling + DTC/MIL readiness pipeline.
- Smartcar adapter: OAuth + webhooks + selective REST fallback.
3. Compute ranking KPIs on normalized data, not directly on raw adapter payloads.
4. Gate risky write operations (`control_*`, mode 04) behind explicit UX confirmations and audit logs.

## 2-week execution plan (technical-only)
1. Create signal catalog v0.1 (source, unit, granularity, confidence) from OBD + Smartcar intersection.
2. Build Smartcar sandbox integration:
- OAuth connect flow
- webhook receiver with signature verification and idempotency
- ingest `read_vehicle_info`, `read_odometer`, `read_location`, `read_diagnostics` where available
3. Build OBD prototype ingestion with one known ELM327-class adapter and parse:
- mode 01 PID 01
- mode 03 DTCs
- core live PIDs needed for initial ranking KPIs
4. Publish KPI definition doc with fallback rules when a signal is missing.

## Open questions to resolve next
- Which initial KPI set is mandatory for ranking v1 (to define minimum viable signal matrix)?
- Is Smartcar Enterprise compatibility API required in phase 1, or can we tolerate connect-time incompatibility fallbacks?
- Which OBD adapter classes are in-scope for first-party support (BLE only vs BLE + Wi-Fi + MFi)?

## Known research gaps (for follow-up)
- `eCFR` direct page capture was blocked by anti-automation; current regulatory text in this workspace comes from downloaded CFR PDFs/text extracts.
- SAE/ANSI paywalled standard detail pages were not fully captured here; mode/PID behavior is currently grounded in CFR references and the ELM327 datasheet.
- Apple Developer pages are JavaScript-heavy; current local captures provide titles/descriptions but not full rendered API details.

## Evidence (source lines)

### OBD-II
- `research/sources/obd/cfr-40-sec85-2222.txt:136`
- `research/sources/obd/cfr-40-sec85-2222.txt:137`
- `research/sources/obd/cfr-40-sec85-2222.txt:170`
- `research/sources/obd/cfr-40-sec85-2222.txt:194`
- `research/sources/obd/cfr-40-sec85-2222.txt:208`
- `research/sources/obd/cfr-40-sec85-2222.txt:233`
- `research/sources/obd/elm327-datasheet.txt:10`
- `research/sources/obd/elm327-datasheet.txt:14`
- `research/sources/obd/elm327-datasheet.txt:84`
- `research/sources/obd/elm327-datasheet.txt:203`
- `research/sources/obd/elm327-datasheet.txt:522`
- `research/sources/obd/elm327-datasheet.txt:2540`
- `research/sources/obd/elm327-datasheet.txt:2719`
- `research/sources/obd/elm327-datasheet.txt:2732`
- `research/sources/obd/elm327-datasheet.txt:2837`

### Smartcar
- `research/sources/smartcar/llms-full.txt:671`
- `research/sources/smartcar/llms-full.txt:687`
- `research/sources/smartcar/llms-full.txt:713`
- `research/sources/smartcar/llms-full.txt:727`
- `research/sources/smartcar/llms-full.txt:889`
- `research/sources/smartcar/llms-full.txt:895`
- `research/sources/smartcar/llms-full.txt:905`
- `research/sources/smartcar/llms-full.txt:8712`
- `research/sources/smartcar/llms-full.txt:8716`
- `research/sources/smartcar/llms-full.txt:8729`
- `research/sources/smartcar/llms-full.txt:8746`
- `research/sources/smartcar/llms-full.txt:8758`
- `research/sources/smartcar/llms-full.txt:8874`
- `research/sources/smartcar/llms-full.txt:8894`
- `research/sources/smartcar/llms-full.txt:8901`
- `research/sources/smartcar/llms-full.txt:8904`
- `research/sources/smartcar/llms-full.txt:3638`
- `research/sources/smartcar/llms-full.txt:3667`
- `research/sources/smartcar/pricing.pretty.html:1642`
- `research/sources/smartcar/pricing.pretty.html:1657`
- `research/sources/smartcar/pricing.pretty.html:1692`
- `research/sources/smartcar/pricing.pretty.html:1707`
- `research/sources/smartcar/pricing.pretty.html:1715`
- `research/sources/smartcar/pricing.pretty.html:1947`
- `research/sources/smartcar/compatible-vehicles-product.pretty.html:1577`
- `research/sources/smartcar/compatible-vehicles-product.pretty.html:1604`
- `research/sources/smartcar/compatible-vehicles-product.pretty.html:1708`
- `research/sources/smartcar/compatible-vehicles-product.pretty.html:1743`
- `research/sources/smartcar/compatible-vehicles-product.pretty.html:1760`

### iOS docs metadata
- `research/sources/ios/aswebauthenticationsession.html:1`
- `research/sources/ios/urlsession.html:1`
- `research/sources/ios/corebluetooth.html:1`
- `research/sources/ios/externalaccessory.html:1`
- `research/sources/ios/backgroundtasks.html:1`
- `research/sources/ios/nsbluetoothalwaysusagedescription.html:1`
- `research/sources/ios/uibackgroundmodes.html:1`

### Blocked/partial captures
- `research/sources/obd/ecfr-85-2222.html:81`
- `research/sources/obd/sae-j1979da-ansi.html:1`
