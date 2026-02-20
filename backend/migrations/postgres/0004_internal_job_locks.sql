CREATE TABLE IF NOT EXISTS internal_job_lock (
  job_kind TEXT PRIMARY KEY,
  owner_token TEXT NOT NULL,
  acquired_at TEXT NOT NULL,
  expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_internal_job_lock_expires_at
  ON internal_job_lock (expires_at);
