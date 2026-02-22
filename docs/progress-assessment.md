# Progress Assessment (2026-02-20)

## Current Status

Completed backend milestones:
- Milestone 1: auth + ownership enforcement across public vehicle-bound APIs.
- Milestone 2: Postgres parity for ingest, rankings, temperature-impact KPIs, and internal job routes.
- Test coverage expanded to validate Postgres ingest idempotency/ownership, rankings, temperature-impact, and internal job execution.
- Auth-scope coverage now explicitly validates foreign-user rejection across all public vehicle-bound KPI handlers and scoped ranking visibility.
- Priority 1 delivered: `GET /v1/kpis/readiness` now returns per-family readiness, confidence, and gate diagnostics.
- Priority 2 partially delivered: internal job runs are now persisted in `internal_job_run`, and `GET /internal/jobs/latest` exposes latest run status.
- Priority 4 delivered baseline: internal job endpoints enforce per-job-kind lease locks, status and conflict payloads expose lock owner/expiry metadata, and stale `running` rows are auto-recovered on next trigger.
- Priority 3 delivered: recompute now runs fully natively in Postgres across charging, range, temperature, and composite families.

Current runtime status:
- Postgres mode: full MVP API surface available and is the MVP runtime target.
- Runtime bootstrap requires a Postgres `DATABASE_URL`.
- Internal KPI/ranking recompute in Postgres mode is fully native.

Recent verification evidence:
- Latest local smoke baseline: `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/summary.md` (all endpoint captures `200`).
- Smoke runner automation: `/Users/albinocordeiro/Code/car_ranks/docs/smoke/scripts/run_local_postgres_smoke.sh`.
- Manual CI smoke workflow: `Postgres Smoke (Manual)` at [run 22245386692](https://github.com/albinocordeiro/car-ranks/actions/runs/22245386692) with uploaded artifact `postgres-smoke-1`.

## Risks and Gaps

- Main MVP risk remains mobile/release readiness and cross-surface stability.
- Backend still needs ongoing P0/P1 defect triage until submission freeze.
- iOS bootstrap blocker is cleared: app target, simulator test matrix, and physical-device install path now exist.

## Next Product Development Plan

Priority 1: MVP ship readiness
- Freeze scope to must-have product behavior and defer non-blocking architecture work.
- Focus engineering time on release blockers: correctness, crash risk, ownership/auth safety, and UX-critical API stability.
- Product runtime focus is Postgres-only for MVP.

Priority 2: Reviewer-friendly implementation gate
- Every change must be reviewer-friendly by default:
  - one responsibility per file/module,
  - small focused functions and explicit naming,
  - comments that explain intent/tradeoffs (why/how),
  - small commits with matching tests/docs.
- Avoid opportunistic refactors outside MVP scope.

Priority 3: iOS replay/live hardening (current mobile blocker)
- Use deterministic replay fixtures to harden OBD parsing, capture orchestration, and upload UX without EV availability.
- Keep one-command capture export pipeline ready so the next EV run is validation-first instead of discovery-first.

Priority 4: Release operations package
- Metadata/privacy/review-note drafts are prepared and should be finalized in parallel with iOS implementation.

Priority 5: Defect burn-down
- Track and close backend/mobile P0/P1 issues daily.
- Keep smoke baseline current after blocker fixes using the one-command runner and manual CI workflow.

## Suggested Execution Order

1. Stabilize and commit the capture-once foundation (session correlation, raw API pagination, run-pack export, tooling/docs).
2. Add replay fixture harness and offline tests for parser, capture coordinator, and upload state handling.
3. Run device-level crash-free smoke on iOS targets and fix blocker defects.
4. Finalize App Store operations package (metadata, screenshots, privacy/support URLs, review notes, TestFlight QA sign-off).
5. Maintain daily P0/P1 triage plus smoke re-validation through submission.
