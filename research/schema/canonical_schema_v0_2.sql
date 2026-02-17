-- Canonical Vehicle Data Schema v0.2
-- Target: PostgreSQL 14+
-- Notes:
-- 1) Uses gen_random_uuid() from pgcrypto.
-- 2) v0.2 is EV + temperature focused for OBD MVP contracts.
-- 3) Privacy model stores salted VIN hash only (no raw VIN persistence).

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ----------------------------
-- Core vehicle identity table
-- ----------------------------
CREATE TABLE IF NOT EXISTS vehicle (
  vehicle_uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_account_id TEXT NOT NULL,

  vin_hash TEXT,
  make TEXT,
  model TEXT,
  trim TEXT,
  model_year INTEGER,

  powertrain_class TEXT NOT NULL DEFAULT 'unknown'
    CHECK (powertrain_class IN ('bev', 'phev_electric_mode', 'unknown')),

  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  CHECK (model_year IS NULL OR model_year BETWEEN 1886 AND 2100)
);

CREATE INDEX IF NOT EXISTS idx_vehicle_source_account_id
  ON vehicle (source_account_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_vehicle_vin_hash
  ON vehicle (vin_hash)
  WHERE vin_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_vehicle_make_model_year
  ON vehicle (make, model, model_year);

-- -----------------------------------
-- Canonical signal observation stream
-- -----------------------------------
CREATE TABLE IF NOT EXISTS vehicle_signal_observation (
  observation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  vehicle_uid UUID NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,

  batch_id UUID,
  session_id UUID,

  signal_key TEXT NOT NULL,

  value_number DOUBLE PRECISION,
  value_string TEXT,
  value_bool BOOLEAN,
  value_json JSONB,
  unit TEXT,

  observed_at TIMESTAMPTZ NOT NULL,
  ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  source TEXT NOT NULL
    CHECK (source IN ('OBD', 'SMARTCAR')),
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

  temperature_bin TEXT
    CHECK (temperature_bin IS NULL OR temperature_bin IN (
      'very_cold',
      'cold',
      'cool',
      'mild',
      'hot'
    )),

  is_temperature_estimated BOOLEAN NOT NULL DEFAULT FALSE,

  raw_payload_ref TEXT
);

CREATE INDEX IF NOT EXISTS idx_obs_vehicle_signal_observed_at
  ON vehicle_signal_observation (vehicle_uid, signal_key, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_obs_batch_id
  ON vehicle_signal_observation (batch_id)
  WHERE batch_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_obs_session_id
  ON vehicle_signal_observation (session_id)
  WHERE session_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_obs_temperature_bin
  ON vehicle_signal_observation (temperature_bin)
  WHERE temperature_bin IS NOT NULL;

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

  batch_id UUID,
  session_id UUID,

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

  observed_at TIMESTAMPTZ NOT NULL,
  ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  source TEXT NOT NULL
    CHECK (source IN ('OBD', 'SMARTCAR')),
  source_event TEXT,
  resolution_hint TEXT
);

CREATE INDEX IF NOT EXISTS idx_diag_vehicle_observed_at
  ON vehicle_diagnostic_event (vehicle_uid, observed_at DESC);

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
    CHECK (source IN ('OBD', 'SMARTCAR')),

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

-- -------------------------
-- Session event timeline
-- -------------------------
CREATE TABLE IF NOT EXISTS vehicle_session_event (
  session_event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  vehicle_uid UUID NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,

  session_id UUID NOT NULL,

  session_type TEXT NOT NULL
    CHECK (session_type IN ('drive', 'charging')),

  event_type TEXT NOT NULL
    CHECK (event_type IN ('start', 'stop')),

  observed_at TIMESTAMPTZ NOT NULL,
  ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  source TEXT NOT NULL
    CHECK (source IN ('OBD', 'SMARTCAR')),

  raw_payload_ref TEXT,

  UNIQUE (vehicle_uid, session_id, session_type, event_type, observed_at)
);

CREATE INDEX IF NOT EXISTS idx_session_event_vehicle_observed_at
  ON vehicle_session_event (vehicle_uid, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_session_event_session_id
  ON vehicle_session_event (session_id, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_session_event_session_type
  ON vehicle_session_event (session_type, observed_at DESC);

-- --------------------------
-- Derived charging sessions
-- --------------------------
CREATE TABLE IF NOT EXISTS vehicle_charging_session (
  charging_session_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  vehicle_uid UUID NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,

  session_id UUID NOT NULL UNIQUE,

  started_at TIMESTAMPTZ NOT NULL,
  ended_at TIMESTAMPTZ,

  status TEXT NOT NULL DEFAULT 'partial'
    CHECK (status IN ('partial', 'complete', 'invalid')),

  charger_type TEXT NOT NULL DEFAULT 'unknown'
    CHECK (charger_type IN ('ac', 'dc', 'unknown')),

  soc_start_pct NUMERIC(5,2)
    CHECK (soc_start_pct IS NULL OR (soc_start_pct >= 0 AND soc_start_pct <= 100)),
  soc_end_pct NUMERIC(5,2)
    CHECK (soc_end_pct IS NULL OR (soc_end_pct >= 0 AND soc_end_pct <= 100)),
  soc_delta_pct NUMERIC(5,2),

  energy_added_kwh NUMERIC(10,3),
  avg_charge_power_kw NUMERIC(10,3),
  peak_charge_power_kw NUMERIC(10,3),

  ambient_temp_avg_c NUMERIC(6,2),
  battery_temp_avg_c NUMERIC(6,2),

  temperature_bin TEXT
    CHECK (temperature_bin IS NULL OR temperature_bin IN (
      'very_cold',
      'cold',
      'cool',
      'mild',
      'hot'
    )),

  temperature_is_estimated BOOLEAN NOT NULL DEFAULT FALSE,

  sample_count INTEGER NOT NULL DEFAULT 0
    CHECK (sample_count >= 0),

  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  CHECK (ended_at IS NULL OR ended_at >= started_at)
);

CREATE INDEX IF NOT EXISTS idx_charge_session_vehicle_started_at
  ON vehicle_charging_session (vehicle_uid, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_charge_session_status
  ON vehicle_charging_session (status, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_charge_session_temperature_bin
  ON vehicle_charging_session (temperature_bin)
  WHERE temperature_bin IS NOT NULL;

-- --------------------------
-- Vehicle KPI snapshots
-- --------------------------
CREATE TABLE IF NOT EXISTS vehicle_kpi_snapshot (
  snapshot_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  vehicle_uid UUID NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,

  ranking_type TEXT NOT NULL
    CHECK (ranking_type IN (
      'ev_range_efficiency',
      'ev_charging_performance',
      'ev_composite',
      'ev_temperature_impact'
    )),

  timeframe TEXT NOT NULL
    CHECK (timeframe IN ('7d', '30d', '90d', '180d')),

  kpi_key TEXT NOT NULL,
  kpi_value NUMERIC(12,4) NOT NULL,
  kpi_unit TEXT,

  direction TEXT NOT NULL
    CHECK (direction IN ('higher_is_better', 'lower_is_better')),

  confidence_level TEXT NOT NULL
    CHECK (confidence_level IN ('preview', 'medium', 'stable')),

  sample_count INTEGER NOT NULL DEFAULT 0
    CHECK (sample_count >= 0),

  temperature_bin TEXT NOT NULL DEFAULT 'all'
    CHECK (temperature_bin IN ('all', 'very_cold', 'cold', 'cool', 'mild', 'hot')),

  baseline_temperature_bin TEXT
    CHECK (baseline_temperature_bin IS NULL OR baseline_temperature_bin IN (
      'very_cold',
      'cold',
      'cool',
      'mild',
      'hot'
    )),

  compare_temperature_bin TEXT
    CHECK (compare_temperature_bin IS NULL OR compare_temperature_bin IN (
      'very_cold',
      'cold',
      'cool',
      'mild',
      'hot'
    )),

  computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  valid_from TIMESTAMPTZ,
  valid_to TIMESTAMPTZ,

  source_job_id TEXT,

  CHECK (
    (baseline_temperature_bin IS NULL AND compare_temperature_bin IS NULL)
    OR
    (baseline_temperature_bin IS NOT NULL AND compare_temperature_bin IS NOT NULL)
  ),
  CHECK (baseline_temperature_bin IS NULL OR baseline_temperature_bin <> compare_temperature_bin)
);

CREATE INDEX IF NOT EXISTS idx_vehicle_kpi_vehicle_time
  ON vehicle_kpi_snapshot (vehicle_uid, timeframe, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_vehicle_kpi_key
  ON vehicle_kpi_snapshot (kpi_key, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_vehicle_kpi_ranking_temp
  ON vehicle_kpi_snapshot (ranking_type, temperature_bin, computed_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_vehicle_kpi_uniqueness
  ON vehicle_kpi_snapshot (
    vehicle_uid,
    ranking_type,
    timeframe,
    temperature_bin,
    kpi_key,
    computed_at
  );

-- ---------------------------------
-- Cohort ranking snapshot materialization
-- ---------------------------------
CREATE TABLE IF NOT EXISTS cohort_ranking_snapshot (
  ranking_snapshot_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  ranking_type TEXT NOT NULL
    CHECK (ranking_type IN (
      'ev_range_efficiency',
      'ev_charging_performance',
      'ev_composite',
      'ev_temperature_impact'
    )),

  timeframe TEXT NOT NULL
    CHECK (timeframe IN ('7d', '30d', '90d', '180d')),

  temperature_bin TEXT NOT NULL DEFAULT 'all'
    CHECK (temperature_bin IN ('all', 'very_cold', 'cold', 'cool', 'mild', 'hot')),

  cohort_key TEXT NOT NULL,
  cohort_size INTEGER NOT NULL CHECK (cohort_size >= 0),
  sample_gate_passed BOOLEAN NOT NULL DEFAULT TRUE,

  vehicle_uid UUID NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  rank_position INTEGER NOT NULL CHECK (rank_position > 0),
  score NUMERIC(12,4) NOT NULL,

  confidence_level TEXT NOT NULL
    CHECK (confidence_level IN ('preview', 'medium', 'stable')),

  computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cohort_rank_key_time
  ON cohort_ranking_snapshot (cohort_key, ranking_type, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_cohort_rank_vehicle
  ON cohort_ranking_snapshot (vehicle_uid, ranking_type, computed_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_cohort_rank_uniqueness
  ON cohort_ranking_snapshot (
    ranking_type,
    timeframe,
    temperature_bin,
    cohort_key,
    vehicle_uid,
    computed_at
  );

-- ---------------------------------
-- Cohort cold-vs-mild KPI snapshots
-- ---------------------------------
CREATE TABLE IF NOT EXISTS cohort_temperature_metric_snapshot (
  temperature_snapshot_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  cohort_key TEXT NOT NULL,
  powertrain_class TEXT NOT NULL
    CHECK (powertrain_class IN ('bev', 'phev_electric_mode', 'unknown')),

  make TEXT,
  model TEXT,
  trim TEXT,
  year_band TEXT,

  metric_key TEXT NOT NULL
    CHECK (metric_key IN (
      'cold_weather_range_retention',
      'range_temperature_sensitivity_index',
      'cold_weather_charge_speed_retention'
    )),

  baseline_temperature_bin TEXT NOT NULL
    CHECK (baseline_temperature_bin IN ('cool', 'mild', 'hot')),

  compare_temperature_bin TEXT NOT NULL
    CHECK (compare_temperature_bin IN ('cold', 'very_cold')),

  metric_value NUMERIC(12,4) NOT NULL,

  baseline_sample_count INTEGER NOT NULL DEFAULT 0 CHECK (baseline_sample_count >= 0),
  compare_sample_count INTEGER NOT NULL DEFAULT 0 CHECK (compare_sample_count >= 0),

  confidence_level TEXT NOT NULL
    CHECK (confidence_level IN ('preview', 'medium', 'stable')),

  computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_temp_metric_cohort_time
  ON cohort_temperature_metric_snapshot (cohort_key, metric_key, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_temp_metric_bins
  ON cohort_temperature_metric_snapshot (baseline_temperature_bin, compare_temperature_bin, computed_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_temp_metric_uniqueness
  ON cohort_temperature_metric_snapshot (
    cohort_key,
    metric_key,
    baseline_temperature_bin,
    compare_temperature_bin,
    computed_at
  );

-- -------------------------------
-- updated_at helper
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

DROP TRIGGER IF EXISTS trg_vehicle_charging_session_set_updated_at ON vehicle_charging_session;

CREATE TRIGGER trg_vehicle_charging_session_set_updated_at
BEFORE UPDATE ON vehicle_charging_session
FOR EACH ROW
EXECUTE FUNCTION set_updated_at_timestamp();
