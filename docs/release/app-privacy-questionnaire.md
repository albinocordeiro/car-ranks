# App Privacy Questionnaire (Draft)

Status: Draft v1  
Date: 2026-02-20  
Scope: initial mapping from MVP behavior to App Privacy form inputs in App Store Connect.

This is a preparation draft, not legal advice. Final answers must match the shipped binary and privacy policy.

## 1. Data Collection Mapping (Draft)

| Data Type | Collected | Linked to User | Used for Tracking | Purpose (Draft) | Notes |
|---|---|---|---|---|---|
| Contact Info (Email) | Yes (if email auth enabled) | Yes | No | Account Management | Confirm actual auth methods in production build. |
| User ID | Yes | Yes | No | App Functionality | Used for ownership-scoped KPI/ranking reads. |
| Vehicle Identifier (Pseudonymous) | Yes | Yes | No | App Functionality | Vehicle UID is used; raw VIN is not persisted in MVP docs. |
| Diagnostics Data | Yes | Yes | No | App Functionality | MIL/DTCs support readiness and health modifier context. |
| Performance Data (Telemetry) | Yes | Yes | No | App Functionality, Analytics | Includes speed/SoC/charging/temperature signals from OBD ingest. |
| Location (Precise) | No | N/A | No | N/A | MVP docs state precise GPS is out of scope. |
| Location (Coarse) | Possibly | Possibly | No | App Functionality | Confirm whether any ambient-temperature fallback uses coarse location. |
| Crash Data | TODO | TODO | No | App Functionality | Depends on crash SDK usage in iOS app. |
| Usage Data | TODO | TODO | No | Analytics | Confirm if app analytics SDK is enabled. |

## 2. Tracking Declaration

- Current draft declaration: `No tracking across apps/websites`.
- Final verification required against all integrated SDKs before submission.

## 3. Required Cross-Checks Before Finalizing

- [ ] Validate auth modes active in production configuration.
- [ ] Validate all iOS SDKs included in release build (analytics, crash, ads, attribution).
- [ ] Validate telemetry fields actually transmitted in production.
- [ ] Confirm privacy policy text matches this table.
- [ ] Confirm App Privacy answers match the same final implementation.

## 4. Evidence Links

- MVP scope: `/Users/albinocordeiro/Code/car_ranks/docs/spec.md`
- Architecture/privacy constraints: `/Users/albinocordeiro/Code/car_ranks/docs/architecture.md`
- API contracts showing telemetry and diagnostics usage: `/Users/albinocordeiro/Code/car_ranks/docs/contracts/`
