# drivers/led

A non-blocking LED blinker: four modes (`Off`, `On`, `Slow` 1 Hz, `Fast`
5 Hz), driven by calling `tick(now_ms)` from your main loop. No delays, no
timers claimed — your loop stays free for radio callbacks and sensor reads,
which is exactly why the reference firmware (Hands-On-Robotics module 06,
BLE LED controller) was built this way.

Mode discriminants match that firmware's BLE wire byte (0–3), so a comms
layer can `Mode::from_byte(payload[0])` directly; unknown bytes safely map
to `Off`.

## Wiring

| LED           | MCU pin              |
| ------------- | -------------------- |
| Onboard       | GPIO2 (ESP32 DevKit) / GPIO25 (Pi Pico) — no wiring |
| External LED+ | any GPIO → 220 Ω–1 kΩ resistor → LED anode |
| External LED− | GND                  |

## Usage

```rust
mod drivers { pub mod led; }                 // module wiring in main.rs
use drivers::led::{Blinker, Mode};

let mut blinker = Blinker::new(led_pin);     // any embedded-hal OutputPin
blinker.set_mode(Mode::Slow, now_ms);
loop {
    blinker.tick(now_ms)?;                   // call every 5-20 ms
    // ... the rest of your loop keeps running
}
```

`now_ms` is any monotonic millisecond counter (e.g. esp-hal's
`Instant::now()`, RP2040's timer). `u32` rollover is handled.

## Try it (no hardware)

```bash
cargo run --example led_modes
```

Prints a tick-by-tick trace of each mode: `mode slow  ██████████··········…`
