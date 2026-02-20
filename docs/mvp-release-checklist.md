# MVP Release Checklist (2026-02-20)

Goal: submit the MVP iOS app to App Store review by **February 28, 2026**.

## Operating Constraints

- Scope freeze: no non-blocking architecture work before submission.
- Runtime target for MVP: PostgreSQL only.
- Reviewer-friendly gate on every change:
  - one clear responsibility per file/module,
  - small focused functions with explicit names,
  - comments explain intent/tradeoffs (why/how),
  - small commits with matching tests/docs.
- Blocker-only rule: prioritize correctness, stability, and release readiness.

## Must-Have Scope (Ship Blockers)

### Backend

- [x] End-to-end path validated for PostgreSQL runtime: ingest -> recompute job -> KPI reads -> ranking reads. (`Due: 2026-02-24`, `Completed: 2026-02-20`, `Status: DONE`)
- [x] Ownership/auth checks verified across all public vehicle-bound endpoints. (`Due: 2026-02-23`, `Completed: 2026-02-20`, `Status: DONE`)
- [x] Internal recompute job reliability verified: lock conflicts, stale-run recovery, latest-status metadata. (`Due: 2026-02-23`, `Completed: 2026-02-20`, `Status: DONE`)
- [x] Critical API contracts current for mobile consumption (KPI, ranking, internal jobs). (`Due: 2026-02-26`, `Completed: 2026-02-20`, `Status: DONE`)
- [ ] No open P0/P1 backend defects. (`Due: 2026-02-27`, `Status: TODO`)

### Mobile App

- [ ] Sign-in/auth flow complete for production configuration. (`Due: 2026-02-24`, `Status: TODO`)
- [ ] Vehicle linkage and vehicle selection flow complete. (`Due: 2026-02-24`, `Status: TODO`)
- [ ] KPI and ranking views complete for MVP scenarios. (`Due: 2026-02-25`, `Status: TODO`)
- [ ] Empty/loading/error states implemented and verified for all core screens. (`Due: 2026-02-25`, `Status: TODO`)
- [ ] Crash-free smoke run on target devices and latest iOS versions in scope. (`Due: 2026-02-26`, `Status: TODO`)
- [ ] No open P0/P1 mobile defects. (`Due: 2026-02-27`, `Status: TODO`)

### Release Operations

- [ ] App Store Connect app metadata complete (name, subtitle, description, keywords, category). (`Due: 2026-02-26`, `Status: TODO`)
- [ ] Screenshots prepared for required device classes. (`Due: 2026-02-26`, `Status: TODO`)
- [ ] App icon and launch assets finalized. (`Due: 2026-02-25`, `Status: TODO`)
- [ ] Privacy policy URL and support URL published and verified. (`Due: 2026-02-25`, `Status: TODO`)
- [ ] App Privacy questionnaire completed and consistent with implementation. (`Due: 2026-02-26`, `Status: TODO`)
- [ ] Review notes drafted for App Review (test account, key flows, known constraints). (`Due: 2026-02-27`, `Status: TODO`)
- [ ] TestFlight build signed, distributed, and accepted by internal QA. (`Due: 2026-02-26`, `Status: TODO`)

## Explicitly Deferred Until After Submission

- Broad refactors not tied to release blockers.
- Non-critical performance tuning and cleanup-only changes.
- Nice-to-have UX enhancements that do not affect MVP acceptance.

## Timeline (Concrete Dates)

- **February 20, 2026**: Scope freeze, checklist lock, due-date assignment.
- **February 21-24, 2026**: Backend/mobile blocker fixes and integration verification.
- **February 24-26, 2026**: TestFlight stabilization and release-candidate hardening.
- **February 26-27, 2026**: App Store metadata/privacy/review package finalization.
- **February 27-28, 2026**: Final go/no-go, submit to App Store review.

## Today (2026-02-20) In Progress

- [x] Scope freeze confirmed across current feature work. (`Due: 2026-02-20`, `Completed: 2026-02-20`, `Status: DONE`)
- [x] MVP checklist locked as the active source of truth. (`Due: 2026-02-20`, `Completed: 2026-02-20`, `Status: DONE`)
- [x] Due dates assigned for all unchecked must-have items. (`Due: 2026-02-20`, `Completed: 2026-02-20`, `Status: DONE`)

## Evidence Snapshot (2026-02-20)

- End-to-end Postgres smoke baseline captured at `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/summary.md` with all endpoint captures returning `200`.
- Manual CI smoke workflow succeeded: [run 22245386692](https://github.com/albinocordeiro/car-ranks/actions/runs/22245386692), artifact `postgres-smoke-1`.
- Integration tests confirm auth/ownership and job reliability:
  - `postgres_ingest_enforces_idempotency_and_vehicle_ownership_when_env_set`
  - `postgres_public_vehicle_handlers_enforce_user_scope_when_env_set`
  - `postgres_latest_job_status_reports_active_lock_metadata_when_env_set`
  - `postgres_recompute_job_conflict_includes_lock_diagnostics_when_env_set`
  - `postgres_internal_job_handler_runs_native_pipeline_when_env_set`
- API contracts are updated in `/Users/albinocordeiro/Code/car_ranks/docs/contracts/` (`kpi-api.md`, `ranking-api.md`, `internal-jobs-api.md`, and matching `examples/*.json`).

## Daily Exit Criteria

- [ ] No unresolved P0 issues. (`Due: Daily EOD`, `Status: TODO`)
- [ ] P1 count decreasing day-over-day. (`Due: Daily EOD`, `Status: TODO`)
- [ ] New code merged only with reviewer-friendly structure and tests/docs updates. (`Due: Daily EOD`, `Status: TODO`)
- [ ] Checklist status updated before end of day. (`Due: Daily EOD`, `Status: TODO`)
