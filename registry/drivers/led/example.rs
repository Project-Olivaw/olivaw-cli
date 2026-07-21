//! Watch the blinker state machine run in your terminal — the "LED" is a
//! printed bar. Swap `FakePin` for a real GPIO on hardware; nothing else
//! changes.
//!
//! Run with: `cargo run --example led_modes`

#[allow(dead_code, unused_imports)]
#[path = "../src/drivers/led.rs"]
mod led;

use core::convert::Infallible;
use std::cell::Cell;
use std::rc::Rc;

use embedded_hal::digital::OutputPin;
use led::{Blinker, Mode};

/// A pretend GPIO whose level is observable from outside the driver.
#[derive(Clone, Default)]
struct FakePin(Rc<Cell<bool>>);
impl embedded_hal::digital::ErrorType for FakePin {
    type Error = Infallible;
}
impl OutputPin for FakePin {
    fn set_low(&mut self) -> Result<(), Infallible> {
        self.0.set(false);
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Infallible> {
        self.0.set(true);
        Ok(())
    }
}

fn main() {
    let level = FakePin::default();
    let mut blinker = Blinker::new(level.clone());

    // Simulated time: 3 seconds per mode, 50 ms loop tick (like a firmware
    // main loop). Each printed char is one tick: █ = LED on, · = off.
    let mut now_ms = 0u32;
    for (mode, label) in [
        (Mode::Off, "off "),
        (Mode::On, "on  "),
        (Mode::Slow, "slow"),
        (Mode::Fast, "fast"),
    ] {
        blinker.set_mode(mode, now_ms);
        print!("mode {label}  ");
        for _ in 0..60 {
            blinker.tick(now_ms).expect("mock pin is infallible");
            print!("{}", if level.0.get() { '█' } else { '·' });
            now_ms += 50;
        }
        println!();
    }
}
