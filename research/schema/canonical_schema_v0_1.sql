-- Canonical Vehicle Data Schema v0.1
-- Target: PostgreSQL 14+
-- Notes:
-- 1) Uses gen_random_uuid() from pgcrypto.
-- 2) Uses CHECK constraints instead of custom enum types for portability.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ----------------------------
-- Core vehicle identity table
-- ----------------------------
CREATE TABLE IF NOT EXISTS vehicle (
  vehicle_uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_account_id TEXT NOT NULL,
  vin TEXT,
  make TEXT,
  model TEXT,
  model_year INTEGER,
  powertrain TEXT NOT NULL DEFAULT 'UNKNOWN'
    CHECK (powertrain IN ('ICE', 'HEV', 'PHEV', 'BEV', 'UNKNOWN')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (model_year IS NULL OR model_year BETWEEN 1886 AND 2100)
);

CREATE INDEX IF NOT EXISTS idx_vehicle_source_account_id
  ON vehicle (source_account_id);

CREATE INDEX IF NOT EXISTS idx_vehicle_vin
  ON vehicle (vin)
  WHERE vin IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_vehicle_make_model_year
  ON vehicle (make, model, model_year);

-- -----------------------------------
-- Canonical signal observation stream
-- -----------------------------------
CREATE TABLE IF NOT EXISTS vehicle_signal_observation (
  observation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  vehicle_uid UUID NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,

  signal_key TEXT NOT NULL,

  value_number DOUBLE PRECISION,
  value_string TEXT,
  value_bool BOOLEAN,
  unit TEXT,

  observed_at TIMESTAMPTZ,
  ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  source TEXT NOT NULL
    CHECK (source IN ('SMARTCAR', 'OBD')),
  source_signal TEXT,

  status TEXT NOT NULL
    CHECK (status IN (
      'ok',
      'stale',
      'unavailable',
      'not_supported',
      'permission_denied',
      'error'
    )),

  confidence NUMERIC(3,2)
    CHECK (confidence IS NULL OR (confidence >= 0.00 AND confidence <= 1.00)),

  freshness_ttl_seconds INTEGER
    CHECK (freshness_ttl_seconds IS NULL OR freshness_ttl_seconds >= 0),

  raw_payload_ref TEXT
);

CREATE INDEX IF NOT EXISTS idx_obs_vehicle_signal_observed_at
  ON vehicle_signal_observation (vehicle_uid, signal_key, observed_at DESC NULLS LAST);

CREATE INDEX IF NOT EXISTS idx_obs_ingested_at
  ON vehicle_signal_observation (ingested_at DESC);

CREATE INDEX IF NOT EXISTS idx_obs_source_signal
  ON vehicle_signal_observation (source, source_signal);

CREATE INDEX IF NOT EXISTS idx_obs_status
  ON vehicle_signal_observation (status);

-- -------------------------
-- Diagnostic event timeline
-- -------------------------
CREATE TABLE IF NOT EXISTS vehicle_diagnostic_event (
  event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  vehicle_uid UUID NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,

  event_type TEXT NOT NULL
    CHECK (event_type IN (
      'DTC_ACTIVE',
      'DTC_CLEARED',
      'MIL_ON',
      'MIL_OFF',
      'READINESS_CHANGED',
      'SOURCE_ERROR'
    )),

  code TEXT,
  severity TEXT,
  description TEXT,

  observed_at TIMESTAMPTZ,
  ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  source TEXT NOT NULL
    CHECK (source IN ('SMARTCAR', 'OBD')),
  source_event TEXT,
  resolution_hint TEXT
);

CREATE INDEX IF NOT EXISTS idx_diag_vehicle_observed_at
  ON vehicle_diagnostic_event (vehicle_uid, observed_at DESC NULLS LAST);

CREATE INDEX IF NOT EXISTS idx_diag_event_type
  ON vehicle_diagnostic_event (event_type);

CREATE INDEX IF NOT EXISTS idx_diag_code
  ON vehicle_diagnostic_event (code)
  WHERE code IS NOT NULL;

-- -------------------
-- Capability snapshot
-- -------------------
CREATE TABLE IF NOT EXISTS vehicle_capability (
  vehicle_uid UUID NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  capability_key TEXT NOT NULL,

  source TEXT NOT NULL
    CHECK (source IN ('SMARTCAR', 'OBD')),

  status TEXT NOT NULL
    CHECK (status IN ('supported', 'not_supported', 'unknown', 'permission_denied')),

  last_checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  check_method TEXT NOT NULL
    CHECK (check_method IN ('compatibility_api', 'connect_grant', 'runtime_probe', 'manual')),

  PRIMARY KEY (vehicle_uid, capability_key, source)
);

CREATE INDEX IF NOT EXISTS idx_capability_status
  ON vehicle_capability (status);

CREATE INDEX IF NOT EXISTS idx_capability_last_checked_at
  ON vehicle_capability (last_checked_at DESC);

-- -------------------------------
-- updated_at helper for vehicle
-- -------------------------------
CREATE OR REPLACE FUNCTION set_updated_at_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_vehicle_set_updated_at ON vehicle;

CREATE TRIGGER trg_vehicle_set_updated_at
BEFORE UPDATE ON vehicle
FOR EACH ROW
EXECUTE FUNCTION set_updated_at_timestamp();
