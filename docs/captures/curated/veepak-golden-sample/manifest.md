# Curated Fixture Manifest: veepak-golden-sample

- Source: representative VeePeak BLE run condensed into deterministic command replay entries.
- Purpose:
  - drive offline parser regression tests,
  - drive offline capture coordinator replay tests,
  - preserve fallback + timeout + malformed-response edge cases without another car run.
- Primary fixture file: `ios-command-fixture.json`

## Included scenarios

1. Adapter bootstrap command sequence (`ATI`, `ATZ`, `ATE0`, `ATL0`, `ATS0`, `ATH0`, `ATSP0`, `ATAT1`, `ATAL`, `ATSP6`, `0100`, `0140`, `ATDP`).
2. Successful speed sample (`010D`).
3. Voltage fallback chain (`0142 -> ATRV`).
4. Ambient temperature success (`0146`).
5. Diagnostic reads (`0101`, `03`).
6. Transport timeout on speed command (`010D`, `status=error`).
7. Malformed speed payload (`010D`, `resp=41 0D GG`).

## Notes

- This fixture is intentionally small and deterministic.
- It is safe to commit and use in CI.
