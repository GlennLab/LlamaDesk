//! NVIDIA GPU telemetry via `nvidia-smi`.
//!
//! The query runs under a hard timeout with `kill_on_drop`, so a hung
//! `nvidia-smi` can never block the application. A missing binary is treated as
//! "hardware unavailable" (empty snapshot), not an error.

use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;

use crate::domain::error::MonitorError;
use crate::domain::hardware::GpuStats;
use crate::ports::hardware::HardwareMonitor;

/// Default timeout for an `nvidia-smi` query.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);

/// Comma-separated fields requested from `nvidia-smi`.
const QUERY_FIELDS: &str =
    "index,name,utilization.gpu,memory.total,memory.used,temperature.gpu,power.draw,power.limit";

const UNKNOWN: u8 = 0;
const AVAILABLE: u8 = 1;
const UNAVAILABLE: u8 = 2;

/// Reads GPU stats by shelling out to `nvidia-smi`.
#[derive(Debug)]
pub struct NvidiaSmiMonitor {
    program: String,
    timeout: Duration,
    availability: AtomicU8,
}

impl Default for NvidiaSmiMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl NvidiaSmiMonitor {
    /// Creates a monitor that invokes `nvidia-smi` from `PATH`.
    pub fn new() -> Self {
        Self::with_program("nvidia-smi")
    }

    /// Creates a monitor that invokes a specific binary.
    pub fn with_program(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            timeout: DEFAULT_TIMEOUT,
            availability: AtomicU8::new(UNKNOWN),
        }
    }

    /// Overrides the query timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Captures the current statistics for every visible GPU.
    ///
    /// Returns an empty vector when no supported backend is available.
    pub async fn snapshot(&self) -> Result<Vec<GpuStats>, MonitorError> {
        if !self.available() {
            return Ok(Vec::new());
        }
        match self.query().await? {
            Some(output) => Ok(parse_csv(&output)),
            None => Ok(Vec::new()),
        }
    }

    /// Whether an `nvidia-smi` binary is present (probed once, then cached).
    pub fn available(&self) -> bool {
        match self.availability.load(Ordering::Relaxed) {
            AVAILABLE => true,
            UNAVAILABLE => false,
            _ => {
                let found = command_exists(&self.program);
                self.availability.store(
                    if found { AVAILABLE } else { UNAVAILABLE },
                    Ordering::Relaxed,
                );
                found
            }
        }
    }

    /// Runs the query. Returns `Ok(None)` when the binary has disappeared.
    async fn query(&self) -> Result<Option<String>, MonitorError> {
        let mut command = Command::new(&self.program);
        command
            .arg(format!("--query-gpu={QUERY_FIELDS}"))
            .arg("--format=csv,noheader,nounits")
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.availability.store(UNAVAILABLE, Ordering::Relaxed);
                return Ok(None);
            }
            Err(error) => {
                return Err(MonitorError::Gpu(format!(
                    "failed to spawn nvidia-smi: {error}"
                )));
            }
        };

        let output = match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(error)) => {
                return Err(MonitorError::Gpu(format!("nvidia-smi failed: {error}")));
            }
            // The future is dropped here, and `kill_on_drop` reaps the child.
            Err(_) => return Err(MonitorError::Timeout),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MonitorError::Gpu(format!(
                "nvidia-smi exited with {}: {}",
                output.status,
                stderr.trim()
            )));
        }

        Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
    }
}

#[async_trait]
impl HardwareMonitor for NvidiaSmiMonitor {
    async fn snapshot(&self) -> Result<Vec<GpuStats>, MonitorError> {
        NvidiaSmiMonitor::snapshot(self).await
    }

    fn available(&self) -> bool {
        NvidiaSmiMonitor::available(self)
    }
}

/// Parses the CSV emitted by `nvidia-smi --format=csv,noheader,nounits`.
fn parse_csv(text: &str) -> Vec<GpuStats> {
    text.lines().filter_map(parse_line).collect()
}

/// Parses one CSV row; returns `None` for blank or unusable rows.
fn parse_line(line: &str) -> Option<GpuStats> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let columns: Vec<&str> = line.split(',').map(str::trim).collect();
    if columns.len() < 2 {
        return None;
    }
    let index = u32::try_from(parse_u64(columns[0])?).ok()?;
    Some(GpuStats {
        index,
        name: columns[1].to_string(),
        utilization_percent: columns.get(2).copied().and_then(parse_f64),
        memory_total_mib: columns.get(3).copied().and_then(parse_u64),
        memory_used_mib: columns.get(4).copied().and_then(parse_u64),
        temperature_c: columns.get(5).copied().and_then(parse_f64),
        power_draw_w: columns.get(6).copied().and_then(parse_f64),
        power_limit_w: columns.get(7).copied().and_then(parse_f64),
    })
}

