# Golden Run Playbook (EV Return)

Purpose: execute one high-value capture run and avoid repeated car/dongle sessions unless a fixture gap is proven.

## 1. Backend Start

```bash
cd /Users/albinocordeiro/Code/car_ranks/backend
export DATABASE_URL='postgres://<user>:<pass>@<host>:5432/<db>'
export BIND_ADDR='0.0.0.0:8080'
cargo run
```

## 2. DB + API Health Check

```bash
curl -sS http://127.0.0.1:8080/health | jq .
curl -sS -H 'x-user-id: <user_uuid>' \
  "http://127.0.0.1:8080/v1/telemetry/raw?vehicle_uid=<vehicle_uuid>&limit=1" | jq .
```

## 3. iPhone Build + Install

```bash
cd /Users/albinocordeiro/Code/car_ranks/ios
xcodebuild \
  -project CarRanksApp.xcodeproj \
  -scheme CarRanksApp \
  -destination 'id=00008110-000419AA0C0A801E' \
  -configuration Debug \
  API_BASE_URL='http://<your-mac-lan-ip>:8080' \
  build
```

Then run/install from Xcode on the connected iPhone.

## 4. API Base Validation From Phone

1. Ensure iPhone and Mac are on the same network.
2. In app dev session/config, set `API_BASE_URL` to `http://<your-mac-lan-ip>:8080`.
3. Verify one live read (for example KPI Me refresh) succeeds.

## 5. Golden Capture Protocol (8–12 minutes)

1. Accessory/awake state for 60–90s.
2. READY state idle for 60–90s.
3. Drive 4–6 minutes with:
  - one steady cruise segment,
  - one acceleration/deceleration segment,
  - one complete stop.
4. Stop capture.
5. Upload pending batch once.
6. Copy `batch_id`/`ingest_id` from app UI.

## 6. Export Investigation Pack Immediately

```bash
cd /Users/albinocordeiro/Code/car_ranks
tools/capture/export_run_pack.sh \
  --api-base 'http://127.0.0.1:8080' \
  --user-id '<user_uuid>' \
  --vehicle-uid '<vehicle_uuid>' \
  --batch-id '<batch_uuid>' \
  --output-dir '/Users/albinocordeiro/Code/car_ranks/docs/captures'
```

Alternative by session:

```bash
tools/capture/export_run_pack.sh \
  --api-base 'http://127.0.0.1:8080' \
  --user-id '<user_uuid>' \
  --vehicle-uid '<vehicle_uuid>' \
  --session-id '<session_uuid>' \
  --output-dir '/Users/albinocordeiro/Code/car_ranks/docs/captures'
```

## 7. Fixture Curation

1. Inspect generated `signal-summary.md`.
2. Promote stable artifacts into:
   - `/Users/albinocordeiro/Code/car_ranks/docs/captures/curated/<fixture-name>/`
3. Keep raw run packs local unless intentionally sharing.

## Stop Rule (Mandatory)

Do not run another real capture until a fixture gap is confirmed.

A new capture is allowed only when:

1. the missing scenario cannot be reproduced from existing curated fixtures, and
2. the missing scenario is required for a blocker-level decision or fix.
