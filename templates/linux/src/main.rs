//! Hello-robot for the `linux` target: a simulated blinky.
//!
//! On a real SBC, swap the simulated pin for `linux-embedded-hal`'s
//! `CdevPin` — every olivaw component is generic over `embedded-hal` traits,
//! so the rest of your code stays identical.

use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut led_on = false;
    println!("simulated blinky — Ctrl-C to stop");
    loop {
        led_on = !led_on;
        println!("LED {}", if led_on { "on " } else { "off" });
        sleep(Duration::from_millis(500));
    }
}
