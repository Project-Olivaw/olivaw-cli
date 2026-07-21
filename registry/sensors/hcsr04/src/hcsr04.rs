//! HC-SR04 ultrasonic distance sensor driver.
//!
//! Generic over `embedded-hal 1.0`: trigger is an [`OutputPin`], echo is an
//! [`InputPin`], timing comes from a [`DelayNs`]. Blocking, poll-based —
//! portable to any HAL, at the cost of a little timing accuracy (see the
//! README; for centimetre work it is plenty, the sensor itself is ±3 mm at
//! best).
//!
//! Protocol (datasheet): a ≥10 µs high pulse on TRIG makes the module emit
//! an 8-cycle 40 kHz burst; ECHO then goes high for a time proportional to
//! the round-trip: `distance = echo_high_time × v_sound / 2`.

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};

/// Speed of sound at ~20 °C in mm/µs (343 m/s). The ~0.6%/°C temperature
/// dependence is below the sensor's own accuracy.
const MM_PER_US_HALVED: f32 = 0.343 / 2.0;

/// Measurement failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<E> {
    /// A pin operation failed.
    Pin(E),
    /// Echo never started — no module, wrong wiring, or a missed pulse.
    NoEcho,
    /// Echo ran past the maximum range — nothing to bounce off (the module
    /// holds ECHO high ~200 ms when it hears no return).
    OutOfRange,
}

/// The driver. `poll_interval_us` sets the measurement granularity —
/// see [`Hcsr04::with_poll_interval`].
pub struct Hcsr04<Trig, Echo, Delay> {
    trig: Trig,
    echo: Echo,
    delay: Delay,
    poll_interval_us: u32,
}

impl<E, Trig, Echo, Delay> Hcsr04<Trig, Echo, Delay>
where
    Trig: OutputPin<Error = E>,
    Echo: InputPin<Error = E>,
    Delay: DelayNs,
{
    /// Default polling granularity: 5 µs (≈ 0.9 mm of range resolution —
    /// finer than the sensor's ±3 mm accuracy).
    pub fn new(trig: Trig, echo: Echo, delay: Delay) -> Self {
        Self {
            trig,
            echo,
            delay,
            poll_interval_us: 5,
        }
    }

    /// Override the echo polling granularity (µs). Coarser polling costs
    /// range resolution (`0.17 mm × interval_us`); finer costs CPU.
    #[must_use]
    pub fn with_poll_interval(mut self, interval_us: u32) -> Self {
        self.poll_interval_us = interval_us.max(1);
        self
    }

    /// One blocking measurement, in millimetres.
    ///
    /// Takes up to ~25 ms for a 4 m target; on [`Error::OutOfRange`] /
    /// [`Error::NoEcho`] it returns after the internal timeout (~35 ms).
    /// Wait ≥60 ms between measurements (datasheet) or old echoes alias.
    ///
    /// # Errors
    ///
    /// [`Error::Pin`] on pin failure, [`Error::NoEcho`] when the echo pulse
    /// never starts, [`Error::OutOfRange`] beyond ~4 m.
    pub fn measure_mm(&mut self) -> Result<f32, Error<E>> {
        // 1. Trigger: ≥10 µs high pulse.
        self.trig.set_low().map_err(Error::Pin)?;
        self.delay.delay_us(2);
        self.trig.set_high().map_err(Error::Pin)?;
        self.delay.delay_us(10);
        self.trig.set_low().map_err(Error::Pin)?;

        // 2. Wait for echo start. The burst takes ~450 µs to fire; give it
        //    5 ms before declaring the module absent.
        let start_budget_polls = 5_000 / self.poll_interval_us;
        let mut waited = 0u32;
        while self.echo.is_low().map_err(Error::Pin)? {
            waited += 1;
            if waited > start_budget_polls {
                return Err(Error::NoEcho);
            }
            self.delay.delay_us(self.poll_interval_us);
        }

        // 3. Time the echo pulse. 4 m ≈ 23.3 ms round trip; budget 30 ms.
        let echo_budget_polls = 30_000 / self.poll_interval_us;
        let mut polls = 0u32;
        while self.echo.is_high().map_err(Error::Pin)? {
            polls += 1;
            if polls > echo_budget_polls {
                return Err(Error::OutOfRange);
            }
            self.delay.delay_us(self.poll_interval_us);
        }

        // Each poll ≈ poll_interval_us of echo-high time.
        #[allow(clippy::cast_precision_loss)]
        let echo_us = (polls * self.poll_interval_us) as f32;
        Ok(echo_us * MM_PER_US_HALVED)
    }

    /// Release the pins and delay, consuming the driver.
    pub fn release(self) -> (Trig, Echo, Delay) {
        (self.trig, self.echo, self.delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Shared clock: delays advance it; the echo pin derives its level
    /// from it, simulating a pulse of a chosen width.
    #[derive(Default)]
    struct Bench {
        now_us: u32,
        echo_start_us: u32,
        echo_end_us: u32,
    }

    #[derive(Clone, Default)]
    struct Shared(Rc<RefCell<Bench>>);

    struct FakeTrig;
    impl embedded_hal::digital::ErrorType for FakeTrig {
        type Error = Infallible;
    }
    impl OutputPin for FakeTrig {
        fn set_low(&mut self) -> Result<(), Infallible> {
            Ok(())
        }
        fn set_high(&mut self) -> Result<(), Infallible> {
            Ok(())
        }
    }

    struct FakeEcho(Shared);
    impl embedded_hal::digital::ErrorType for FakeEcho {
        type Error = Infallible;
    }
    impl InputPin for FakeEcho {
        fn is_high(&mut self) -> Result<bool, Infallible> {
            let b = self.0 .0.borrow();
            Ok(b.now_us >= b.echo_start_us && b.now_us < b.echo_end_us)
        }
        fn is_low(&mut self) -> Result<bool, Infallible> {
            self.is_high().map(|h| !h)
        }
    }

    struct FakeDelay(Shared);
    impl DelayNs for FakeDelay {
        fn delay_ns(&mut self, ns: u32) {
            self.0 .0.borrow_mut().now_us += ns / 1000;
        }
    }

    fn sensor_with_pulse(
        start_us: u32,
        width_us: u32,
    ) -> Hcsr04<FakeTrig, FakeEcho, FakeDelay> {
        let shared = Shared::default();
        {
            let mut b = shared.0.borrow_mut();
            b.echo_start_us = start_us;
            b.echo_end_us = start_us + width_us;
        }
        Hcsr04::new(FakeTrig, FakeEcho(shared.clone()), FakeDelay(shared))
    }

    #[test]
    fn measures_a_1m_target_within_tolerance() {
        // 1 m → 2 m round trip → 5831 µs echo.
        let mut s = sensor_with_pulse(500, 5831);
        let mm = s.measure_mm().unwrap();
        assert!((mm - 1000.0).abs() < 20.0, "got {mm} mm");
    }

    #[test]
    fn measures_close_range() {
        // 50 mm → 292 µs echo.
        let mut s = sensor_with_pulse(500, 292);
        let mm = s.measure_mm().unwrap();
        assert!((mm - 50.0).abs() < 5.0, "got {mm} mm");
    }

    #[test]
    fn missing_module_reports_no_echo() {
        let mut s = sensor_with_pulse(u32::MAX, 0);
        assert_eq!(s.measure_mm(), Err(Error::NoEcho));
    }

    #[test]
    fn endless_echo_reports_out_of_range() {
        let mut s = sensor_with_pulse(500, 10_000_000);
        assert_eq!(s.measure_mm(), Err(Error::OutOfRange));
    }
}
