# Curated Fixture Manifest: veepak-live-20260224

- Source: iPhone-exported run-pack from live VeePeak session `8198ea51-1ec3-46fe-8ea0-999b5e576f9a`.
- Purpose:
  - preserve full local command exchange timeline (including bootstrap and diagnostics),
  - drive replay tests directly from iOS-exported run-pack format,
  - provide deterministic fixture for offline parser/capture iteration without another EV run.
- Primary fixture file: `8198ea51-1ec3-46fe-8ea0-999b5e576f9a.json`

## Included scenarios

1. Adapter bootstrap sequence (`ATI`, `ATZ`, `ATE0`, `ATL0`, `ATS0`, `ATH0`, `ATSP0`, `ATAT1`, `ATAL`, `ATSP6`, `0100`, `0140`, `ATDP`).
2. Repeated signal polling (`010D`, `0142`, `0146`) across capture window.
3. Fallback flow for voltage (`0142` unavailable then `ATRV` success).
4. Diagnostic checks (`0101`, `03`).
5. Accepted upload receipt metadata with `batch_id` and `ingest_id`.

## Notes

- This fixture is captured from device export and intentionally retains raw response artifacts.
- Replay tests should load this file directly via `RunPackReplayFixture`.
