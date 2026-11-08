//! PIT counting, port protocol and timing, checked against dosaudio's channel 2 behaviour.

use game_toolkit_dosbus::{Channel, Mode, Pit, Tick};

/// Program channel 2 like a speaker driver: control word, then low/high count.
fn program(pit: &mut Pit, at: Tick, mode: u8, count: u16) {
    pit.write(at, 0x43, 0b1011_0000 | (mode << 1));
    let [low, high] = count.to_le_bytes();
    pit.write(at, 0x42, low);
    pit.write(at, 0x42, high);
}

fn clocks(n: u64) -> Tick {
    Tick(n * 12)
}

/// Channel 2 logic of game-toolkit-dosaudio's `SpeakerState` and `SpeakerControl::counter`.
mod reference {
    pub fn output(divisor: u64, mode: u8, epoch: u64, gate: bool, at: u64) -> bool {
        if !gate {
            return true;
        }
        let phase = at.saturating_sub(epoch) / 12 % divisor;
        let high = if mode == 2 {
            divisor - 1
        } else {
            divisor.div_ceil(2)
        };
        phase < high
    }

    pub fn counter(d: u64, mode: u8, epoch: u64, gate: bool, at: u64) -> u16 {
        let elapsed = if gate {
            at.saturating_sub(epoch) / 12
        } else {
            0
        };
        let phase = elapsed % d;
        let count = if mode == 3 {
            let high = d.div_ceil(2);
            if phase == 0 {
                d
            } else if phase < high {
                d + d % 2 - 2 * phase
            } else {
                d - d % 2 - 2 * (phase - high)
            }
        } else {
            d - phase
        };
        count as u16
    }

    fn high_ticks(divisor: u64, mode: u8, elapsed: f64) -> f64 {
        let x = elapsed.max(0.0) / 12.0;
        let d = divisor as f64;
        let high = if mode == 2 { d - 1.0 } else { (d / 2.0).ceil() };
        ((x / d).floor() * high + (x % d).min(high)) * 12.0
    }

    pub fn integral(divisor: u64, mode: u8, epoch: u64, start: f64, end: f64) -> f64 {
        high_ticks(divisor, mode, end - epoch as f64)
            - high_ticks(divisor, mode, start - epoch as f64)
    }
}

#[test]
fn power_on_matches_bios() {
    let pit = Pit::new();
    let expect = [
        (Channel::Ch0, 65_536, Mode::SquareWave, true),
        (Channel::Ch1, 18, Mode::RateGenerator, true),
        (Channel::Ch2, 65_536, Mode::SquareWave, false),
    ];
    for (channel, divisor, mode, gate) in expect {
        let state = pit.channel(channel);
        assert_eq!(
            (state.divisor(), state.mode(), state.gate(), state.epoch()),
            (divisor, mode, gate, Tick::ZERO)
        );
    }
    assert_eq!(Pit::default(), pit);
}

