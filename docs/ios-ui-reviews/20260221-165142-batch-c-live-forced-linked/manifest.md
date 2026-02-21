# iOS UI Checkpoint Manifest

- Timestamp (UTC): 20260221-165142
- Checkpoint: batch-c-live-forced-linked
- Device: iPhone 16 Pro
- Mode: live
- Live Capture Override: force-states

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
- Accepted on 2026-02-21.
- Success screen is backed by staging data for vehicle `e11889bf-504c-4238-9583-bc8840f20e19`.
- Empty and error screens are deterministic via `LIVE_CAPTURE_OVERRIDE_MODE=force-states`.
