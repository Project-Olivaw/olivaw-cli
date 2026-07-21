//! Run the HC-SR04 driver against a simulated target moving away — the same
//! driver code that runs on hardware, on your desk.
//!
//! Run with: `cargo run --example hcsr04_sweep`

#[allow(dead_code, unused_imports)]
#[path = "../src/sensors/hcsr04.rs"]
mod hcsr04;

use core::convert::Infallible;
use std::cell::RefCell;
use std::rc::Rc;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use hcsr04::Hcsr04;

/// Bench state: a virtual clock plus the echo-pulse window that a target at
/// `distance_mm` would produce.
#[derive(Default)]
struct Bench {
    now_us: u32,
    echo_start_us: u32,
    echo_end_us: u32,
}

#[derive(Clone, Default)]
struct Shared(Rc<RefCell<Bench>>);

impl Shared {
    fn place_target(&self, distance_mm: f32) {
        let mut b = self.0.borrow_mut();
        let echo_us = distance_mm * 2.0 / 0.343; // round trip at 343 m/s
        b.echo_start_us = b.now_us + 450; // burst fire delay
        b.echo_end_us = b.echo_start_us + echo_us as u32;
    }
}

struct Trig;
impl embedded_hal::digital::ErrorType for Trig {
    type Error = Infallible;
}
impl OutputPin for Trig {
    fn set_low(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
}

struct Echo(Shared);
impl embedded_hal::digital::ErrorType for Echo {
    type Error = Infallible;
}
impl InputPin for Echo {
    fn is_high(&mut self) -> Result<bool, Infallible> {
        let b = self.0 .0.borrow();
        Ok(b.now_us >= b.echo_start_us && b.now_us < b.echo_end_us)
    }
    fn is_low(&mut self) -> Result<bool, Infallible> {
        self.is_high().map(|h| !h)
    }
}

struct Clock(Shared);
impl DelayNs for Clock {
    fn delay_ns(&mut self, ns: u32) {
        self.0 .0.borrow_mut().now_us += ns / 1000;
    }
}

fn main() {
    let bench = Shared::default();
    let mut sensor = Hcsr04::new(Trig, Echo(bench.clone()), Clock(bench.clone()));

    println!("target moving away from the sensor:");
    println!("  actual (mm)   measured (mm)");
    for actual in [50.0f32, 150.0, 400.0, 1000.0, 2500.0, 4000.0] {
        bench.place_target(actual);
        match sensor.measure_mm() {
            Ok(mm) => println!("  {actual:11.0}   {mm:13.1}"),
            Err(err) => println!("  {actual:11.0}   error: {err:?}"),
        }
    }

    // Nothing in range: the module would hold echo low forever.
    bench.0.borrow_mut().echo_start_us = u32::MAX;
    println!("  (no target)   {:?}", sensor.measure_mm());
}