#[test]
fn channel_two_matches_dosaudio_bit_exactly() {
    for mode in [2, 3] {
        for divisor in 1..=21u16 {
            let epoch = 7;
            let mut pit = Pit::new();
            pit.set_gate(Channel::Ch2, Tick(0), true);
            program(&mut pit, Tick(epoch), mode, divisor);
            let d = u64::from(divisor);
            for at in 0..epoch + 12 * 3 * d + 30 {
                let t = Tick(at);
                assert_eq!(
                    pit.output(Channel::Ch2, t),
                    reference::output(d, mode, epoch, true, at),
                    "output mode {mode} d {divisor} at {at}"
                );
                assert_eq!(
                    pit.counter(Channel::Ch2, t),
                    reference::counter(d, mode, epoch, true, at),
                    "counter mode {mode} d {divisor} at {at}"
                );
                let high = pit.high_time(Channel::Ch2, Tick(epoch), t);
                let expected = reference::integral(d, mode, epoch, epoch as f64, at as f64);
                // The reference integrates in f64 and carries rounding noise; high_time is exact.
                assert!(
                    (high as f64 - expected).abs() < 1e-6,
                    "high_time mode {mode} d {divisor} at {at}: {high} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn square_wave_count_sequences() {
    let mut pit = Pit::new();
    pit.set_gate(Channel::Ch2, Tick::ZERO, true);
    program(&mut pit, Tick::ZERO, 3, 5);
    let seq: Vec<_> = (0..6)
        .map(|n| pit.counter(Channel::Ch2, clocks(n)))
        .collect();
    assert_eq!(seq, [5, 4, 2, 4, 2, 5]);
    let out: Vec<_> = (0..5)
        .map(|n| pit.output(Channel::Ch2, clocks(n)))
        .collect();
    assert_eq!(out, [true, true, true, false, false]);

    program(&mut pit, Tick::ZERO, 3, 4);
    let seq: Vec<_> = (0..4)
        .map(|n| pit.counter(Channel::Ch2, clocks(n)))
        .collect();
    assert_eq!(seq, [4, 2, 4, 2]);
}

#[test]
fn rate_generator_is_high_for_all_but_one_clock() {
    let mut pit = Pit::new();
    pit.set_gate(Channel::Ch2, Tick::ZERO, true);
    program(&mut pit, Tick::ZERO, 2, 4);
    let seq: Vec<_> = (0..5)
        .map(|n| pit.counter(Channel::Ch2, clocks(n)))
        .collect();
    assert_eq!(seq, [4, 3, 2, 1, 4]);
    let out: Vec<_> = (0..5)
        .map(|n| pit.output(Channel::Ch2, clocks(n)))
        .collect();
    assert_eq!(out, [true, true, true, false, true]);
}

#[test]
fn zero_count_means_65536() {
    let mut pit = Pit::new();
    program(&mut pit, Tick(100), 3, 0);
    assert_eq!(pit.channel(Channel::Ch2).divisor(), 65_536);
    assert_eq!(pit.counter(Channel::Ch2, Tick(100)), 0);
}

#[test]
fn full_count_write_resets_epoch() {
    let mut pit = Pit::new();
    pit.write(Tick(10), 0x43, 0xb6);
    pit.write(Tick(20), 0x42, 0x34);
    assert_eq!(pit.channel(Channel::Ch2).epoch(), Tick::ZERO);
    pit.write(Tick(30), 0x42, 0x12);
    let state = pit.channel(Channel::Ch2);
    assert_eq!((state.divisor(), state.epoch()), (0x1234, Tick(30)));
}

#[test]
fn gate_freezes_counter_and_rising_edge_restarts() {
    let mut pit = Pit::new();
    program(&mut pit, Tick::ZERO, 3, 10);
    assert_eq!(pit.counter(Channel::Ch2, clocks(13)), 10);
    assert!(pit.output(Channel::Ch2, clocks(17)));
    assert_eq!(pit.high_time(Channel::Ch2, Tick(5), Tick(50)), 45);

    pit.set_gate(Channel::Ch2, Tick(1_000), true);
    assert_eq!(pit.channel(Channel::Ch2).epoch(), Tick(1_000));
    pit.set_gate(Channel::Ch2, Tick(2_000), true);
    assert_eq!(
        pit.channel(Channel::Ch2).epoch(),
        Tick(1_000),
        "level, not edge"
    );
    pit.set_gate(Channel::Ch2, Tick(3_000), false);
    assert_eq!(
        pit.channel(Channel::Ch2).next_rising_edge(Tick(3_000)),
        None
    );
}

#[test]
fn control_word_modes_map() {
    let expect = [
        (0, Mode::TerminalCount),
        (1, Mode::SquareWave),
        (2, Mode::RateGenerator),
        (3, Mode::SquareWave),
        (4, Mode::SquareWave),
        (5, Mode::SquareWave),
        (6, Mode::RateGenerator),
        (7, Mode::SquareWave),
    ];
    let mut pit = Pit::new();
    for (bits, mode) in expect {
        pit.write(Tick::ZERO, 0x43, 0b1011_0000 | (bits << 1));
        assert_eq!(pit.channel(Channel::Ch2).mode(), mode, "mode bits {bits}");
    }
}

#[test]
fn latch_holds_until_fully_read() {
    let mut pit = Pit::new();
    pit.set_gate(Channel::Ch2, Tick::ZERO, true);
    program(&mut pit, Tick::ZERO, 2, 0x1234);
    pit.write(clocks(1), 0x43, 0x80);
    pit.write(clocks(5), 0x43, 0x80); // ignored while a latch is pending
    assert_eq!(pit.read(clocks(9), 0x42), 0x33);
    assert_eq!(pit.read(clocks(9), 0x42), 0x12);
    // Unlatched reads follow the live count.
    assert_eq!(pit.read(clocks(0x10), 0x42), 0x24);
    assert_eq!(pit.read(clocks(0x10), 0x42), 0x12);
}

#[test]
fn single_byte_access_modes() {
    let mut pit = Pit::new();
    pit.write(Tick::ZERO, 0x43, 0b1001_0110); // ch2, low byte only
    pit.write(Tick::ZERO, 0x42, 0x40);
    assert_eq!(pit.channel(Channel::Ch2).divisor(), 0x40);
    assert_eq!(pit.read(Tick::ZERO, 0x42), 0x40);
    assert_eq!(pit.read(Tick::ZERO, 0x42), 0x40);

    pit.write(Tick::ZERO, 0x43, 0b1010_0110); // ch2, high byte only
    pit.write(Tick::ZERO, 0x42, 0x02);
    assert_eq!(pit.channel(Channel::Ch2).divisor(), 0x200);
    assert_eq!(pit.read(Tick::ZERO, 0x42), 0x02);
}

#[test]
fn channels_are_independent_and_other_ports_ignored() {
    let mut pit = Pit::new();
    pit.write(Tick::ZERO, 0x43, 0x34); // ch0, lo/hi, mode 2
    pit.write(Tick::ZERO, 0x40, 0x9c);
    pit.write(Tick::ZERO, 0x40, 0x2e);
    assert_eq!(pit.channel(Channel::Ch0).divisor(), 0x2e9c);
    assert_eq!(pit.channel(Channel::Ch0).mode(), Mode::RateGenerator);
    assert_eq!(pit.channel(Channel::Ch2), Pit::new().channel(Channel::Ch2));

    let before = pit.clone();
    pit.write(Tick::ZERO, 0x44, 0);
    pit.write(Tick::ZERO, 0x43, 0xc0); // 8254 read-back
    assert_eq!(pit, before);
    assert_eq!(pit.read(Tick::ZERO, 0x43), 0xff);
    assert_eq!(pit.read(Tick::ZERO, 0x61), 0xff);
}

#[test]
fn irq0_at_power_on_is_18_2_hz() {
    let pit = Pit::new();
    let period = Tick(12 * 65_536);
    assert_eq!(pit.next_irq0(Tick::ZERO), Some(period));
    assert_eq!(pit.next_irq0(period), Some(Tick(2 * period.0)));
    assert_eq!(pit.irq0_edges(Tick::ZERO, period), 1);
    assert_eq!(pit.irq0_edges(Tick::ZERO, Tick(period.0 - 1)), 0);
    assert_eq!(pit.irq0_edges(Tick::ZERO, Tick::from_micros(1_000_000)), 18);
}

#[test]
fn edges_match_output_transitions() {
    for (mode, divisor) in [(2, 5), (3, 5), (3, 4), (2, 1), (3, 1), (0, 6)] {
        let mut pit = Pit::new();
        pit.write(Tick::ZERO, 0x43, 0b0011_0000 | (mode << 1));
        pit.write(Tick(3), 0x40, divisor);
        pit.write(Tick(3), 0x40, 0);
        let state = pit.channel(Channel::Ch0);
        let mut edges = 0;
        let mut next = state.next_rising_edge(Tick::ZERO);
        for at in 1..400 {
            if !state.output(Tick(at - 1)) && state.output(Tick(at)) {
                edges += 1;
                assert_eq!(next, Some(Tick(at)), "mode {mode} d {divisor}");
                next = state.next_rising_edge(Tick(at));
            }
            assert_eq!(
                state.rising_edges(Tick::ZERO, Tick(at)),
                edges,
                "mode {mode} d {divisor} at {at}"
            );
        }
        assert!(next.is_none_or(|t| t.0 >= 400));
    }
}

#[test]
fn terminal_count_fires_once_and_counter_wraps() {
    let mut pit = Pit::new();
    pit.write(Tick::ZERO, 0x43, 0x30); // ch0, lo/hi, mode 0
    pit.write(Tick::ZERO, 0x40, 3);
    pit.write(Tick::ZERO, 0x40, 0);
    assert!(!pit.output(Channel::Ch0, clocks(2)));
    assert!(pit.output(Channel::Ch0, clocks(3)));
    assert!(pit.output(Channel::Ch0, clocks(1_000)));
    assert_eq!(pit.counter(Channel::Ch0, clocks(4)), 0xffff);
    assert_eq!(pit.next_irq0(Tick::ZERO), Some(clocks(3)));
    assert_eq!(pit.next_irq0(clocks(3)), None);
    assert_eq!(pit.high_time(Channel::Ch0, Tick::ZERO, clocks(10)), 7 * 12);
}

#[test]
fn refresh_toggle_matches_dosaudio() {
    let pit = Pit::new();
    for at in (0..5_000).step_by(7) {
        let toggle = pit.channel(Channel::Ch1).rising_edges(Tick::ZERO, Tick(at)) & 1;
        assert_eq!(toggle, (at / 216) & 1, "at {at}");
    }
}
