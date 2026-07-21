//! Scan matching: estimate the rigid transform between two lidar scans.
//!
//! Vendored from `olivaw-slam` (<https://github.com/Project-Olivaw/olivaw-slam>).
//! This component carries the point-to-point ICP matcher; the full crate
//! adds correlative matching (CSM) and scan-to-map for drift-free operation
//! without odometry.
//!
//! All matchers implement [`ScanMatcher`]: given a reference, a query, and an
//! initial guess, they return the pose of the query frame expressed in the
//! reference frame, with a covariance and a normalized score.

pub mod icp;
#[cfg(test)]
pub(crate) mod test_scenes;

use nalgebra::Matrix3;

pub use icp::{IcpConfig, IcpMatcher};

use super::error::SlamError;
use super::pose::Pose2;
use super::scan::ScanCloud;

/// Result of a scan match.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Estimated pose of the query frame in the reference frame
    /// (metres/radians).
    pub pose: Pose2,
    /// Covariance of the estimate over `(x, y, θ)` (m², m·rad, rad²).
    pub covariance: Matrix3<f64>,
    /// Normalized match quality in `0..=1`, higher is better.
    pub score: f64,
    /// Iterations (ICP) or candidate evaluations performed.
    pub iterations: usize,
    /// `true` if the estimate is trustworthy: the optimization converged
    /// with an acceptable inlier ratio. A `false` result is still returned —
    /// the caller decides what to do with a weak match.
    pub converged: bool,
}

/// A scan-to-scan matcher.
pub trait ScanMatcher {
    /// Estimate the pose of `query`'s frame expressed in `reference`'s frame,
    /// starting from `initial_guess` (metres/radians).
    ///
    /// # Errors
    ///
    /// [`SlamError::MatchFailed`] when no estimate can be produced at all
    /// (degenerate input, singular normal equations). Low-quality matches are
    /// *not* errors: they come back as `Ok` with `converged = false`.
    fn match_scans(
        &self,
        reference: &ScanCloud,
        query: &ScanCloud,
        initial_guess: &Pose2,
    ) -> Result<MatchResult, SlamError>;
}
