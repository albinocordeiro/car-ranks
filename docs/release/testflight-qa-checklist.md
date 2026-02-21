# TestFlight QA Checklist (MVP Release Candidate)

Status: Draft v1  
Date: 2026-02-20  
Goal: define explicit go/no-go checks before App Store submission.

## 1. Build and Distribution Gate

- [ ] Release build created from `main` at commit `TODO`.
- [ ] Build signed with production provisioning profile.
- [ ] Build uploaded to TestFlight and processing completed.
- [ ] Internal testers added and notified.

## 2. Core Flow Validation

- [ ] Sign-in works with production configuration.
- [ ] Vehicle linkage/selection flow works end-to-end.
- [ ] KPI views load correctly for MVP scenarios.
- [ ] Rankings view loads correctly for MVP scenarios.
- [ ] Empty/loading/error states display correctly for each core view.

## 3. Stability and Defect Gate

- [ ] No unresolved P0 issues.
- [ ] No unresolved P1 issues accepted for MVP.
- [ ] Crash-free verification completed on target iOS versions/devices.
- [ ] Backend smoke baseline re-run after latest blocker fix.

## 4. Backend Readiness Cross-Check

- [ ] Latest smoke summary reviewed:
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/summary.md`
- [ ] Manual CI smoke run green:
  - [run 22245386692](https://github.com/albinocordeiro/car-ranks/actions/runs/22245386692)
- [ ] Postgres integration tests green in local/staging environment.

## 5. App Store Package Gate

- [ ] Metadata finalized (`app-store-metadata.md`).
- [ ] Privacy answers finalized (`app-privacy-questionnaire.md`).
- [ ] Review notes finalized (`app-review-notes.md`).
- [ ] Screenshots finalized (`screenshot-plan.md`).
- [ ] Privacy policy and support URLs live and verified.

## 6. Final Decision

- [ ] Go/No-Go decision logged with date/time.
- [ ] If Go: submit build to App Review.
- [ ] If No-Go: list blockers and next decision checkpoint.
