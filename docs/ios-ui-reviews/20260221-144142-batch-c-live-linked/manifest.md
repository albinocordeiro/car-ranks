# iOS UI Checkpoint Manifest

- Timestamp (UTC): 20260221-144142
- Checkpoint: batch-c-live-linked
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
- Live pass completed against local staging backend on 2026-02-21 with linked default user/vehicle IDs.
- Rankings screen rendered a successful live payload for `ranking_type=ev_temperature_impact`, `timeframe=90d`, `temperature_bin=all`.
- Limitation: in live mode, `rankings-empty` and `rankings-error` scenarios resolve to the backend response and currently render success.
