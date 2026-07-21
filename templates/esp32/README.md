# {{project_name}}

A robotics project scaffolded by [olivaw](https://github.com/Project-Olivaw/olivaw-cli)
for the ESP32 (Xtensa, `no_std` via [esp-hal](https://github.com/esp-rs/esp-hal)).

## One-time setup

```bash
cargo install espup espflash
espup install               # installs the Xtensa Rust toolchain ("esp" channel)
```

## Flash the blinky

```bash
cargo run                   # builds, flashes over USB, opens the serial monitor
```

The onboard LED (GPIO2 on most DevKit boards) blinks at 1 Hz.

## Adding components

```bash
olivaw list
olivaw add sensors/mpu6050      # I2C IMU — SDA=GPIO21 SCL=GPIO22 by convention
olivaw add drivers/l298n        # dual H-bridge motor driver
```
