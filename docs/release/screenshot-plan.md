# Screenshot Plan (Draft)

Status: Draft v1  
Date: 2026-02-20  
Scope: App Store screenshot capture plan for MVP core flows.

## 1. Device-Class Matrix

Confirm exact required classes in App Store Connect for this binary before capture.

- [ ] iPhone large class set A (for example: 6.9-inch)
- [ ] iPhone large class set B (for example: 6.5-inch)
- [ ] iPad classes (only if this binary targets iPad)

## 2. Required Screenshot Storyline

1. Sign-in screen (production branding)
2. Vehicle selection/link screen
3. KPI range/efficiency view
4. KPI charging performance view
5. KPI readiness view (including missing-requirement state)
6. Temperature-impact KPI view (cold vs mild comparison)
7. Rankings view (EV ranking list)
8. Empty/loading/error state example

## 3. Capture Rules

- Use consistent, release-quality data (no placeholder lorem text).
- Ensure all text is legible on device-scale screenshots.
- Avoid debug overlays, test banners, and internal build labels.
- Keep timestamps/telemetry realistic and internally consistent across shots.
- Verify no sensitive personal information appears in screenshots.

## 4. Shot Checklist Template

| Shot | Screen | Data State | Device Class | File Name | Owner | Status |
|---|---|---|---|---|---|---|
| 01 | Sign-in | Ready | TODO | TODO | Solo | TODO |
| 02 | Vehicle Link | Linked vehicle present | TODO | TODO | Solo | TODO |
| 03 | KPI Range | Data loaded | TODO | TODO | Solo | TODO |
| 04 | KPI Charging | Data loaded | TODO | TODO | Solo | TODO |
| 05 | KPI Readiness | Partial data with gates | TODO | TODO | Solo | TODO |
| 06 | Temperature Impact | Cold vs mild | TODO | TODO | Solo | TODO |
| 07 | Rankings | Ranked rows present | TODO | TODO | Solo | TODO |
| 08 | Error/Empty | Controlled fallback | TODO | TODO | Solo | TODO |

## 5. Finalization Checklist

- [ ] All required device classes captured.
- [ ] Screenshots uploaded to App Store Connect and ordered.
- [ ] Screenshots reviewed for consistency and policy compliance.
