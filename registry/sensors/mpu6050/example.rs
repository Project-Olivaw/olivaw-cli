//! Read the MPU-6050 driver against a simulated chip — the same driver code
//! that runs on hardware, exercised on your desk.
//!
//! On an ESP32 (esp-hal), construct the bus instead of the mock:
//! `let i2c = I2c::new(peripherals.I2C0, Config::default())
//!      .with_sda(peripherals.GPIO21).with_scl(peripherals.GPIO22);`
//!
//! Run with: `cargo run --example mpu6050_read`

#[allow(dead_code, unused_imports)]
#[path = "../src/sensors/mpu6050.rs"]
mod mpu6050;

use core::convert::Infallible;
use std::cell::RefCell;
use std::rc::Rc;

use embedded_hal::i2c::{ErrorType, I2c, Operation};
use mpu6050::{Mpu6050, ADDR_DEFAULT};

/// A pretend MPU-6050: a register file behind a shared handle, so the test
/// bench can "tilt the board" while the driver owns the bus.
#[derive(Clone)]
struct FakeImu(Rc<RefCell<[u8; 128]>>);

impl FakeImu {
    fn new() -> Self {
        let mut regs = [0u8; 128];
        regs[0x75] = 0x68; // WHO_AM_I
        Self(Rc::new(RefCell::new(regs)))
    }

    /// Simulate the board tilting: gravity swings from z to x.
    fn tilt(&self, step: u16) {
        let angle = f32::from(step) * 0.15;
        let (s, c) = angle.sin_cos();
        let x = (s * 16384.0) as i16;
        let z = (c * 16384.0) as i16;
        let mut regs = self.0.borrow_mut();
        regs[0x3B..0x3D].copy_from_slice(&x.to_be_bytes()); // ACCEL_XOUT
        regs[0x3F..0x41].copy_from_slice(&z.to_be_bytes()); // ACCEL_ZOUT
        regs[0x41..0x43].copy_from_slice(&700i16.to_be_bytes()); // ~38.6 °C
    }
}

impl ErrorType for FakeImu {
    type Error = Infallible;
}

impl I2c for FakeImu {
    fn transaction(
        &mut self,
        _address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Infallible> {
        let mut regs = self.0.borrow_mut();
        let mut reg_ptr = 0usize;
        for op in operations {
            match op {
                Operation::Write(bytes) => {
                    reg_ptr = bytes[0] as usize;
                    if bytes.len() > 1 {
                        regs[reg_ptr] = bytes[1];
                    }
                }
                Operation::Read(buf) => {
                    for (i, slot) in buf.iter_mut().enumerate() {
                        *slot = regs[reg_ptr + i];
                    }
                }
            }
        }
        Ok(())
    }
}

fn main() {
    let chip = FakeImu::new();
    let mut imu = Mpu6050::new(chip.clone(), ADDR_DEFAULT);
    imu.init().expect("mock chip reports a valid WHO_AM_I");

    println!("tilting the simulated board:");
    println!("  step   accel x (g)  accel z (g)   temp (°C)");
    for step in 0..8u16 {
        chip.tilt(step);
        let s = imu.read().expect("mock bus is infallible");
        println!(
            "  {step:4}   {:+10.3}  {:+10.3}   {:8.2}",
            s.accel_g[0], s.accel_g[2], s.temp_c
        );
    }
}
