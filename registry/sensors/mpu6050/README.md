# sensors/mpu6050

MPU-6050 / MPU-6500 6-axis IMU driver over `embedded-hal 1.0` I2C. Blocking,
burst-read, explicit units: acceleration in **g**, angular rate in **°/s**,
temperature in **°C**. `init()` verifies WHO_AM_I and wakes the chip (it
powers on asleep — the classic "all my readings are zero" bug).

## Wiring (GY-521 breakout on ESP32 DevKit)

| GY-521 pin | ESP32 pin | Notes                          |
| ---------- | --------- | ------------------------------ |
| VCC        | 3.3V      | 5V also fine (onboard LDO)     |
| GND        | GND       |                                |
| SDA        | GPIO21    | I2C data                       |
| SCL        | GPIO22    | I2C clock, 100–400 kHz         |
| AD0        | (float)   | High → address 0x69            |

## Usage

```rust
mod sensors { pub mod mpu6050; }             // module wiring in main.rs
use sensors::mpu6050::{Mpu6050, AccelRange, ADDR_DEFAULT};

let mut imu = Mpu6050::new(i2c, ADDR_DEFAULT);
imu.init()?;                                  // WHO_AM_I check + wake
imu.set_accel_range(AccelRange::G4)?;         // optional; ±2 g default
loop {
    let s = imu.read()?;                      // one 14-byte burst
    // s.accel_g, s.gyro_dps, s.temp_c
}
```

Defaults after `init()`: ±2 g, ±250 °/s, 1 kHz sample rate, ~44 Hz low-pass
filter (suppresses motor vibration).

## Try it (no hardware)

```bash
cargo run --example mpu6050_read
```

## Troubleshooting

- **`UnknownDevice(0x00)` / bus errors** — SDA/SCL swapped, or missing
  pull-ups (GY-521 has them onboard; bare chips need 4.7 kΩ).
- **All readings zero** — the chip was not woken; call `init()`.
- **z ≈ 1 g but noisy** — normal. Mount the IMU on foam if it sits over
  motors; the DLPF helps but cannot fix a vibrating board.
