use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::time::Duration;

/// An instant or duration in cycles of the PC's 315/22 MHz crystal.
///
/// One cycle is exactly 22/315 microseconds. PIT input cycles span 12 ticks;
/// an OPL native sample spans 288 ticks. Conversions round down to whole ticks.
/// For repeated charges, use [`TickAccumulator`] to retain the fractional part.
///
/// Arithmetic panics on overflow or subtraction below zero in all build profiles.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Tick(pub u64);

impl Tick {
    /// Power-on time, or a zero duration.
    pub const ZERO: Self = Self(0);
    /// Largest representable timestamp or duration.
    pub const MAX: Self = Self(u64::MAX);
    /// Numerator of the crystal frequency in hertz.
    pub const HZ_NUMERATOR: u64 = 315_000_000;
    /// Denominator of the crystal frequency in hertz.
    pub const HZ_DENOMINATOR: u64 = 22;

    /// Convert integer microseconds, rounding down.
    ///
    /// # Panics
    /// Panics if the result exceeds [`Tick::MAX`].
    pub const fn from_micros(micros: u64) -> Self {
        Self::from_duration(Duration::from_micros(micros))
    }

    /// Convert integer nanoseconds, rounding down.
    pub const fn from_nanos(nanos: u64) -> Self {
        Self::from_duration(Duration::from_nanos(nanos))
    }

    /// Convert a duration, rounding down.
    ///
    /// # Panics
    /// Panics if the result exceeds [`Tick::MAX`].
    pub const fn from_duration(duration: Duration) -> Self {
        match Self::checked_from_duration(duration) {
            Some(tick) => tick,
            None => panic!("duration exceeds Tick::MAX"),
        }
    }

    /// Convert a duration, returning `None` on overflow.
    pub const fn checked_from_duration(duration: Duration) -> Option<Self> {
        // Even Duration::MAX multiplied by the crystal numerator fits u128.
        let ticks = duration.as_nanos() * Self::HZ_NUMERATOR as u128
            / (Self::HZ_DENOMINATOR as u128 * 1_000_000_000);
        if ticks > u64::MAX as u128 {
            None
        } else {
            Some(Self(ticks as u64))
        }
    }

    /// Convert approximate floating-point seconds, rounding down.
    ///
    /// Prefer [`Self::from_duration`] when exact integer conversion matters.
    ///
    /// # Panics
    /// Panics for negative, non-finite or out-of-range input.
    pub fn from_secs_f64(seconds: f64) -> Self {
        assert!(seconds.is_finite() && seconds >= 0.0, "invalid seconds");
        let ticks = seconds * Self::HZ_NUMERATOR as f64 / Self::HZ_DENOMINATOR as f64;
        // u64::MAX as f64 rounds up to 2^64, the exclusive upper bound.
        assert!(ticks < u64::MAX as f64, "seconds exceed Tick::MAX");
        Self(ticks as u64)
    }

    /// Convert to a duration, rounding down to whole nanoseconds.
    pub const fn as_duration(self) -> Duration {
        let nanos = self.0 as u128 * Self::HZ_DENOMINATOR as u128 * 1_000_000_000
            / Self::HZ_NUMERATOR as u128;
        Duration::new(
            (nanos / 1_000_000_000) as u64,
            (nanos % 1_000_000_000) as u32,
        )
    }

    /// Approximate seconds, for display or compatibility with older clocks.
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 * Self::HZ_DENOMINATOR as f64 / Self::HZ_NUMERATOR as f64
    }

    /// Approximate nanoseconds, for compatibility with older device interfaces.
    pub fn as_nanos_f64(self) -> f64 {
        self.as_secs_f64() * 1_000_000_000.0
    }

    /// Add a duration, returning `None` on overflow.
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Subtract an instant or duration, returning `None` if `rhs` is greater.
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl Add for Tick {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs).expect("tick addition overflow")
    }
}

impl AddAssign for Tick {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Tick {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs).expect("tick subtraction below zero")
    }
}

impl SubAssign for Tick {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

/// Converts cycles at a fixed integer frequency to crystal ticks without drift.
///
/// Use one accumulator per source clock: a 33 MHz CPU, a 48 kHz audio stream,
/// or nanosecond bus charges (1 GHz). Each call returns the whole ticks elapsed
/// since the previous call and retains the fractional remainder internally.
/// Splitting a charge across any number of calls produces the same total ticks.
/// Creating a new accumulator resets the source clock's fractional phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TickAccumulator {
    source_hz: u64,
    remainder: u128,
}

impl TickAccumulator {
    /// Create a converter for a source clock at `source_hz` hertz.
    ///
    /// # Panics
    /// Panics if the frequency is zero.
    pub const fn new(source_hz: u64) -> Self {
        assert!(source_hz > 0, "source frequency must be nonzero");
        Self {
            source_hz,
            remainder: 0,
        }
    }

    /// The source clock's frequency in hertz.
    pub const fn source_hz(&self) -> u64 {
        self.source_hz
    }

    /// Charge source cycles and return the elapsed whole crystal ticks.
    ///
    /// # Panics
    /// Panics if this call's result exceeds [`Tick::MAX`]. On overflow the
    /// fractional phase remains unchanged.
    pub fn advance(&mut self, cycles: u64) -> Tick {
        self.checked_advance(cycles)
            .expect("cycle charge exceeds Tick::MAX")
    }

    /// Charge source cycles, returning `None` on overflow without changing state.
    pub fn checked_advance(&mut self, cycles: u64) -> Option<Tick> {
        let denominator = self.source_hz as u128 * Tick::HZ_DENOMINATOR as u128;
        let numerator = cycles as u128 * Tick::HZ_NUMERATOR as u128 + self.remainder;
        let ticks = u64::try_from(numerator / denominator).ok()?;
        self.remainder = numerator % denominator;
        Some(Tick(ticks))
    }
}
