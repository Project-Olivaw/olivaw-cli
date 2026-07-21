//! Print the command table for a small 2WD robot: twists in, per-side motor
//! commands out. Pure math — runs anywhere.
//!
//! Run with: `cargo run --example diffdrive_table`

#[allow(dead_code, unused_imports)]
#[path = "../src/kinematics/differential_drive.rs"]
mod differential_drive;

use differential_drive::DifferentialDrive;

fn main() {
    // A typical hobby 2WD chassis: 17 cm track, ~0.8 m/s top wheel speed.
    let base = DifferentialDrive::new(0.17, 0.8);

    println!("  v (m/s)  ω (rad/s)   ->   left‰   right‰");
    for (v, omega, label) in [
        (0.0, 0.0, "stop"),
        (0.4, 0.0, "cruise straight"),
        (0.0, 3.0, "spin left in place"),
        (0.4, 2.0, "arc left"),
        (0.4, -2.0, "arc right"),
        (-0.3, 0.0, "reverse"),
        (0.8, 6.0, "saturated: scaled, arc preserved"),
    ] {
        let m = base.mix(v, omega);
        println!(
            "  {v:7.2}  {omega:9.2}   ->  {:6}  {:7}   {label}",
            m.left_permille, m.right_permille
        );
    }

    // Round-trip check: wheels → twist.
    let wheels = base.wheel_speeds(0.4, 2.0);
    let (v, omega) = base.twist(wheels);
    println!("\nround-trip: wheels {wheels:?} -> v={v:.2} m/s, ω={omega:.2} rad/s");
}
