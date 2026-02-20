CREATE TABLE IF NOT EXISTS app_user (
  user_id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_vehicle_access (
  user_id TEXT NOT NULL REFERENCES app_user(user_id) ON DELETE CASCADE,
  vehicle_uid TEXT NOT NULL REFERENCES vehicle(vehicle_uid) ON DELETE CASCADE,
  access_role TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (user_id, vehicle_uid)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_vehicle_access_vehicle_uid
  ON user_vehicle_access (vehicle_uid);

CREATE INDEX IF NOT EXISTS idx_user_vehicle_access_user
  ON user_vehicle_access (user_id);
