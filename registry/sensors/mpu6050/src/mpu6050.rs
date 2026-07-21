//! MPU-6050 / MPU-6500 6-axis IMU driver (I2C).
//!
//! Generic over `embedded-hal 1.0`'s [`I2c`] trait — no HAL types. Blocking
//! reads, no interrupts, no DMP: the 90% use case (read acceleration and
//! angular rate in a control loop) with nothing to configure first.
//!
//! Units are explicit: acceleration in g, angular rate in °/s, temperature
//! in °C. Raw LSB values are available if you need them.
//!
//! Register map: InvenSense MPU-6000/6050 Register Map rev 4.2.

use embedded_hal::i2c::I2c;

/// Default I2C address (AD0 low). AD0 high → [`ADDR_AD0_HIGH`].
pub const ADDR_DEFAULT: u8 = 0x68;
/// I2C address when the AD0 pin is pulled high.
pub const ADDR_AD0_HIGH: u8 = 0x69;

// Registers.
const REG_SMPLRT_DIV: u8 = 0x19;
const REG_CONFIG: u8 = 0x1A;
const REG_GYRO_CONFIG: u8 = 0x1B;
const REG_ACCEL_CONFIG: u8 = 0x1C;
const REG_ACCEL_XOUT_H: u8 = 0x3B;
const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_WHO_AM_I: u8 = 0x75;

/// WHO_AM_I values this driver recognizes.
const WHO_AM_I_MPU6050: u8 = 0x68;
const WHO_AM_I_MPU6500: u8 = 0x70;

/// Driver failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<E> {
    /// The underlying I2C transaction failed.
    I2c(E),
    /// WHO_AM_I returned a value this driver does not recognize — wrong
    /// address, wrong chip, or a wiring problem. Payload is the byte read.
    UnknownDevice(u8),
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::I2c(e)
    }
}

/// Accelerometer full-scale range. Wider range = coarser resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccelRange {
    /// ±2 g (16384 LSB/g) — default.
    #[default]
    G2,
    /// ±4 g (8192 LSB/g).
    G4,
    /// ±8 g (4096 LSB/g).
    G8,
    /// ±16 g (2048 LSB/g).
    G16,
}

impl AccelRange {
    const fn bits(self) -> u8 {
        (match self {
            Self::G2 => 0u8,
            Self::G4 => 1,
            Self::G8 => 2,
            Self::G16 => 3,
        }) << 3
    }

    /// LSB per g at this range.
    #[must_use]
    pub const fn lsb_per_g(self) -> f32 {
        match self {
            Self::G2 => 16384.0,
            Self::G4 => 8192.0,
            Self::G8 => 4096.0,
            Self::G16 => 2048.0,
        }
    }
}

/// Gyroscope full-scale range. Wider range = coarser resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GyroRange {
    /// ±250 °/s (131 LSB/(°/s)) — default.
    #[default]
    Dps250,
    /// ±500 °/s (65.5 LSB/(°/s)).
    Dps500,
    /// ±1000 °/s (32.8 LSB/(°/s)).
    Dps1000,
    /// ±2000 °/s (16.4 LSB/(°/s)).
    Dps2000,
}

impl GyroRange {
    const fn bits(self) -> u8 {
        (match self {
            Self::Dps250 => 0u8,
            Self::Dps500 => 1,
            Self::Dps1000 => 2,
            Self::Dps2000 => 3,
        }) << 3
    }

    /// LSB per °/s at this range.
    #[must_use]
    pub const fn lsb_per_dps(self) -> f32 {
        match self {
            Self::Dps250 => 131.0,
            Self::Dps500 => 65.5,
            Self::Dps1000 => 32.8,
            Self::Dps2000 => 16.4,
        }
    }
}

