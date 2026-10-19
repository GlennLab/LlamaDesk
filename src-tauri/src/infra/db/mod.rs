//! SQLite-backed implementation of the [`HistoryStore`] port.
//!
//! Samples are written with parameterised statements (never string-concatenated
//! SQL) and read back with their nullable columns preserved as `None`, so a
//! missing metric stays distinguishable from a real zero.

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};

use crate::domain::error::MonitorError;
use crate::domain::history::{GpuSample, ServerSample};
use crate::ports::storage::HistoryStore;

/// Busy timeout applied to every connection before returning `SQLITE_BUSY`.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
/// Maximum number of pooled connections.
const MAX_CONNECTIONS: u32 = 4;
/// Hard cap on rows returned by a single range query.
///
/// Without it, a request covering the whole retention window could load the
/// entire table into memory. Rows are returned oldest-first.
const MAX_RANGE_ROWS: i64 = 20_000;

/// Error helper that prefixes a static context onto a [`sqlx::Error`].
fn storage_error(context: &str, error: sqlx::Error) -> MonitorError {
    MonitorError::Storage(format!("{context}: {error}"))
}

/// Decodes a single column from `row`, mapping failures to
/// [`MonitorError::Storage`] with the column name as context.
fn column<T>(row: &SqliteRow, name: &str) -> Result<T, MonitorError>
where
    for<'r> T: sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
{
    row.try_get(name).map_err(|error| {
        MonitorError::Storage(format!("failed to decode column `{name}`: {error}"))
    })
}

/// Maps a `server_samples` row to a [`ServerSample`].
fn map_server_row(row: SqliteRow) -> Result<ServerSample, MonitorError> {
    Ok(ServerSample {
        timestamp_ms: column(&row, "timestamp_ms")?,
        prompt_tokens_per_sec: column(&row, "prompt_tokens_per_sec")?,
        generation_tokens_per_sec: column(&row, "generation_tokens_per_sec")?,
        active_requests: column(&row, "active_requests")?,
        deferred_requests: column(&row, "deferred_requests")?,
        kv_cache_usage_ratio: column(&row, "kv_cache_usage_ratio")?,
        prompt_tokens_total: column(&row, "prompt_tokens_total")?,
        tokens_predicted_total: column(&row, "tokens_predicted_total")?,
    })
}

/// Maps a `gpu_samples` row to a [`GpuSample`].
fn map_gpu_row(row: SqliteRow) -> Result<GpuSample, MonitorError> {
    Ok(GpuSample {
        timestamp_ms: column(&row, "timestamp_ms")?,
        gpu_index: column(&row, "gpu_index")?,
        utilization_percent: column(&row, "utilization_percent")?,
        memory_used_mib: column(&row, "memory_used_mib")?,
        memory_total_mib: column(&row, "memory_total_mib")?,
          temperature_c: column(&row, "temperature_c")?,
          power_draw_w: column(&row, "power_draw_w")?,
          fan_speed_percent: column(&row, "fan_speed_rpm")?,
      })
}

