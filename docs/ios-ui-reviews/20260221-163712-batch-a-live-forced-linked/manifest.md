# iOS UI Checkpoint Manifest

- Timestamp (UTC): 20260221-163712
- Checkpoint: batch-a-live-forced-linked
- Device: iPhone 16 Pro
- Mode: live
- Live Capture Override: force-states

## Screen List
1. 01-kpi-me-loading-live.png
2. 02-kpi-me-success-live.png
3. 03-kpi-me-empty-live.png
4. 04-kpi-me-error-live.png
5. 05-dev-session-panel-live.png

## State Coverage
- KPI Me loading: 01-kpi-me-loading-live.png
- KPI Me success: 02-kpi-me-success-live.png
- KPI Me empty: 03-kpi-me-empty-live.png
- KPI Me error: 04-kpi-me-error-live.png
- Dev Session Panel: 05-dev-session-panel-live.png

## Acceptance Notes
- Accepted on 2026-02-21.
- Live success frame is backed by staging data for vehicle `e11889bf-504c-4238-9583-bc8840f20e19`.
- Empty and error frames are deterministic via `LIVE_CAPTURE_OVERRIDE_MODE=force-states`.
