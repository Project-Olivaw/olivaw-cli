# sensors/hcsr04

HC-SR04 ultrasonic ranger: blocking, poll-based, millimetres out. Generic
over `embedded-hal 1.0` (`OutputPin` trigger, `InputPin` echo, `DelayNs`
timing), so it runs unmodified on ESP32, RP2040, STM32 or a Linux SBC.

## Wiring (ESP32 DevKit reference)

| HC-SR04 pin | ESP32 pin | Notes                                    |
| ----------- | --------- | ---------------------------------------- |
| VCC         | VIN (5V)  | The classic module needs 5V to transmit  |
| GND         | GND       |                                          |
| TRIG        | GPIO5     | 3.3V trigger is accepted                 |
| ECHO        | GPIO4     | **Through a 1 kΩ / 2 kΩ divider** — ECHO is 5V and ESP32 pins are not 5V-tolerant |

The HC-SR04**P** variant runs entirely at 3.3V — no divider needed.

## Usage

```rust
mod sensors { pub mod hcsr04; }              // module wiring in main.rs
use sensors::hcsr04::Hcsr04;

let mut ranger = Hcsr04::new(trig_pin, echo_pin, delay);
match ranger.measure_mm() {
    Ok(mm) => defmt::info!("obstacle at {} mm", mm),
    Err(e) => defmt::warn!("no reading: {:?}", e),   // NoEcho / OutOfRange
}
// wait >= 60 ms before the next measurement (datasheet)
```

## Accuracy honesty

This driver measures the echo by polling every 5 µs (configurable), not by
timer capture. That bounds resolution at ~1 mm per 5 µs of polling and adds
jitter from your delay implementation — fine for obstacle avoidance,
not for metrology. If you need the last millimetre, wire ECHO to a timer
input-capture pin and use your HAL's capture API instead.

## Try it (no hardware)

```bash
cargo run --example hcsr04_sweep
```
