//! Blinky for the Raspberry Pi Pico (RP2040): onboard LED on GPIO25.
//!
//! Flash it: hold BOOTSEL, plug the Pico in, then `cargo run`
//! (uses `elf2uf2-rs -d`; see .cargo/config.toml for the probe-rs option).

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use panic_halt as _;
use rp2040_hal as hal;

use hal::pac;

/// Second-stage bootloader, required at the start of flash.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// External crystal frequency on the Pico board, Hz.
const XTAL_FREQ_HZ: u32 = 12_000_000;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().expect("peripherals taken once");
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap_or_else(|_| panic!("clock init failed"));

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led = pins.gpio25.into_push_pull_output();
    loop {
        let _ = led.set_high();
        timer.delay_ms(500);
        let _ = led.set_low();
        timer.delay_ms(500);
    }
}
