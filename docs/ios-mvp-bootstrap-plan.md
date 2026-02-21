# iOS MVP Bootstrap Plan

Status: Active  
Date: 2026-02-20  
Scope: establish the first shippable iOS client shell against the existing backend contracts.

## Why This Exists

Current repository state is backend + docs only.  
Without an iOS app target, App Store/TestFlight execution work is blocked.

## Immediate Goal

Create a minimal but real iOS app that can:

1. authenticate (temporary MVP mode acceptable),
2. select a vehicle id,
3. read KPI endpoints,
4. read rankings endpoint,
5. show loading/empty/error states.

## Phase 1: Project Foundation (Blocker)

- [ ] Create iOS app project (`SwiftUI`) in this repo.
- [ ] Define bundle id, signing setup, and target iOS version.
- [ ] Add environment config for backend base URL.
- [ ] Add typed API client module for existing contracts in `/Users/albinocordeiro/Code/car_ranks/docs/contracts/`.

## Phase 2: MVP Core Screens (Blocker)

- [ ] Auth entry screen (MVP-safe approach; can be temporary while production auth is finalized).
- [ ] Vehicle selection/input screen.
- [ ] KPI screens:
  - [ ] range/efficiency
  - [ ] charging
  - [ ] readiness
  - [ ] temperature impact
- [ ] Rankings screen.

## Phase 3: Reliability UX (Blocker)

- [ ] Standard loading state on all core screens.
- [ ] Deterministic empty state when backend returns no data.
- [ ] Error state with retry action.
- [ ] Basic telemetry/logging for request failures.

## Phase 4: Release Prep (Unblocked After Phases 1-3)

- [ ] Capture App Store screenshots from real app screens.
- [ ] Run TestFlight QA checklist.
- [ ] Final App Store submission pass.

## Suggested Execution Order (Next 3 Steps)

1. Scaffold iOS project and commit baseline.
2. Implement API client + models for `GET /v1/kpis/*` and `GET /v1/rankings`.
3. Ship first vertical slice: vehicle input -> KPI read -> render states.
