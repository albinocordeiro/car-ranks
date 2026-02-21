# iOS UI Checkpoint Manifest

- Timestamp (UTC): 20260221-143413
- Checkpoint: batch-c-live-pass
- Device: iPhone 16 Pro
- Mode: live

## Screen List
1. 01-rankings-loading-live.png
2. 02-rankings-success-live.png
3. 03-rankings-empty-live.png
4. 04-rankings-error-live.png
5. 05-dev-session-panel-live.png

## State Coverage
- Rankings loading: 01-rankings-loading-live.png
- Rankings success: 02-rankings-success-live.png
- Rankings empty: 03-rankings-empty-live.png
- Rankings error: 04-rankings-error-live.png
- Dev Session Panel: 05-dev-session-panel-live.png

## Acceptance Notes
- Live pass completed against local backend on 2026-02-21.
- Observed mismatch: rankings captures rendered backend error `no ranking snapshot found for requested filter`.
- Follow-up: run telemetry ingest + recompute/ranking jobs for the selected user/vehicle, then rerun Batch C live capture.
