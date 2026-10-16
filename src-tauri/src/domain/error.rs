//! Unified error type shared across the domain, ports, and infrastructure layers.
//!
//! The domain must not depend on transport crates, so underlying causes are
//! stored behind a boxed [`BoxError`] rather than a concrete `reqwest::Error`.

use thiserror::Error;

/// Boxed, thread-safe error used to preserve an underlying cause without
/// coupling the domain layer to a specific transport crate.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Error returned by every LlamaDesk backend operation.
#[derive(Debug, Error)]
pub enum MonitorError {
    /// Transport/HTTP-layer failure while talking to a provider.
    #[error("http error: {source}")]
    Http {
        /// Underlying transport error.
        source: BoxError,
    },

    /// The request exceeded its configured timeout.
    #[error("request timed out")]
    Timeout,

    /// The server returned a non-success HTTP status.
    #[error("unexpected status {status}: {body}")]
    HttpStatus {
        /// HTTP status code returned by the server.
        status: u16,
        /// Raw response body (possibly empty).
        body: String,
    },

    /// A response body could not be decoded into the expected shape.
    #[error("decode error: {source}")]
    Decode {
        /// Underlying decoding error.
        source: BoxError,
    },

    /// GPU/hardware probing failed.
    #[error("gpu error: {0}")]
    Gpu(String),

    /// Configuration was invalid or missing.
    #[error("config error: {0}")]
    Config(String),

    /// Persistence failed.
    #[error("storage error: {0}")]
    Storage(String),

    /// The requested resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),
}

impl MonitorError {
    /// Wraps a transport error as [`MonitorError::Http`].
    pub fn http(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Http {
            source: Box::new(source),
        }
    }

    /// Wraps a decoding error as [`MonitorError::Decode`].
    pub fn decode(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Decode {
            source: Box::new(source),
        }
    }

    /// Classification helper: `true` when the server actively refused the
    /// connection (nothing listening at the target address).
    ///
    /// Only a TCP `ConnectionRefused` counts. Timeouts, DNS, TLS, and reset
    /// failures all return `false` so callers report `Disconnected` solely for
    /// a refused connection and `Error` for every other transport failure.
    pub fn is_connection_refused(&self) -> bool {
        match self {
            Self::Http { source } => source_has_connection_refused(source.as_ref()),
            _ => false,
        }
    }

    /// Classification helper: `true` when the failure was a timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }
}

/// Walks `error`'s source chain looking for an [`std::io::Error`] whose kind is
/// [`std::io::ErrorKind::ConnectionRefused`].
///
/// A transport crate (e.g. `reqwest`) wraps the underlying socket error, so the
/// refusal is detected by inspecting the chain rather than the outer type. This
/// keeps the classification independent of any transport dependency.
fn source_has_connection_refused(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(err) = current {
        if let Some(io) = err.downcast_ref::<std::io::Error>()
            && io.kind() == std::io::ErrorKind::ConnectionRefused
        {
            return true;
        }
        current = err.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_refused_kind_is_detected() {
        let refused = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        assert!(MonitorError::http(refused).is_connection_refused());
    }

    #[test]
    fn timeout_and_other_transport_failures_are_not_connection_refused() {
        assert!(!MonitorError::Timeout.is_connection_refused());
        assert!(!MonitorError::http(std::io::Error::other("dns")).is_connection_refused());
        assert!(
            !MonitorError::http(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "reset"
            ))
            .is_connection_refused()
        );
        assert!(!MonitorError::Config("bad".to_string()).is_connection_refused());
    }

    #[test]
    fn refusal_is_found_through_a_source_chain() {
        #[derive(Debug)]
        struct Outer(std::io::Error);
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("outer transport error")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }

        let nested = Outer(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "refused",
        ));
        assert!(MonitorError::http(nested).is_connection_refused());
    }
}
