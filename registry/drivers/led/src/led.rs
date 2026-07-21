//! Non-blocking LED blinker state machine.
//!
//! Generic over `embedded-hal 1.0`'s [`OutputPin`]; time comes from you.
//! Call [`Blinker::tick`] from your main loop with a monotonic millisecond
//! count and it drives the pin — no delays, no timers, so your loop stays
//! free for BLE/serial/sensor work.
//!
//! Ported from the Hands-On-Robotics module 06 firmware (C++, ESP32 DevKit
//! v1), where this exact state machine kept an LED responsive while a BLE
//! GATT server ran callbacks. Mode values 0–3 match that firmware's wire
//! protocol, so `comms` layers can cast bytes straight into [`Mode`].

use embedded_hal::digital::OutputPin;

/// What the LED should be doing. Discriminants match the module 06 BLE wire
/// protocol (one byte, 0–3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Mode {
    /// Steady off.
    #[default]
    Off = 0,
    /// Steady on.
    On = 1,
    /// Blink with a 500 ms half-period (1 Hz).
    Slow = 2,
    /// Blink with a 100 ms half-period (5 Hz).
    Fast = 3,
}

impl Mode {
    /// Decode a wire byte; anything out of range is the safe default
    /// ([`Mode::Off`]), matching the reference firmware.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Mode {
        match byte {
            1 => Mode::On,
            2 => Mode::Slow,
            3 => Mode::Fast,
            _ => Mode::Off,
        }
    }

    /// Half-period in milliseconds for the blinking modes, `None` for the
    /// steady ones.
    #[must_use]
    pub const fn half_period_ms(self) -> Option<u32> {
        match self {
            Mode::Slow => Some(500),
            Mode::Fast => Some(100),
            Mode::Off | Mode::On => None,
        }
    }
}

/// The blinker: owns the pin, tracks toggle timing.
pub struct Blinker<Pin> {
    pin: Pin,
    mode: Mode,
    led_high: bool,
    last_toggle_ms: u32,
}

impl<Pin: OutputPin> Blinker<Pin> {
    /// Take ownership of the pin. It is driven low on the first
    /// [`tick`](Self::tick) (mode starts as [`Mode::Off`]).
    pub fn new(pin: Pin) -> Self {
        Self {
            pin,
            mode: Mode::Off,
            led_high: false,
            last_toggle_ms: 0,
        }
    }

    /// Change what the LED does. Takes effect on the next
    /// [`tick`](Self::tick); switching to a blink mode restarts its phase.
    pub fn set_mode(&mut self, mode: Mode, now_ms: u32) {
        if mode != self.mode {
            self.mode = mode;
            self.last_toggle_ms = now_ms;
        }
    }

    /// The current mode.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Advance the state machine. `now_ms` is any monotonic millisecond
    /// counter (wrapping is handled). Call this at least a few times per
    /// half-period — a 5–20 ms loop tick keeps blinks crisp.
    ///
    /// # Errors
    ///
    /// Propagates the pin's error unchanged.
    pub fn tick(&mut self, now_ms: u32) -> Result<(), Pin::Error> {
        match self.mode {
            Mode::Off => self.set_led(false),
            Mode::On => self.set_led(true),
            Mode::Slow | Mode::Fast => {
                let half = self.mode.half_period_ms().unwrap_or(500);
                // wrapping_sub keeps this correct across u32 rollover (~49 days).
                if now_ms.wrapping_sub(self.last_toggle_ms) >= half {
                    self.last_toggle_ms = now_ms;
                    let next = !self.led_high;
                    self.set_led(next)?;
                }
                Ok(())
            }
        }
    }

    fn set_led(&mut self, high: bool) -> Result<(), Pin::Error> {
        if high == self.led_high {
            return Ok(());
        }
        if high {
            self.pin.set_high()?;
        } else {
            self.pin.set_low()?;
        }
        self.led_high = high;
        Ok(())
    }

    /// Release the pin, consuming the blinker.
    pub fn release(self) -> Pin {
        self.pin
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;

    #[derive(Default)]
    struct MockPin {
        high: bool,
        transitions: usize,
    }
    impl embedded_hal::digital::ErrorType for MockPin {
        type Error = Infallible;
    }
    impl OutputPin for MockPin {
        fn set_low(&mut self) -> Result<(), Infallible> {
            self.high = false;
            self.transitions += 1;
            Ok(())
        }
        fn set_high(&mut self) -> Result<(), Infallible> {
            self.high = true;
            self.transitions += 1;
            Ok(())
        }
    }

    #[test]
    fn mode_from_byte_defaults_unknown_to_off() {
        assert_eq!(Mode::from_byte(0), Mode::Off);
        assert_eq!(Mode::from_byte(1), Mode::On);
        assert_eq!(Mode::from_byte(2), Mode::Slow);
        assert_eq!(Mode::from_byte(3), Mode::Fast);
        assert_eq!(Mode::from_byte(200), Mode::Off);
    }

    #[test]
    fn steady_modes_do_not_retrigger_the_pin() {
        let mut b = Blinker::new(MockPin::default());
        b.set_mode(Mode::On, 0);
        for t in 0..100 {
            b.tick(t).unwrap();
        }
        assert!(b.pin.high);
        assert_eq!(b.pin.transitions, 1, "only the initial low→high write");
    }

    #[test]
    fn slow_blink_toggles_every_half_period() {
        let mut b = Blinker::new(MockPin::default());
        b.set_mode(Mode::Slow, 0);
        let mut states = Vec::new();
        for t in (0..2000).step_by(10) {
            b.tick(t).unwrap();
            states.push(b.pin.high);
        }
        // Toggles due at t = 500, 1000, 1500; the t = 2000 one is just past
        // the sampled window.
        assert_eq!(b.pin.transitions, 3);
        assert!(states.iter().any(|s| *s) && states.iter().any(|s| !*s));
    }

    #[test]
    fn survives_u32_millisecond_rollover() {
        let mut b = Blinker::new(MockPin::default());
        let start = u32::MAX - 250;
        b.set_mode(Mode::Slow, start);
        b.tick(start).unwrap();
        let before = b.pin.transitions;
        // 500 ms later, across the wrap.
        b.tick(start.wrapping_add(500)).unwrap();
        assert_eq!(b.pin.transitions, before + 1);
    }
}
