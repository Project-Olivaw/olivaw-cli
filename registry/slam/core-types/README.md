# slam/core-types

The foundation types every 2D SLAM component shares, extracted from
[olivaw-slam](https://github.com/Project-Olivaw/olivaw-slam):

- **`Pose2`** — SE(2) rigid-body pose. Metres, radians, θ always normalized
  to `(-π, π]` through a single `normalize_angle` function (angle-wrapping
  bugs are the classic SLAM failure mode; keeping the invariant in one place
  is the fix). `compose`, `inverse`, `between`, `transform_point`.
- **`ScanCloud`** — a Cartesian lidar scan in the sensor frame,
  message-shaped for easy bridging to ROS2-style middleware.
- **`SlamError`** — shared error enum (`#[non_exhaustive]`).

Conventions, stated once and used everywhere: **metres, radians, x forward,
y left, counter-clockwise positive, convert sensor units at the boundary
and never again.**

Requires `std` (heap-allocated scans, nalgebra) — SLAM runs on the SBC side
of your robot, not the microcontroller.

## Usage

```rust
mod slam {                                   // module wiring in main.rs
    pub mod error;
    pub mod pose;
    pub mod scan;
}
use slam::pose::{Pose2, Point2};
use slam::scan::ScanCloud;

let odom_step = Pose2::new(0.05, 0.0, 0.01);       // 5 cm forward, slight left
let world_pose = previous_pose.compose(&odom_step);
let p_world = world_pose.transform_point(Point2::new(1.0, 0.0));
```

`slam/scan-matcher` builds on these types (installing it pulls this
component in automatically).