/// A [`HistoryStore`] backed by a file-based SQLite database in WAL mode.
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Opens (creating if necessary) the database at `path` and runs migrations.
    ///
    /// Any missing parent directories are created first. Calling this twice on
    /// the same path is safe: the schema is created idempotently.
    pub async fn connect(path: &Path) -> Result<Self, MonitorError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| {
                MonitorError::Storage(format!(
                    "failed to create database directory {}: {error}",
                    parent.display()
                ))
            })?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(BUSY_TIMEOUT);

        let pool = SqlitePoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect_with(options)
            .await
            .map_err(|error| storage_error("failed to open database", error))?;

        sqlx::migrate!("./src/infra/db/migrations")
            .run(&pool)
            .await
            .map_err(|error| MonitorError::Storage(format!("failed to run migrations: {error}")))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl HistoryStore for SqliteStore {
    async fn insert_server_sample(&self, sample: &ServerSample) -> Result<(), MonitorError> {
        sqlx::query(
            "INSERT INTO server_samples (\
                timestamp_ms, prompt_tokens_per_sec, generation_tokens_per_sec, \
                active_requests, deferred_requests, kv_cache_usage_ratio, \
                prompt_tokens_total, tokens_predicted_total\
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(sample.timestamp_ms)
        .bind(sample.prompt_tokens_per_sec)
        .bind(sample.generation_tokens_per_sec)
        .bind(sample.active_requests)
        .bind(sample.deferred_requests)
        .bind(sample.kv_cache_usage_ratio)
        .bind(sample.prompt_tokens_total)
        .bind(sample.tokens_predicted_total)
        .execute(&self.pool)
        .await
        .map_err(|error| storage_error("failed to insert server sample", error))?;
        Ok(())
    }

    async fn insert_gpu_sample(&self, sample: &GpuSample) -> Result<(), MonitorError> {
        sqlx::query(
            "INSERT INTO gpu_samples (\
                timestamp_ms, gpu_index, utilization_percent, \
                memory_used_mib, memory_total_mib, temperature_c, power_draw_w, fan_speed_rpm\
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
          )
          .bind(sample.timestamp_ms)
          .bind(sample.gpu_index)
          .bind(sample.utilization_percent)
          // SQLite stores integers as `i64`; `u64` is not encodable, so cast the
          // MiB counts (always far below `i64::MAX`).
          .bind(sample.memory_used_mib.map(|value| value as i64))
          .bind(sample.memory_total_mib.map(|value| value as i64))
          .bind(sample.temperature_c)
          .bind(sample.power_draw_w)
          .bind(sample.fan_speed_percent)
        .execute(&self.pool)
        .await
        .map_err(|error| storage_error("failed to insert gpu sample", error))?;
        Ok(())
    }

    async fn query_server_range(
        &self,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<ServerSample>, MonitorError> {
        let rows = sqlx::query(
            "SELECT timestamp_ms, prompt_tokens_per_sec, generation_tokens_per_sec, \
                    active_requests, deferred_requests, kv_cache_usage_ratio, \
                    prompt_tokens_total, tokens_predicted_total \
             FROM server_samples \
             WHERE timestamp_ms BETWEEN ?1 AND ?2 \
             ORDER BY timestamp_ms ASC \
             LIMIT ?3",
        )
        .bind(from_ms)
        .bind(to_ms)
        .bind(MAX_RANGE_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| storage_error("failed to query server samples", error))?;

        rows.into_iter().map(map_server_row).collect()
    }

    async fn query_gpu_range(
        &self,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<GpuSample>, MonitorError> {
        let rows = sqlx::query(
            "SELECT timestamp_ms, gpu_index, utilization_percent, \
                      memory_used_mib, memory_total_mib, temperature_c, power_draw_w, fan_speed_rpm \
               FROM gpu_samples \
             WHERE timestamp_ms BETWEEN ?1 AND ?2 \
             ORDER BY timestamp_ms ASC \
             LIMIT ?3",
        )
        .bind(from_ms)
        .bind(to_ms)
        .bind(MAX_RANGE_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| storage_error("failed to query gpu samples", error))?;

        rows.into_iter().map(map_gpu_row).collect()
    }

    async fn prune(&self, retention_ms: i64) -> Result<u64, MonitorError> {
        let server = sqlx::query("DELETE FROM server_samples WHERE timestamp_ms < ?1")
            .bind(retention_ms)
            .execute(&self.pool)
            .await
            .map_err(|error| storage_error("failed to prune server samples", error))?
            .rows_affected();

        let gpu = sqlx::query("DELETE FROM gpu_samples WHERE timestamp_ms < ?1")
            .bind(retention_ms)
            .execute(&self.pool)
            .await
            .map_err(|error| storage_error("failed to prune gpu samples", error))?
            .rows_affected();

        Ok(server + gpu)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    /// Returns a unique database path inside the system temp directory whose
    /// parent does not exist yet (proving `connect` creates it).
    fn temp_db_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = format!(
            "llamadesk-db-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        std::env::temp_dir().join(unique).join("history.db")
    }

    fn server_sample(timestamp_ms: i64) -> ServerSample {
        ServerSample {
            timestamp_ms,
            prompt_tokens_per_sec: Some(12.5),
            generation_tokens_per_sec: None,
            active_requests: Some(2.0),
            deferred_requests: None,
            kv_cache_usage_ratio: Some(0.5),
            prompt_tokens_total: None,
            tokens_predicted_total: Some(100.0),
        }
    }

    fn gpu_sample(timestamp_ms: i64) -> GpuSample {
        GpuSample {
            timestamp_ms,
            gpu_index: 0,
            utilization_percent: Some(75.0),
            memory_used_mib: None,
            memory_total_mib: Some(24564),
            temperature_c: None,
            power_draw_w: Some(120.5),
            fan_speed_percent: None,
        }
    }

    #[tokio::test]
    async fn connect_creates_schema_and_parent_directory() {
        let path = temp_db_path();
        let store = SqliteStore::connect(&path).await.expect("connect");

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('server_samples', 'gpu_samples')",
        )
        .fetch_one(&store.pool)
        .await
        .expect("query sqlite_master");

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn connect_is_idempotent_on_same_path() {
        let path = temp_db_path();
        let first = SqliteStore::connect(&path).await.expect("first connect");
        let second = SqliteStore::connect(&path).await.expect("second connect");

        first
            .insert_server_sample(&server_sample(1_000))
            .await
            .expect("insert via first handle");
        let rows = second
            .query_server_range(0, 2_000)
            .await
            .expect("query via second handle");
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn server_samples_round_trip_preserving_none() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        let sample = server_sample(1_700_000_000_000);
        store.insert_server_sample(&sample).await.expect("insert");

        let rows = store.query_server_range(0, i64::MAX).await.expect("query");
        assert_eq!(rows, vec![sample]);
        assert_eq!(rows[0].generation_tokens_per_sec, None);
        assert_eq!(rows[0].deferred_requests, None);
        assert_eq!(rows[0].prompt_tokens_total, None);
    }

    #[tokio::test]
    async fn gpu_samples_round_trip_preserving_none() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        let sample = gpu_sample(1_700_000_000_000);
        store.insert_gpu_sample(&sample).await.expect("insert");

        let rows = store.query_gpu_range(0, i64::MAX).await.expect("query");
        assert_eq!(rows, vec![sample]);
        assert_eq!(rows[0].memory_used_mib, None);
        assert_eq!(rows[0].temperature_c, None);
    }

    #[tokio::test]
    async fn range_query_excludes_out_of_range_rows() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        store
            .insert_server_sample(&server_sample(1_000))
            .await
            .expect("insert old");
        store
            .insert_server_sample(&server_sample(5_000))
            .await
            .expect("insert in range");
        store
            .insert_server_sample(&server_sample(9_000))
            .await
            .expect("insert new");

        let rows = store.query_server_range(2_000, 6_000).await.expect("query");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].timestamp_ms, 5_000);
    }

    #[tokio::test]
    async fn query_returns_rows_ordered_oldest_first() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        store
            .insert_server_sample(&server_sample(9_000))
            .await
            .expect("first");
        store
            .insert_server_sample(&server_sample(1_000))
            .await
            .expect("second");

        let rows = store.query_server_range(0, i64::MAX).await.expect("query");
        let timestamps: Vec<i64> = rows.iter().map(|row| row.timestamp_ms).collect();
        assert_eq!(timestamps, vec![1_000, 9_000]);
    }

    #[tokio::test]
    async fn prune_removes_only_old_rows_and_returns_count() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        store
            .insert_server_sample(&server_sample(1_000))
            .await
            .expect("old server");
        store
            .insert_server_sample(&server_sample(5_000))
            .await
            .expect("new server");
        store
            .insert_gpu_sample(&gpu_sample(1_000))
            .await
            .expect("old gpu");
        store
            .insert_gpu_sample(&gpu_sample(5_000))
            .await
            .expect("new gpu");

        let removed = store.prune(3_000).await.expect("prune");
        assert_eq!(removed, 2);

        let remaining_server = store.query_server_range(0, i64::MAX).await.expect("query");
        let remaining_gpu = store.query_gpu_range(0, i64::MAX).await.expect("query");
        assert_eq!(remaining_server.len(), 1);
        assert_eq!(remaining_gpu.len(), 1);
        assert_eq!(remaining_server[0].timestamp_ms, 5_000);
    }

    #[tokio::test]
    async fn all_none_server_sample_round_trips_as_none() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        let sample = ServerSample {
            timestamp_ms: 1_700_000_001_000,
            prompt_tokens_per_sec: None,
            generation_tokens_per_sec: None,
            active_requests: None,
            deferred_requests: None,
            kv_cache_usage_ratio: None,
            prompt_tokens_total: None,
            tokens_predicted_total: None,
        };
        store.insert_server_sample(&sample).await.expect("insert");

        let rows = store.query_server_range(0, i64::MAX).await.expect("query");
        assert_eq!(rows, vec![sample]);
        // A missing metric must stay `None`, never be coerced to a real 0.
        assert!(rows[0].prompt_tokens_per_sec.is_none());
        assert!(rows[0].active_requests.is_none());
        assert!(rows[0].tokens_predicted_total.is_none());
    }

    #[tokio::test]
    async fn all_none_gpu_sample_round_trips_as_none() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        let sample = GpuSample {
            timestamp_ms: 1_700_000_001_000,
            gpu_index: 2,
            utilization_percent: None,
            memory_used_mib: None,
            memory_total_mib: None,
            temperature_c: None,
            power_draw_w: None,
            fan_speed_percent: None,
        };
        store.insert_gpu_sample(&sample).await.expect("insert");

        let rows = store.query_gpu_range(0, i64::MAX).await.expect("query");
        assert_eq!(rows, vec![sample]);
        assert!(rows[0].memory_used_mib.is_none());
        assert!(rows[0].power_draw_w.is_none());
    }

    #[tokio::test]
    async fn inverted_range_returns_empty() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        store
            .insert_server_sample(&server_sample(5_000))
            .await
            .expect("server");
        store
            .insert_gpu_sample(&gpu_sample(5_000))
            .await
            .expect("gpu");

        let server = store.query_server_range(6_000, 1_000).await.expect("query");
        let gpu = store.query_gpu_range(6_000, 1_000).await.expect("query");
        assert!(server.is_empty(), "from > to must yield no server rows");
        assert!(gpu.is_empty(), "from > to must yield no gpu rows");
    }

    #[tokio::test]
    async fn prune_with_nothing_older_removes_zero() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        store
            .insert_server_sample(&server_sample(5_000))
            .await
            .expect("server");
        store
            .insert_gpu_sample(&gpu_sample(5_000))
            .await
            .expect("gpu");

        let removed = store.prune(1_000).await.expect("prune");
        assert_eq!(removed, 0);

        let server = store.query_server_range(0, i64::MAX).await.expect("query");
        assert_eq!(server.len(), 1);
    }

    #[tokio::test]
    async fn duplicate_timestamps_both_persist() {
        let store = SqliteStore::connect(&temp_db_path())
            .await
            .expect("connect");
        store
            .insert_server_sample(&server_sample(7_000))
            .await
            .expect("first");
        store
            .insert_server_sample(&server_sample(7_000))
            .await
            .expect("second");

        let rows = store.query_server_range(0, i64::MAX).await.expect("query");
        assert_eq!(
            rows.len(),
            2,
            "samples share a timestamp but are distinct rows"
        );
        assert!(rows.iter().all(|row| row.timestamp_ms == 7_000));
    }

    #[tokio::test]
    async fn range_queries_are_capped_to_max_rows() {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

        let path = temp_db_path();
        let store = SqliteStore::connect(&path).await.expect("connect");

        // Bulk-insert past the cap through a second pool using a recursive CTE,
        // so the test does not pay for tens of thousands of round trips.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&path))
            .await
            .expect("pool");
        let over_cap = MAX_RANGE_ROWS + 50;
        sqlx::query(
            "WITH RECURSIVE seq(n) AS (SELECT 0 UNION ALL SELECT n + 1 FROM seq WHERE n < ?1) \
             INSERT INTO server_samples (timestamp_ms) SELECT n FROM seq",
        )
        .bind(over_cap)
        .execute(&pool)
        .await
        .expect("bulk insert");

        let server = store.query_server_range(0, i64::MAX).await.expect("query");
        assert_eq!(
            server.len() as i64,
            MAX_RANGE_ROWS,
            "a wide range must return no more than the cap"
        );

        let gpu = store.query_gpu_range(0, i64::MAX).await.expect("query");
        assert!(gpu.len() as i64 <= MAX_RANGE_ROWS);
    }
}
