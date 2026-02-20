CREATE TABLE IF NOT EXISTS internal_job_run (
  job_run_id TEXT PRIMARY KEY,
  job_kind TEXT NOT NULL,
  backend TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  error_message TEXT,
  response_job_id TEXT,
  charging_sessions_upserted INTEGER,
  kpi_rows_upserted INTEGER,
  ranking_rows_upserted INTEGER,
  recomputed_vehicles INTEGER
);

CREATE INDEX IF NOT EXISTS idx_internal_job_run_kind_started
  ON internal_job_run (job_kind, started_at DESC);
