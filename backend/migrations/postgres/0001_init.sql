
CREATE TABLE IF NOT EXISTS vehicle (
  vehicle_uid TEXT PRIMARY KEY,
  source_account_id TEXT NOT NULL,
  vin_hash TEXT UNIQUE,
  make TEXT,
  model TEXT,
  trim TEXT,
  model_year INTEGER,
  powertrain_class TEXT NOT NULL DEFAULT 'bev',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ingest_batch (
  batch_id TEXT PRIMARY KEY,
  vehicle_uid TEXT NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  schema_version TEXT NOT NULL,
  source TEXT NOT NULL,
  capture_started_at TEXT NOT NULL,
  capture_ended_at TEXT NOT NULL,
  received_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS vehicle_signal_observation (
  observation_id TEXT PRIMARY KEY,
  vehicle_uid TEXT NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  batch_id TEXT REFERENCES ingest_batch(batch_id) ON DELETE SET NULL,
  session_id TEXT,
  signal_key TEXT NOT NULL,
  value_number REAL,
  value_string TEXT,
  value_bool INTEGER,
  value_json TEXT,
  unit TEXT,
  observed_at TEXT NOT NULL,
  ingested_at TEXT NOT NULL,
  source TEXT NOT NULL,
  source_signal TEXT,
  status TEXT NOT NULL,
  confidence REAL,
  freshness_ttl_seconds INTEGER,
  temperature_bin TEXT,
  is_temperature_estimated INTEGER NOT NULL DEFAULT 0,
  raw_payload_ref TEXT
);

CREATE INDEX IF NOT EXISTS idx_obs_vehicle_signal_observed_at
  ON vehicle_signal_observation (vehicle_uid, signal_key, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_obs_vehicle_observed_at
  ON vehicle_signal_observation (vehicle_uid, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_obs_batch_id
  ON vehicle_signal_observation (batch_id);

CREATE INDEX IF NOT EXISTS idx_obs_temperature_bin
  ON vehicle_signal_observation (temperature_bin);

CREATE TABLE IF NOT EXISTS vehicle_diagnostic_event (
  event_id TEXT PRIMARY KEY,
  vehicle_uid TEXT NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  batch_id TEXT REFERENCES ingest_batch(batch_id) ON DELETE SET NULL,
  session_id TEXT,
  event_type TEXT NOT NULL,
  code TEXT,
  severity TEXT,
  description TEXT,
  observed_at TEXT NOT NULL,
  ingested_at TEXT NOT NULL,
  source TEXT NOT NULL,
  source_event TEXT,
  resolution_hint TEXT
);

CREATE INDEX IF NOT EXISTS idx_diag_vehicle_observed_at
  ON vehicle_diagnostic_event (vehicle_uid, observed_at DESC);

CREATE TABLE IF NOT EXISTS vehicle_session_event (
  session_event_id TEXT PRIMARY KEY,
  vehicle_uid TEXT NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  session_id TEXT NOT NULL,
  session_type TEXT NOT NULL,
  event_type TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  ingested_at TEXT NOT NULL,
  source TEXT NOT NULL,
  raw_payload_ref TEXT,
  UNIQUE (vehicle_uid, session_id, session_type, event_type, observed_at)
);

CREATE INDEX IF NOT EXISTS idx_session_event_vehicle_time
  ON vehicle_session_event (vehicle_uid, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_session_event_session
  ON vehicle_session_event (session_id, observed_at DESC);

CREATE TABLE IF NOT EXISTS vehicle_charging_session (
  charging_session_id TEXT PRIMARY KEY,
  vehicle_uid TEXT NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  session_id TEXT NOT NULL UNIQUE,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  status TEXT NOT NULL,
  charger_type TEXT NOT NULL,
  soc_start_pct REAL,
  soc_end_pct REAL,
  soc_delta_pct REAL,
  energy_added_kwh REAL,
  avg_charge_power_kw REAL,
  peak_charge_power_kw REAL,
  ambient_temp_avg_c REAL,
  battery_temp_avg_c REAL,
  temperature_bin TEXT,
  temperature_is_estimated INTEGER NOT NULL DEFAULT 0,
  sample_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_charge_session_vehicle_started_at
  ON vehicle_charging_session (vehicle_uid, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_charge_session_temperature_bin
  ON vehicle_charging_session (temperature_bin);

CREATE TABLE IF NOT EXISTS vehicle_kpi_snapshot (
  snapshot_id TEXT PRIMARY KEY,
  vehicle_uid TEXT NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  ranking_type TEXT NOT NULL,
  timeframe TEXT NOT NULL,
  kpi_key TEXT NOT NULL,
  kpi_value REAL NOT NULL,
  kpi_unit TEXT,
  direction TEXT NOT NULL,
  confidence_level TEXT NOT NULL,
  sample_count INTEGER NOT NULL DEFAULT 0,
  temperature_bin TEXT NOT NULL DEFAULT 'all',
  baseline_temperature_bin TEXT,
  compare_temperature_bin TEXT,
  computed_at TEXT NOT NULL,
  valid_from TEXT,
  valid_to TEXT,
  source_job_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_vehicle_kpi_vehicle_time
  ON vehicle_kpi_snapshot (vehicle_uid, timeframe, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_vehicle_kpi_key
  ON vehicle_kpi_snapshot (kpi_key, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_vehicle_kpi_rank_temp
  ON vehicle_kpi_snapshot (ranking_type, timeframe, temperature_bin, computed_at DESC);

CREATE TABLE IF NOT EXISTS cohort_ranking_snapshot (
  ranking_snapshot_id TEXT PRIMARY KEY,
  ranking_type TEXT NOT NULL,
  timeframe TEXT NOT NULL,
  temperature_bin TEXT NOT NULL DEFAULT 'all',
  cohort_key TEXT NOT NULL,
  cohort_size INTEGER NOT NULL,
  sample_gate_passed INTEGER NOT NULL DEFAULT 1,
  vehicle_uid TEXT NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  rank_position INTEGER NOT NULL,
  score REAL NOT NULL,
  confidence_level TEXT NOT NULL,
  computed_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cohort_rank_key_time
  ON cohort_ranking_snapshot (cohort_key, ranking_type, timeframe, temperature_bin, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_cohort_rank_vehicle
  ON cohort_ranking_snapshot (vehicle_uid, ranking_type, timeframe, computed_at DESC);
