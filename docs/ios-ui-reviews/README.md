# iOS UI Review Checkpoints

This folder stores simulator screenshot packs for each visual iteration batch.

## Naming

Checkpoint folders use this convention:

`<UTC timestamp>-<checkpoint-name>/`

Example:

`20260221-103000-batch-a-initial/`

## Required Files Per Checkpoint

Each checkpoint should include a batch-specific minimum set:

Batch A (`kpi-me` slice):
- `01-kpi-me-loading.png`
- `02-kpi-me-success.png`
- `03-kpi-me-empty.png`
- `04-kpi-me-error.png`
- `05-dev-session-panel.png`

Batch B (`charging`, `readiness`, `temperature-impact` slices):
- `01-kpi-charging-loading.png`
- `02-kpi-charging-success.png`
- `03-kpi-charging-empty.png`
- `04-kpi-charging-error.png`
- `05-kpi-readiness-loading.png`
- `06-kpi-readiness-success.png`
- `07-kpi-readiness-empty.png`
- `08-kpi-readiness-error.png`
- `09-kpi-temperature-impact-loading.png`
- `10-kpi-temperature-impact-success.png`
- `11-kpi-temperature-impact-empty.png`
- `12-kpi-temperature-impact-error.png`
- `13-dev-session-panel.png`

Batch C (`rankings` slice):
- `01-rankings-loading.png`
- `02-rankings-success.png`
- `03-rankings-empty.png`
- `04-rankings-error.png`
- `05-dev-session-panel.png`

All checkpoints:
- `manifest.md`

When captured in live mode, screenshot filenames include a `-live` suffix.

## Capture Command

From repository root:

```bash
ios/scripts/capture_checkpoint.sh --checkpoint batch-a-initial --device "iPhone 16 Pro" --mode mock
```

Use `--mode live` at batch boundaries for staging validation passes.
