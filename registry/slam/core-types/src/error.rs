//! Shared error type for the vendored SLAM components.
//!
//! Vendored (and trimmed) from `olivaw-slam`. The enum is
//! `#[non_exhaustive]`: other SLAM components add variants without breaking
//! your match arms.

/// Errors returned by the vendored SLAM component APIs.
#[derive(Debug)]
#[non_exhaustive]
pub enum SlamError {
    /// A configuration field failed validation.
    InvalidConfig {
        /// Name of the offending config field.
        field: &'static str,
        /// Why the value was rejected.
        reason: String,
    },

    /// An input scan exceeded a configured point-count limit.
    ///
    /// This is a denial-of-service guard: a malformed input returns an error
    /// instead of driving unbounded allocation.
    ScanTooLarge {
        /// Number of points in the rejected scan.
        actual: usize,
        /// Configured maximum.
        limit: usize,
    },

    /// Scan matching failed to produce a usable estimate.
    MatchFailed {
        /// Why the match was rejected (too few points, no convergence, …).
        reason: String,
    },
}

impl std::fmt::Display for SlamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig { field, reason } => {
                write!(f, "invalid config field `{field}`: {reason}")
            }
            Self::ScanTooLarge { actual, limit } => {
                write!(f, "scan has {actual} points, exceeding limit {limit}")
            }
            Self::MatchFailed { reason } => write!(f, "scan match failed: {reason}"),
        }
    }
}

impl std::error::Error for SlamError {}