/// One full sensor reading, converted to physical units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sample {
    /// Acceleration `[x, y, z]` in g. At rest, z ≈ +1.0 with the chip flat.
    pub accel_g: [f32; 3],
    /// Angular rate `[x, y, z]` in °/s.
    pub gyro_dps: [f32; 3],
    /// Die temperature in °C (sensor self-heating adds a degree or two).
    pub temp_c: f32,
}

/// The driver. Owns the bus handle (or a shared-bus proxy).
pub struct Mpu6050<I2C> {
    i2c: I2C,
    address: u8,
    accel_range: AccelRange,
    gyro_range: GyroRange,
}

impl<I2C: I2c> Mpu6050<I2C> {
    /// Create the driver at `address` ([`ADDR_DEFAULT`] or
    /// [`ADDR_AD0_HIGH`]). Call [`init`](Self::init) before reading.
    pub fn new(i2c: I2C, address: u8) -> Self {
        Self {
            i2c,
            address,
            accel_range: AccelRange::default(),
            gyro_range: GyroRange::default(),
        }
    }

    /// Verify the chip identity and wake it from sleep (the power-on
    /// default is sleep mode — skipping this reads all zeros).
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if WHO_AM_I is not an MPU-6050/6500;
    /// [`Error::I2c`] on bus failure.
    pub fn init(&mut self) -> Result<(), Error<I2C::Error>> {
        let who = self.read_reg(REG_WHO_AM_I)?;
        if who != WHO_AM_I_MPU6050 && who != WHO_AM_I_MPU6500 {
            return Err(Error::UnknownDevice(who));
        }
        // Wake up, clock from the X-gyro PLL (datasheet-recommended over the
        // default internal oscillator for stability).
        self.write_reg(REG_PWR_MGMT_1, 0x01)?;
        // 1 kHz sample rate (divider 0), ~44 Hz DLPF: a sane control-loop
        // default that suppresses motor vibration aliasing.
        self.write_reg(REG_SMPLRT_DIV, 0x00)?;
        self.write_reg(REG_CONFIG, 0x03)?;
        self.set_accel_range(self.accel_range)?;
        self.set_gyro_range(self.gyro_range)?;
        Ok(())
    }

    /// Change the accelerometer full-scale range.
    ///
    /// # Errors
    ///
    /// [`Error::I2c`] on bus failure.
    pub fn set_accel_range(&mut self, range: AccelRange) -> Result<(), Error<I2C::Error>> {
        self.write_reg(REG_ACCEL_CONFIG, range.bits())?;
        self.accel_range = range;
        Ok(())
    }

    /// Change the gyroscope full-scale range.
    ///
    /// # Errors
    ///
    /// [`Error::I2c`] on bus failure.
    pub fn set_gyro_range(&mut self, range: GyroRange) -> Result<(), Error<I2C::Error>> {
        self.write_reg(REG_GYRO_CONFIG, range.bits())?;
        self.gyro_range = range;
        Ok(())
    }

    /// Read everything in one 14-byte burst (accel, temperature, gyro —
    /// they are contiguous in the register map), converted to units.
    ///
    /// # Errors
    ///
    /// [`Error::I2c`] on bus failure.
    pub fn read(&mut self) -> Result<Sample, Error<I2C::Error>> {
        let mut raw = [0u8; 14];
        self.i2c
            .write_read(self.address, &[REG_ACCEL_XOUT_H], &mut raw)?;
        let word = |i: usize| i16::from_be_bytes([raw[i], raw[i + 1]]);

        let a = self.accel_range.lsb_per_g();
        let g = self.gyro_range.lsb_per_dps();
        Ok(Sample {
            accel_g: [
                f32::from(word(0)) / a,
                f32::from(word(2)) / a,
                f32::from(word(4)) / a,
            ],
            // Datasheet: Temp(°C) = raw / 340 + 36.53.
            temp_c: f32::from(word(6)) / 340.0 + 36.53,
            gyro_dps: [
                f32::from(word(8)) / g,
                f32::from(word(10)) / g,
                f32::from(word(12)) / g,
            ],
        })
    }

