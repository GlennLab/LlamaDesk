CREATE TABLE IF NOT EXISTS server_samples (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp_ms INTEGER NOT NULL,
  prompt_tokens_per_sec REAL,
  generation_tokens_per_sec REAL,
  active_requests REAL,
  deferred_requests REAL,
  kv_cache_usage_ratio REAL,
  prompt_tokens_total REAL,
  tokens_predicted_total REAL
);
CREATE INDEX IF NOT EXISTS idx_server_samples_ts ON server_samples(timestamp_ms);

CREATE TABLE IF NOT EXISTS gpu_samples (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp_ms INTEGER NOT NULL,
  gpu_index INTEGER NOT NULL,
  utilization_percent REAL,
  memory_used_mib INTEGER,
  memory_total_mib INTEGER,
  temperature_c REAL,
  power_draw_w REAL
);
CREATE INDEX IF NOT EXISTS idx_gpu_samples_ts ON gpu_samples(timestamp_ms);
