# App Review Notes (Draft)

Status: Draft v1  
Date: 2026-02-20  
Audience: Apple App Review.

## 1. Copy/Paste Draft For App Store Connect

`Thank you for reviewing Car Ranks.`

`Car Ranks is an EV analytics app that uses OBD telemetry to show read-only insights for EV range, charging, and temperature impact.`

`Test account:`
- `Email: apple-review@car-ranks.app`
- `Password: CarRanks-AppReview-2026!`

`Primary flow to test:`
- `Sign in`
- `Select/link vehicle`
- `Open KPI views (range, charging, readiness, temperature impact)`
- `Open rankings view`

`Important constraints for MVP:`
- `OBD telemetry source only`
- `Read-only app (no remote commands)`
- `iOS only`

`If the account has limited telemetry, some cards may show preview/readiness states until enough samples are available.`

`Note: provision the exact App Review account in the production auth system before submitting the build.`

## 2. Internal Reviewer Notes

- Backend smoke baseline evidence:
  - `/Users/albinocordeiro/Code/car_ranks/docs/smoke/postgres-local-20260220T233027Z/summary.md`
  - CI smoke run: [22245386692](https://github.com/albinocordeiro/car-ranks/actions/runs/22245386692)
- Contracts for expected API payloads:
  - `/Users/albinocordeiro/Code/car_ranks/docs/contracts/kpi-api.md`
  - `/Users/albinocordeiro/Code/car_ranks/docs/contracts/ranking-api.md`
  - `/Users/albinocordeiro/Code/car_ranks/docs/contracts/internal-jobs-api.md`

## 3. Finalization Checklist

- [ ] Test credentials created and verified in production-like environment.
- [ ] Review note text aligned with final shipped app behavior.
- [ ] Known limitations list reviewed by product and engineering.
