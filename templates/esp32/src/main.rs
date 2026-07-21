//! Blinky for the ESP32 DevKit: onboard LED on GPIO2.
//!
//! One-time setup, then `cargo run` flashes and monitors:
//! ```sh
//! espup install            # Xtensa Rust toolchain
//! cargo install espflash
//! ```

#![no_std]
#![no_main]

use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let delay = Delay::new();

    loop {
        led.toggle();
        delay.delay_millis(500);
    }
}
