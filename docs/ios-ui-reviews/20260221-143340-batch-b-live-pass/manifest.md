# iOS UI Checkpoint Manifest

- Timestamp (UTC): 20260221-143340
- Checkpoint: batch-b-live-pass
- Device: iPhone 16 Pro
- Mode: live

## Screen List
1. 01-kpi-charging-loading-live.png
2. 02-kpi-charging-success-live.png
3. 03-kpi-charging-empty-live.png
4. 04-kpi-charging-error-live.png
5. 05-kpi-readiness-loading-live.png
6. 06-kpi-readiness-success-live.png
7. 07-kpi-readiness-empty-live.png
8. 08-kpi-readiness-error-live.png
9. 09-kpi-temperature-impact-loading-live.png
10. 10-kpi-temperature-impact-success-live.png
11. 11-kpi-temperature-impact-empty-live.png
12. 12-kpi-temperature-impact-error-live.png
13. 13-dev-session-panel-live.png

## State Coverage
- KPI Charging loading: 01-kpi-charging-loading-live.png
- KPI Charging success: 02-kpi-charging-success-live.png
- KPI Charging empty: 03-kpi-charging-empty-live.png
- KPI Charging error: 04-kpi-charging-error-live.png
- KPI Readiness loading: 05-kpi-readiness-loading-live.png
- KPI Readiness success: 06-kpi-readiness-success-live.png
- KPI Readiness empty: 07-kpi-readiness-empty-live.png
- KPI Readiness error: 08-kpi-readiness-error-live.png
- KPI Temperature Impact loading: 09-kpi-temperature-impact-loading-live.png
- KPI Temperature Impact success: 10-kpi-temperature-impact-success-live.png
- KPI Temperature Impact empty: 11-kpi-temperature-impact-empty-live.png
- KPI Temperature Impact error: 12-kpi-temperature-impact-error-live.png
- Dev Session Panel: 13-dev-session-panel-live.png

## Acceptance Notes
- Live pass completed against local backend on 2026-02-21.
- Observed mismatch: KPI Charging, KPI Readiness, and KPI Temperature all rendered access-denied errors for non-loading captures.
- Follow-up: use a valid staged user/vehicle link in Dev Session panel, then rerun Batch B live capture.
