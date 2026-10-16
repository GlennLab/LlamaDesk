//! Ports: the interfaces the application depends on, implemented by `infra`.
//!
//! All traits are object-safe (via `async-trait`) so the application layer can
//! hold `Box<dyn ...>` / `Arc<dyn ...>` and swap implementations freely.

pub mod hardware;
pub mod provider;
pub mod storage;

pub use hardware::HardwareMonitor;
pub use provider::InferenceProvider;
pub use storage::HistoryStore;
