# Competitor Table (OBD + Connected-Car)

Date: 2026-02-17  
Purpose: pricing and product-shape snapshot to inform technical/product decisions while white-label decisions are deferred.

| Product | Model | Upfront cost (USD) | Ongoing cost | Data/access angle | Notes | Sources |
|---|---|---:|---:|---|---|---|
| BlueDriver OBDII Scan Tool | Hardware + companion app | $84.95 (page also shows $119.95 compare-at) | None claimed (`No subscriptions. No add-ons. Ever.`) | OBD-II scan tool + mobile diagnostics | One-time hardware purchase positioning | `research/sources/competitors/bluedriver-scan-tool.html:1632`, `research/sources/competitors/bluedriver-scan-tool.html:2696` |
| FIXD Premium trial + sensor | Hardware + subscription | $19.99 intro for 14 days, includes free sensor | $69.99/year after trial (captured) | OBD + premium support/features | Promo landing page; pricing copy is campaign-style | `research/sources/competitors/fixd-trial-offer.pretty.html:21`, `research/sources/competitors/fixd-trial-offer.pretty.html:2681`, `research/sources/competitors/fixd-trial-offer.pretty.html:3003` |
| Carly OBD shopping page | Hardware + app/service | $89.80 appears in metadata and page blocks; $79.90 also appears | Not explicit in captured snippets | OBD scanner positioning | Mixed price points on same captured page; confirm current checkout price before decisions | `research/sources/competitors/carly-shopping.pretty.html:107`, `research/sources/competitors/carly-shopping.pretty.html:352`, `research/sources/competitors/carly-shopping.pretty.html:421` |
| Bouncie pricing page | Device + required subscription | $89.99 device (JSON-LD offer) | Subscription required; page shows `9` + `65` (monthly) and `8.35` for 3+ devices | Always-on connected vehicle telematics style | Monthly `9.65` is reconstructed from split markup; verify checkout totals | `research/sources/competitors/bouncie-pricing.pretty.html:269`, `research/sources/competitors/bouncie-pricing.pretty.html:279`, `research/sources/competitors/bouncie-pricing.pretty.html:288`, `research/sources/competitors/bouncie-pricing.pretty.html:290`, `research/sources/competitors/bouncie-pricing.pretty.html:794` |
| OBD Fusion (iOS App Store) | App-only (adapter required) | $9.99 | Not shown | App supports reading OBD2 data, DTC clearing, dashboards; requires compatible ELM327 Wi-Fi/BLE class adapters | Useful baseline for app-only pricing and adapter compatibility messaging | `research/sources/competitors/itunes-obd-fusion.pretty.json:176`, `research/sources/competitors/itunes-obd-fusion.pretty.json:198`, `research/sources/competitors/itunes-obd-fusion.pretty.json:209` |

## Quick readout
- Consumer market patterns in this sample:
  - one-time hardware (`BlueDriver`)
  - hardware + subscription (`FIXD`, `Bouncie`)
  - low-ticket app entry price (`OBD Fusion`).
- For your roadmap, this supports testing two monetization paths in parallel:
  - pure software (bring-your-own adapter)
  - managed experience (hardware + subscription), decided after technical feasibility/ranking value is proven.
