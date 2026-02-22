# Capture Packs

This directory stores exported real-world OBD capture packs produced by:

- `/Users/albinocordeiro/Code/car_ranks/tools/capture/export_run_pack.sh`

## Workflow

1. Run a real capture once.
2. Upload telemetry from iOS.
3. Export a pack by `batch_id` or `session_id`.
4. Investigate with generated artifacts.
5. Promote only stable fixture subsets into `docs/captures/curated/` for commit.

## Curation Policy

- Generated per-run packs are local artifacts under:
  - `docs/captures/<timestamp>-<batch|session>-<id>/`
- Curated fixtures are committed under:
  - `docs/captures/curated/<fixture-name>/`

Each curated fixture should contain:

1. `ios-command-fixture.json`
2. `manifest.md`
3. optional helper artifacts (for example small derived summaries used by tests)

## Exporter Guarantees

`export_run_pack.sh` now enforces:

1. strict required-arg validation (`http(s)` API base + UUID checks),
2. cursor-loop guard to prevent infinite pagination,
3. deterministic output artifact structure validation before success.

`signal-summary.md` includes:

1. signal availability ratio,
2. top command clusters,
3. dedicated `NO DATA` command clusters,
4. dedicated error command clusters.
