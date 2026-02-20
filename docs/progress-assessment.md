# Progress Assessment (2026-02-20)

## Current Status

Completed backend milestones:
- Milestone 1: auth + ownership enforcement across public vehicle-bound APIs.
- Milestone 2: Postgres parity for ingest, rankings, temperature-impact KPIs, and internal job routes.
- Test coverage expanded to validate Postgres ingest idempotency/ownership, rankings, temperature-impact, and internal job bridge execution.

Current runtime status:
- SQLite mode: full MVP API surface available.
- Postgres mode: full MVP API surface now available.
- Internal KPI/ranking recompute in Postgres mode currently uses a SQLite bridge (input sync -> SQLite compute -> output sync).

## Risks and Gaps

- Postgres KPI/ranking jobs are bridge-based and perform full-table syncs, which is acceptable for MVP but not scalable.
- Internal jobs lack distributed locking and scheduling guardrails for multi-instance deployments.
- Product-facing readiness/confidence progress for new users is implicit in KPI data and not yet exposed as a dedicated API view.

## Next Product Development Plan

Priority 1: User readiness and confidence visibility
- Add a readiness summary endpoint for a vehicle that reports preview/medium/stable status by ranking family.
- Return explicit gating reasons (for example: missing cold distance, missing charging sessions).

Priority 2: KPI freshness and compute reliability
- Add internal job run metadata tables and idempotent job-run tracking.
- Add lightweight lock/lease semantics so concurrent internal job triggers are safe.

Priority 3: Postgres-native compute path
- Replace bridge-based Postgres internal jobs with native Postgres computation stages.
- Keep module boundaries aligned with single responsibility (charging sessions, KPI recompute, ranking snapshots).

Priority 4: Product contract hardening
- Add contract examples for readiness/freshness responses under `docs/contracts/examples/`.
- Add API tests for new readiness and freshness paths in both SQLite and Postgres modes.

## Suggested Execution Order

1. Implement readiness endpoint and tests (fastest user-visible product gain).
2. Add job-run metadata + lock semantics to stabilize operations.
3. Migrate job computation to Postgres-native stages and deprecate bridge sync path.
4. Update docs/contracts once new product endpoints are stable.
