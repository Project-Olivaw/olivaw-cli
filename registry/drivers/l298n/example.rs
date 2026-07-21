//! Sweep an L298N motor pair through a speed profile — on your desk, with
//! mock pins, so you can see exactly what the driver does before wiring it.
//!
//! On real hardware, replace the mocks with your HAL's pins:
//!   ESP32 (esp-hal):  Output::new(...GPIO16..., Level::Low, ...) for IN pins,
//!                     an LEDC channel (implements SetDutyCycle) for EN.
//!
//! Run with: `cargo run --example l298n_sweep`

#[allow(dead_code, unused_imports)]
#[path = "../src/drivers/l298n.rs"]
mod l298n;

use core::convert::Infallible;
use std::cell::Cell;
use std::rc::Rc;

use embedded_hal::digital::OutputPin;
use embedded_hal::pwm::SetDutyCycle;
use l298n::{L298n, Motor};

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

/// A pretend 8-bit PWM channel (max duty 255, like the reference firmware).
#[derive(Clone, Default)]
struct FakePwm(Rc<Cell<u16>>);
impl embedded_hal::pwm::ErrorType for FakePwm {
    type Error = Infallible;
}
impl SetDutyCycle for FakePwm {
    fn max_duty_cycle(&self) -> u16 {
        255
    }
    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Infallible> {
        self.0.set(duty);
        Ok(())
    }
}

struct Channel {
    in1: FakePin,
    in2: FakePin,
    duty: FakePwm,
}

impl Channel {
    fn describe(&self) -> String {
        let dir = match (self.in1.0.get(), self.in2.0.get()) {
            (true, false) => "forward",
            (false, true) => "reverse",
            (false, false) => "coast  ",
            (true, true) => "brake  ",
        };
        format!("{dir} duty {:3}/255", self.duty.0.get())
    }
}

fn main() {
    let left = Channel {
        in1: FakePin::default(),
        in2: FakePin::default(),
        duty: FakePwm::default(),
    };
    let right = Channel {
        in1: FakePin::default(),
        in2: FakePin::default(),
        duty: FakePwm::default(),
    };

    let mut pair = L298n::new(
        Motor::new(left.in1.clone(), left.in2.clone(), left.duty.clone()),
        Motor::new(right.in1.clone(), right.in2.clone(), right.duty.clone()),
    );

    println!("speed sweep (per-mille -> pin states)");
    for speed in [-1000i16, -500, -100, 0, 100, 500, 1000] {
        pair.drive(speed, -speed).expect("mock pins are infallible");
        println!(
            "  cmd {speed:5}  left:  {}   right: {}",
            left.describe(),
            right.describe(),
        );
    }
    pair.stop().expect("mock pins are infallible");
    println!("stopped.");
}
