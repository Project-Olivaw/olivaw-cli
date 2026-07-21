# kinematics/differential-drive

Differential-drive kinematics with explicit units: body twists
(`v` m/s, `ω` rad/s, CCW-positive) ↔ per-side wheel speeds, plus a
`mix()` that outputs per-mille motor commands (`-1000..=1000`) ready for
`drivers/l298n` and `comms/cmdvel-protocol`.

**Why it's a component and not three lines:** when a command exceeds what
the wheels can do, `mix()` scales *both* sides by the same factor instead of
clamping each independently — so a fast, tight turn stays a turn instead of
straightening out. Saturation bugs like that are miserable to debug on a
moving robot.

## Measuring your robot

| parameter             | how to get it                                          |
| --------------------- | ------------------------------------------------------ |
| `track_width_m`       | distance between the two wheel contact patches, metres |
| `max_wheel_speed_mps` | wheel circumference × motor no-load RPM / 60, minus ~20% for load |

Typical hobby 2WD chassis: `0.17 m`, `0.8 m/s`.

## Usage

```rust
mod kinematics { pub mod differential_drive; }   // module wiring in main.rs
use kinematics::differential_drive::DifferentialDrive;

let base = DifferentialDrive::new(0.17, 0.8);
let cmd = base.mix(0.4, 2.0);                    // arc left
motors.drive(cmd.left_permille, cmd.right_permille)?;   // drivers/l298n
```

`twist()` is the exact inverse of `wheel_speeds()` — use it with encoder
feedback for odometry.

## Try it

```bash
cargo run --example diffdrive_table
```
