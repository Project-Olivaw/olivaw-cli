//! Match two synthetic lidar scans of a room and recover the robot's motion
//! between them — the whole point of a scan matcher, on your desk.
//!
//! Run with: `cargo run --example scan_match_demo`

// Mount the vendored tree exactly as a project does (`mod slam { … }` in
// main.rs); the `#[path]` re-bases the module onto the vendored directory.
#[allow(dead_code, unused_imports)]
#[path = "../src/slam"]
mod slam {
    pub mod error;
    pub mod matcher;
    pub mod pose;
    pub mod scan;
}

use slam::matcher::{IcpMatcher, ScanMatcher};
use slam::pose::{Point2, Pose2};
use slam::scan::ScanCloud;

/// Ray-cast a 360-beam scan of a rectangular room with an inner box, from
/// `pose`. (A miniature of the test scene used by the vendored unit tests.)
fn room_scan(pose: Pose2) -> ScanCloud {
    let mut segs: Vec<(Point2, Point2)> = Vec::new();
    let mut rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
        segs.push((Point2::new(x0, y0), Point2::new(x1, y0)));
        segs.push((Point2::new(x1, y0), Point2::new(x1, y1)));
        segs.push((Point2::new(x1, y1), Point2::new(x0, y1)));
        segs.push((Point2::new(x0, y1), Point2::new(x0, y0)));
    };
    rect(-3.0, -2.0, 4.0, 3.0);
    rect(0.5, 0.5, 1.5, 1.2);

    let origin = Point2::new(pose.x, pose.y);
    let mut points = Vec::with_capacity(360);
    for i in 0..360 {
        let bearing = f64::from(i) * std::f64::consts::TAU / 360.0;
        let world_angle = pose.theta + bearing;
        let dir = (world_angle.cos(), world_angle.sin());
        let mut range = f64::INFINITY;
        for &(a, b) in &segs {
            let (ex, ey) = (b.x - a.x, b.y - a.y);
            let denom = dir.0 * ey - dir.1 * ex;
            if denom.abs() < 1e-12 {
                continue;
            }
            let (ox, oy) = (a.x - origin.x, a.y - origin.y);
            let t = (ox * ey - oy * ex) / denom;
            let u = (ox * dir.1 - oy * dir.0) / -denom;
            if t > 1e-9 && (0.0..=1.0).contains(&u) {
                range = range.min(t);
            }
        }
        if range.is_finite() {
            points.push(Point2::new(range * bearing.cos(), range * bearing.sin()));
        }
    }
    ScanCloud::new(points, 0)
}

fn main() {
    // The robot moved 12 cm forward, 8 cm right, turned 4° left — pretend we
    // don't know that and ask ICP to recover it from the two scans.
    let truth = Pose2::new(0.12, -0.08, 4.0_f64.to_radians());
    let reference = room_scan(Pose2::identity());
    let query = room_scan(truth);

    let matcher = IcpMatcher::default();
    let result = matcher
        .match_scans(&reference, &query, &Pose2::identity())
        .expect("scans are well-formed");

    println!("ground truth:  x {:+.3} m   y {:+.3} m   θ {:+.2}°", truth.x, truth.y, truth.theta.to_degrees());
    println!(
        "ICP estimate:  x {:+.3} m   y {:+.3} m   θ {:+.2}°",
        result.pose.x,
        result.pose.y,
        result.pose.theta.to_degrees()
    );
    println!(
        "converged: {}   score: {:.2}   iterations: {}",
        result.converged, result.score, result.iterations
    );
    println!(
        "σ (from covariance): x {:.1} mm, y {:.1} mm",
        result.covariance.m11.sqrt() * 1000.0,
        result.covariance.m22.sqrt() * 1000.0
    );
}
