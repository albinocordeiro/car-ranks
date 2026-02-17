# Initial Findings (OBD-II + Car KPI Rankings App)

Status: draft notes based on early desk research. Treat as preliminary until sources are verified.

## Problem Framing
- App reads OBD-II telemetry, computes higher-level KPIs, and ranks cars by make/model/trim.
- Use cases: car shoppers (real-world comparisons) and owners (benchmarking their vehicle).
- Potential differentiator: community + gamified challenges tied to vehicle performance.

## Existing Players (Preliminary)
- Diagnostic bundles that pair a branded dongle with a paid app and analytics (bundled hardware + license model exists).
- Enthusiast tools that focus on coding/customization and model-specific features (hardware lock-in is accepted for added value).
- General-purpose OBD apps that visualize sensor data but do not aggregate fleet KPIs.
- API-based vehicle data access providers (for OEM telematics on newer cars).
- Fleet gamification platforms that reward safer or more efficient driving (proves engagement mechanics).

## White-Label Hardware (Preliminary)
- OEM/ODM OBD-II suppliers exist that offer branding, firmware customization, and bulk pricing.
- Some vendors provide SDKs to embed hardware support in third-party apps.
- Cellular OBD devices exist (device can transmit without phone), which could expand use cases.

## Gaps / Opportunities
- Few consumer products appear to rank real-world performance across models using aggregated OBD metrics.
- Gamification is underused for consumer car communities compared to fleets.
- Dual ingestion (OBD + OEM API) could reduce friction, especially for newer vehicles.

## Risks / Unknowns
- Data normalization across vehicle years/engines/trim levels.
- Privacy and consent for aggregated rankings.
- Regulatory considerations (right-to-repair, OEM access policies, regional differences).
- Data quality variance across OBD dongles.

## Next Research Tasks
- Validate and cite key competitors and their positioning.
- Identify top 3-5 white-label hardware vendors with pricing, MOQ, certifications.
- Define a minimal KPI set and map to required PIDs/events.
- Sketch pricing strategy (hardware margin vs subscription value).

