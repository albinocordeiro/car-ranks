# iOS UI Checkpoint Manifest

- Timestamp (UTC): 20260221-144101
- Checkpoint: batch-a-live-linked
- Device: iPhone 16 Pro
- Mode: live

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
- Live pass completed against local staging backend on 2026-02-21 with linked default user/vehicle IDs.
- KPI Me live data rendered successfully (no access-denied contract errors).
- Limitation: in live mode, `kpi-me-empty` and `kpi-me-error` scenarios resolve to the backend response and currently render success.
