//! Exact conversion, accumulated phase and overflow checks through the public interface.

use core::time::Duration;
use game_toolkit_dosbus::{Tick, TickAccumulator};

#[test]
fn crystal_period_and_integer_conversions_are_exact() {
    assert_eq!(
        Tick::from_duration(Duration::from_secs(22)),
        Tick(315_000_000)
    );
    assert_eq!(Tick(315_000_000).as_duration(), Duration::from_secs(22));
    assert_eq!(Tick::from_micros(1), Tick(14));
    assert_eq!(Tick::from_micros(80), Tick(1_145));
    assert_eq!(Tick::from_nanos(69), Tick::ZERO);
    assert_eq!(Tick::from_nanos(70), Tick(1));
    assert_eq!(Tick(1).as_duration(), Duration::from_nanos(69));
    assert_eq!(Tick::from_secs_f64(22.0), Tick(315_000_000));
    assert_eq!(Tick(315_000_000).as_secs_f64(), 22.0);
    assert_eq!(Tick(315_000_000).as_nanos_f64(), 22_000_000_000.0);
}

#[test]
fn duration_conversion_handles_the_representable_limits() {
    let last_duration = Tick::MAX.as_duration();
    // Rounding a tick to nanoseconds and back can lose one tick.
    assert_eq!(Tick::from_duration(last_duration), Tick(u64::MAX - 1));
    assert_eq!(
        Tick::checked_from_duration(last_duration + Duration::from_nanos(70)),
        Some(Tick::MAX)
    );
    assert_eq!(
        Tick::checked_from_duration(last_duration + Duration::from_nanos(140)),
        None
    );
    assert_eq!(Tick::checked_from_duration(Duration::MAX), None);
    assert!(Tick::from_nanos(u64::MAX) < Tick::MAX);
}

#[test]
fn repeated_cpu_and_sample_charges_do_not_drift() {
    for hz in [1, 7, 48_000, 33_000_000, 1_000_000_000, u64::MAX] {
        let mut split = TickAccumulator::new(hz);
        let mut total = Tick::ZERO;
        for _ in 0..10_000 {
            total += split.advance(1);
        }
        let mut whole = TickAccumulator::new(hz);
        assert_eq!(total, whole.advance(10_000), "source rate {hz}");
        // The fractional phase must also be independent of chunking.
        assert_eq!(split, whole);
        assert_eq!(split.advance(0), Tick::ZERO);
    }
}

#[test]
fn source_clock_is_exact_after_twenty_two_seconds() {
    let mut cpu = TickAccumulator::new(33_000_000);
    let mut audio = TickAccumulator::new(48_000);
    assert_eq!(cpu.advance(33_000_000 * 22), Tick(315_000_000));
    assert_eq!(audio.advance(48_000 * 22), Tick(315_000_000));
    assert_eq!(cpu.advance(33_000_000 * 22), Tick(315_000_000));
}

#[test]
fn fractional_charges_survive_until_a_whole_tick_is_due() {
    let mut cpu = TickAccumulator::new(33_000_000);
    assert_eq!(cpu.advance(1), Tick::ZERO);
    assert_eq!(cpu.advance(1), Tick::ZERO);
    assert_eq!(cpu.advance(1), Tick(1));
    assert_eq!(cpu.source_hz(), 33_000_000);
}

#[test]
fn accumulator_overflow_preserves_its_fractional_phase() {
    let mut clock = TickAccumulator::new(1);
    clock.advance(1);
    let before = clock.clone();
    assert_eq!(clock.checked_advance(u64::MAX), None);
    assert_eq!(clock, before);
    assert_eq!(clock.advance(21), Tick(300_681_819));
    // Even the largest source rate and source charge fit the intermediate math.
    let mut slow_ticks = TickAccumulator::new(u64::MAX);
    assert_eq!(slow_ticks.advance(u64::MAX), Tick(14_318_181));
}

#[test]
fn checked_arithmetic_never_wraps() {
    assert_eq!(Tick::MAX.checked_add(Tick(1)), None);
    assert_eq!(Tick::ZERO.checked_sub(Tick(1)), None);
    assert_eq!(Tick(10).checked_sub(Tick(10)), Some(Tick::ZERO));
    let mut now = Tick(10);
    now += Tick(20);
    now -= Tick(5);
    assert_eq!(now, Tick(25));
}

#[test]
#[should_panic(expected = "tick addition overflow")]
fn addition_overflow_panics_in_release_too() {
    let _ = Tick::MAX + Tick(1);
}

#[test]
#[should_panic(expected = "tick subtraction below zero")]
fn backwards_subtraction_panics_in_release_too() {
    let _ = Tick::ZERO - Tick(1);
}

#[test]
#[should_panic(expected = "source frequency must be nonzero")]
fn zero_source_frequency_is_rejected() {
    let _ = TickAccumulator::new(0);
}

#[test]
fn invalid_floating_point_times_are_rejected() {
    for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        assert!(std::panic::catch_unwind(|| Tick::from_secs_f64(value)).is_err());
    }
}
