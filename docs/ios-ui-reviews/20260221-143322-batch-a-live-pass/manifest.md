# iOS UI Checkpoint Manifest

- Timestamp (UTC): 20260221-143322
- Checkpoint: batch-a-live-pass
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
- Live pass completed against local backend on 2026-02-21.
- Observed mismatch: all non-loading KPI Me captures rendered `vehicle access denied for this user`.
- Follow-up: set `x-user-id` and `vehicle_uid` in Dev Session panel to a linked pair before the next live pass.