/// Parses a numeric field, treating `N/A`, `[N/A]`, and blanks as absent.
fn parse_f64(value: &str) -> Option<f64> {
    let value = value.trim();
    if is_missing(value) {
        return None;
    }
    value.parse::<f64>().ok()
}

/// Parses an integer field, treating `N/A`, `[N/A]`, and blanks as absent.
fn parse_u64(value: &str) -> Option<u64> {
    let value = value.trim();
    if is_missing(value) {
        return None;
    }
    value.parse::<u64>().ok()
}

/// Whether a field is empty or an `N/A` placeholder.
fn is_missing(value: &str) -> bool {
    value.is_empty() || value.eq_ignore_ascii_case("n/a") || value.eq_ignore_ascii_case("[n/a]")
}

/// Whether a program name resolves to a file on disk or on `PATH`.
fn command_exists(program: &str) -> bool {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return path.is_file();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| dir.join(program).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "0, NVIDIA GeForce RTX 3090 Ti, 24, 24564, 21958, 52, 206.03, 480.00\r\n\
                          1, NVIDIA RTX PRO 5000 Blackwell, 77, 48935, 45720, 70, 207.76, 300.00\r\n";

    #[test]
    fn parses_two_gpus() {
        let gpus = parse_csv(SAMPLE);
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].index, 0);
        assert_eq!(gpus[0].name, "NVIDIA GeForce RTX 3090 Ti");
        assert_eq!(gpus[0].utilization_percent, Some(24.0));
        assert_eq!(gpus[0].memory_total_mib, Some(24564));
        assert_eq!(gpus[0].memory_used_mib, Some(21958));
        assert_eq!(gpus[0].temperature_c, Some(52.0));
        assert_eq!(gpus[0].power_draw_w, Some(206.03));
        assert_eq!(gpus[0].power_limit_w, Some(480.0));
        assert_eq!(gpus[1].index, 1);
        assert_eq!(gpus[1].name, "NVIDIA RTX PRO 5000 Blackwell");
        assert_eq!(gpus[1].utilization_percent, Some(77.0));
    }

    #[test]
    fn treats_na_fields_as_none() {
        let gpus = parse_csv("0, RTX 3090, [N/A], 24564, N/A, 52, , 480.00\n");
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].utilization_percent, None);
        assert_eq!(gpus[0].memory_used_mib, None);
        assert_eq!(gpus[0].power_draw_w, None);
        assert_eq!(gpus[0].temperature_c, Some(52.0));
    }

    #[test]
    fn handles_extra_and_missing_columns() {
        let gpus =
            parse_csv("0, RTX 3090, 24, 24564, 21958, 52, 206.03, 480.00, extra\n1, RTX 5000");
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].utilization_percent, Some(24.0));
        assert_eq!(gpus[1].name, "RTX 5000");
        assert_eq!(gpus[1].power_limit_w, None);
    }

    #[test]
    fn empty_and_malformed_input_yields_nothing() {
        assert!(parse_csv("").is_empty());
        assert!(parse_csv("\n  \n").is_empty());
        // Non-numeric index row is skipped.
        assert!(parse_csv("not,a,gpu,row").is_empty());
    }

    #[test]
    fn shorter_row_does_not_pad_missing_columns_with_zero() {
        let gpus = parse_csv("0, RTX 3090, 24, 24564\n");
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].utilization_percent, Some(24.0));
        assert_eq!(gpus[0].memory_total_mib, Some(24564));
        assert_eq!(gpus[0].memory_used_mib, None);
        assert_eq!(gpus[0].temperature_c, None);
        assert_eq!(gpus[0].power_draw_w, None);
        assert_eq!(gpus[0].power_limit_w, None);
    }

    #[test]
    fn empty_field_is_none_not_zero() {
        let gpus = parse_csv("0, GPU, , 100, 50, 40, 10, 20\n");
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].utilization_percent, None);
        assert_eq!(gpus[0].memory_total_mib, Some(100));
        assert_eq!(gpus[0].memory_used_mib, Some(50));
        assert_eq!(gpus[0].temperature_c, Some(40.0));
        assert_eq!(gpus[0].power_draw_w, Some(10.0));
        assert_eq!(gpus[0].power_limit_w, Some(20.0));
    }

    #[test]
    fn na_power_limit_only_is_none() {
        let gpus = parse_csv("0, RTX 3090, 24, 24564, 21958, 52, 206.03, [N/A]\n");
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].power_limit_w, None);
        assert_eq!(gpus[0].power_draw_w, Some(206.03));
        assert_eq!(gpus[0].utilization_percent, Some(24.0));
    }

    #[test]
    fn non_numeric_field_is_none() {
        let gpus = parse_csv("0, RTX 3090, abc, 24564, 21958, 52, 206.03, 480.00\n");
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].utilization_percent, None);
        assert_eq!(gpus[0].memory_total_mib, Some(24564));
        assert_eq!(gpus[0].power_limit_w, Some(480.0));
    }
}