    /// Release the bus handle, consuming the driver.
    pub fn release(self) -> I2C {
        self.i2c
    }

    fn read_reg(&mut self, reg: u8) -> Result<u8, Error<I2C::Error>> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(self.address, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    fn write_reg(&mut self, reg: u8, value: u8) -> Result<(), Error<I2C::Error>> {
        self.i2c.write(self.address, &[reg, value])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::i2c::{ErrorType, Operation};

    /// Register-level I2C mock: a tiny register file plus a write log.
    struct MockBus {
        regs: [u8; 128],
        writes: Vec<(u8, u8)>,
    }

    impl MockBus {
        fn new(who_am_i: u8) -> Self {
            let mut regs = [0u8; 128];
            regs[REG_WHO_AM_I as usize] = who_am_i;
            Self { regs, writes: Vec::new() }
        }
    }

    impl ErrorType for MockBus {
        type Error = core::convert::Infallible;
    }

    impl I2c for MockBus {
        fn transaction(
            &mut self,
            _address: u8,
            operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            let mut reg_ptr = 0u8;
            for op in operations {
                match op {
                    Operation::Write(bytes) => {
                        reg_ptr = bytes[0];
                        if bytes.len() > 1 {
                            self.regs[reg_ptr as usize] = bytes[1];
                            self.writes.push((bytes[0], bytes[1]));
                        }
                    }
                    Operation::Read(buf) => {
                        for (i, slot) in buf.iter_mut().enumerate() {
                            *slot = self.regs[reg_ptr as usize + i];
                        }
                    }
                }
            }
            Ok(())
        }
    }

    #[test]
    fn init_wakes_a_recognized_chip() {
        let mut imu = Mpu6050::new(MockBus::new(0x68), ADDR_DEFAULT);
        imu.init().unwrap();
        let bus = imu.release();
        assert!(bus.writes.contains(&(REG_PWR_MGMT_1, 0x01)), "must wake from sleep");
    }

    #[test]
    fn init_rejects_unknown_chips() {
        let mut imu = Mpu6050::new(MockBus::new(0x42), ADDR_DEFAULT);
        assert_eq!(imu.init(), Err(Error::UnknownDevice(0x42)));
    }

    #[test]
    fn converts_known_raw_values_to_units() {
        let mut bus = MockBus::new(0x68);
        // accel z = +16384 LSB = +1 g at ±2 g.
        bus.regs[0x3F] = 0x40; // ACCEL_ZOUT_H
        bus.regs[0x40] = 0x00;
        // temp raw = 0 → 36.53 °C.
        // gyro x = -131 LSB = -1 °/s at ±250.
        let neg = (-131i16).to_be_bytes();
        bus.regs[0x43] = neg[0];
        bus.regs[0x44] = neg[1];

        let mut imu = Mpu6050::new(bus, ADDR_DEFAULT);
        imu.init().unwrap();
        let s = imu.read().unwrap();
        assert!((s.accel_g[2] - 1.0).abs() < 1e-3, "z accel {}", s.accel_g[2]);
        assert!((s.temp_c - 36.53).abs() < 1e-2, "temp {}", s.temp_c);
        assert!((s.gyro_dps[0] + 1.0).abs() < 1e-3, "gyro x {}", s.gyro_dps[0]);
    }

    #[test]
    fn range_changes_rescale_readings() {
        let mut bus = MockBus::new(0x68);
        bus.regs[0x3B] = 0x40; // ACCEL_XOUT_H: +16384 LSB
        let mut imu = Mpu6050::new(bus, ADDR_DEFAULT);
        imu.init().unwrap();
        assert!((imu.read().unwrap().accel_g[0] - 1.0).abs() < 1e-3);
        imu.set_accel_range(AccelRange::G4).unwrap();
        assert!((imu.read().unwrap().accel_g[0] - 2.0).abs() < 1e-3);
    }
}
