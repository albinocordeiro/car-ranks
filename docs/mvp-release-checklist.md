# MVP Release Checklist (2026-02-20)

Goal: submit the MVP iOS app to App Store review by **February 28, 2026**.

## Operating Constraints

- Scope freeze: no non-blocking architecture work before submission.
- Reviewer-friendly gate on every change:
  - one clear responsibility per file/module,
  - small focused functions with explicit names,
  - comments explain intent/tradeoffs (why/how),
  - small commits with matching tests/docs.
- Blocker-only rule: prioritize correctness, stability, and release readiness.

## Must-Have Scope (Ship Blockers)

### Backend

- [ ] End-to-end path validated in both backends: ingest -> recompute job -> KPI reads -> ranking reads.
- [ ] Ownership/auth checks verified across all public vehicle-bound endpoints.
- [ ] Internal recompute job reliability verified: lock conflicts, stale-run recovery, latest-status metadata.
- [ ] Critical API contracts current for mobile consumption (KPI, ranking, internal jobs).
- [ ] No open P0/P1 backend defects.

### Mobile App

- [ ] Sign-in/auth flow complete for production configuration.
- [ ] Vehicle linkage and vehicle selection flow complete.
- [ ] KPI and ranking views complete for MVP scenarios.
- [ ] Empty/loading/error states implemented and verified for all core screens.
- [ ] Crash-free smoke run on target devices and latest iOS versions in scope.
- [ ] No open P0/P1 mobile defects.

### Release Operations

- [ ] App Store Connect app metadata complete (name, subtitle, description, keywords, category).
- [ ] Screenshots prepared for required device classes.
- [ ] App icon and launch assets finalized.
- [ ] Privacy policy URL and support URL published and verified.
- [ ] App Privacy questionnaire completed and consistent with implementation.
- [ ] Review notes drafted for App Review (test account, key flows, known constraints).
- [ ] TestFlight build signed, distributed, and accepted by internal QA.

## Explicitly Deferred Until After Submission

- Remaining Postgres-native migration work for bridge-backed families.
- Broad refactors not tied to release blockers.
- Non-critical performance tuning and cleanup-only changes.
- Nice-to-have UX enhancements that do not affect MVP acceptance.

## Timeline (Concrete Dates)

- **February 20, 2026**: Scope freeze, checklist lock, owner assignment.
- **February 21-24, 2026**: Backend/mobile blocker fixes and integration verification.
- **February 24-26, 2026**: TestFlight stabilization and release-candidate hardening.
- **February 26-27, 2026**: App Store metadata/privacy/review package finalization.
- **February 27-28, 2026**: Final go/no-go, submit to App Store review.

## Daily Exit Criteria

- [ ] No unresolved P0 issues.
- [ ] P1 count decreasing day-over-day.
- [ ] New code merged only with reviewer-friendly structure and tests/docs updates.
- [ ] Checklist status updated before end of day.
