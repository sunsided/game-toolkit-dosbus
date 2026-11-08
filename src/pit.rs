use crate::Tick;

/// Crystal ticks per PIT input clock (1.193182 MHz).
const TICKS_PER_CLOCK: u64 = 12;

/// One of the three 8253 counters.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Channel {
    /// System timer; its output drives IRQ 0 (INT 8, 18.2 Hz at power-on).
    Ch0,
    /// DRAM refresh; its output toggles port 61h bit 4.
    Ch1,
    /// PC speaker; gated by port 61h bit 0, output readable as port 61h bit 5.
    Ch2,
}

/// Counting mode selected by a control word.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Mode {
    /// Mode 0: output low until the count expires, then high until reprogrammed.
    TerminalCount,
    /// Mode 2 (also selected by 6): output high for `divisor - 1` of every `divisor` clocks.
    RateGenerator,
    /// Mode 3 (also selected by 7): output high for `ceil(divisor / 2)` of every `divisor` clocks.
    SquareWave,
}

impl Mode {
    fn from_control(bits: u8) -> Self {
        match bits {
            0 => Self::TerminalCount,
            2 | 6 => Self::RateGenerator,
            // shortcut: modes 1, 4 and 5 run as mode 3; model them when software needs one-shots.
            _ => Self::SquareWave,
        }
    }
}

/// A snapshot of one channel's waveform: everything needed to evaluate its
/// output and count at any time until the channel is next reprogrammed.
///
/// The state is `Copy` and `Send`, so a host can hand it to another thread,
/// for example to integrate the speaker waveform per audio sample.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ChannelState {
    divisor: u32,
    mode: Mode,
    epoch: Tick,
    gate: bool,
}

impl ChannelState {
    /// Input clocks per period, 1 through 65536; a written count of 0 means 65536.
    pub const fn divisor(self) -> u32 {
        self.divisor
    }

    /// The counting mode.
    pub const fn mode(self) -> Mode {
        self.mode
    }

    /// When counting started: the last full count write or rising gate edge.
    pub const fn epoch(self) -> Tick {
        self.epoch
    }

    /// The gate input. With the gate low the counter is frozen at its reload value.
    pub const fn gate(self) -> bool {
        self.gate
    }

    /// Whole input clocks counted at `at`; zero before the epoch or with the gate low.
    fn elapsed(self, at: Tick) -> u64 {
        if self.gate {
            at.0.saturating_sub(self.epoch.0) / TICKS_PER_CLOCK
        } else {
            0
        }
    }

    /// Input clocks per period with the output high, for the periodic modes.
    fn high(self) -> u64 {
        let d = u64::from(self.divisor);
        match self.mode {
            Mode::RateGenerator => d - 1,
            _ => d.div_ceil(2),
        }
    }

    /// Whether the periodic waveform has rising edges at all (excludes divisor 1).
    fn toggles(self) -> bool {
        (1..u64::from(self.divisor)).contains(&self.high())
    }

    /// The count a read would return at `at`, without latching.
    ///
    /// Mode 3 decrements by two and reloads at each half period; a reload
    /// reads as the divisor. A divisor of 65536 reads as 0.
    pub fn counter(self, at: Tick) -> u16 {
        let d = u64::from(self.divisor);
        let elapsed = self.elapsed(at);
        let count = match self.mode {
            // Mode 0 keeps decrementing after terminal count, wrapping through 0xffff.
            Mode::TerminalCount => return (d as u16).wrapping_sub(elapsed as u16),
            Mode::RateGenerator => d - elapsed % d,
            Mode::SquareWave => {
                let phase = elapsed % d;
                let high = self.high();
                if phase == 0 {
                    d
                } else if phase < high {
                    d + d % 2 - 2 * phase
                } else {
                    d - d % 2 - 2 * (phase - high)
                }
            }
        };
        count as u16
    }

    /// The output level at `at`. Modes 2 and 3 read high while the gate is low.
    pub fn output(self, at: Tick) -> bool {
        match self.mode {
            Mode::TerminalCount => self.elapsed(at) >= u64::from(self.divisor),
            _ if !self.gate => true,
            _ => self.elapsed(at) % u64::from(self.divisor) < self.high(),
        }
    }

    /// Ticks in `from..to` during which the output is high; zero if `to <= from`.
    pub fn high_time(self, from: Tick, to: Tick) -> u64 {
        self.high_before(to).saturating_sub(self.high_before(from))
    }

    /// High ticks in `0..t`. Monotonic in `t`.
    fn high_before(self, t: Tick) -> u64 {
        if !self.gate {
            return if self.output(t) { t.0 } else { 0 };
        }
        let period = TICKS_PER_CLOCK * u64::from(self.divisor);
        let since = t.0.saturating_sub(self.epoch.0);
        match self.mode {
            // Low from the epoch until terminal count, and at elapsed 0 before it.
            Mode::TerminalCount => since.saturating_sub(period),
            _ => {
                let high = TICKS_PER_CLOCK * self.high();
                // Before the epoch the waveform sits at phase 0, which is high unless high == 0.
                let before = if high > 0 { t.0.min(self.epoch.0) } else { 0 };
                before + since / period * high + (since % period).min(high)
            }
        }
    }

    /// Rising output edges at times in `from+1..=to` since the epoch.
    ///
    /// Only edges of the current programming count; query before reprogramming.
    pub fn rising_edges(self, from: Tick, to: Tick) -> u64 {
        self.edges_through(to)
            .saturating_sub(self.edges_through(from))
    }

    /// Rising edges at times `<= t` since the epoch.
    fn edges_through(self, t: Tick) -> u64 {
        let d = u64::from(self.divisor);
        match self.mode {
            Mode::TerminalCount => u64::from(self.elapsed(t) >= d),
            _ if self.toggles() => self.elapsed(t) / d,
            _ => 0,
        }
    }

    /// The first rising output edge strictly after `after`, if any.
    ///
    /// `None` with the gate low, after mode 0's single edge, for a periodic
    /// divisor of 1, or if the edge lies beyond [`Tick::MAX`].
    pub fn next_rising_edge(self, after: Tick) -> Option<Tick> {
        if !self.gate || !(self.mode == Mode::TerminalCount || self.toggles()) {
            return None;
        }
        let period = TICKS_PER_CLOCK * u64::from(self.divisor);
        let k = self.edges_through(after) + 1;
        if self.mode == Mode::TerminalCount && k > 1 {
            return None;
        }
        self.epoch.0.checked_add(period.checked_mul(k)?).map(Tick)
    }
}

/// Port-protocol state of one counter around its waveform.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter {
    state: ChannelState,
    /// Control word access bits: 1 = low byte, 2 = high byte, 3 = low then high.
    access: u8,
    /// Low byte of a pending low/high count write.
    low: Option<u8>,
    /// Read flip-flop: the next low/high read returns the high byte.
    read_high: bool,
    latched: Option<u16>,
}

impl Counter {
    const fn new(divisor: u32, mode: Mode, gate: bool, access: u8) -> Self {
        Self {
            state: ChannelState {
                divisor,
                mode,
                epoch: Tick::ZERO,
                gate,
            },
            access,
            low: None,
            read_high: false,
            latched: None,
        }
    }

    fn write_count(&mut self, at: Tick, value: u8) {
        let count = match self.access {
            1 => u16::from(value),
            2 => u16::from(value) << 8,
            _ => match self.low.take() {
                None => {
                    self.low = Some(value);
                    return;
                }
                Some(low) => u16::from_le_bytes([low, value]),
            },
        };
        self.state.divisor = if count == 0 { 65_536 } else { u32::from(count) };
        self.state.epoch = at;
    }

    fn read_count(&mut self, at: Tick) -> u8 {
        let count = self.latched.unwrap_or_else(|| self.state.counter(at));
        let [low, high] = count.to_le_bytes();
        let value = if self.access == 2 || (self.access == 3 && self.read_high) {
            high
        } else {
            low
        };
        // A latch holds until every byte of the access mode has been read.
        if self.access != 3 || self.read_high {
            self.latched = None;
        }
        if self.access == 3 {
            self.read_high = !self.read_high;
        }
        value
    }
}

/// A passive 8253 programmable interval timer: ports 40h-43h, input clock = crystal / 12.
///
/// This is the crate's one device model, kept here because the PIT is part of
/// the machine clock. It is a value type, not an [`IoDevice`](crate::IoDevice):
/// the host routes ports 40h-43h to [`Pit::write`] and [`Pit::read`], drives
/// channel 2's gate from port 61h bit 0, and polls outputs and IRQ 0 timing.
///
/// Keep exactly one `Pit` per machine. If a sound device embeds the PIT for
/// the speaker, the host must route 40h-43h only to that device and read timer
/// state from it; two copies that both see control writes to 43h drift apart.
///
/// The power-on state matches a BIOS after POST: channel 0 divisor 65536 in
/// mode 3, channel 1 divisor 18 in mode 2, channel 2 divisor 65536 in mode 3
/// with its gate low. Modes 0, 2 and 3 are modelled; BCD counting is ignored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pit {
    channels: [Counter; 3],
}

impl Default for Pit {
    fn default() -> Self {
        Self::new()
    }
}

impl Pit {
    /// The power-on state; see the type documentation.
    pub const fn new() -> Self {
        Self {
            channels: [
                Counter::new(65_536, Mode::SquareWave, true, 3),
                Counter::new(18, Mode::RateGenerator, true, 1),
                Counter::new(65_536, Mode::SquareWave, false, 3),
            ],
        }
    }

    /// Handle a write to port 40h-43h, including the counter latch command. Other ports are ignored.
    pub fn write(&mut self, at: Tick, port: u16, value: u8) {
        match port {
            0x40..=0x42 => self.channels[usize::from(port - 0x40)].write_count(at, value),
            0x43 => {
                // Select 3 is the 8254 read-back command, which an 8253 ignores.
                let Some(counter) = self.channels.get_mut(usize::from(value >> 6)) else {
                    return;
                };
                let access = (value >> 4) & 3;
                if access == 0 {
                    if counter.latched.is_none() {
                        counter.latched = Some(counter.state.counter(at));
                    }
                    return;
                }
                counter.state.mode = Mode::from_control((value >> 1) & 7);
                counter.access = access;
                counter.low = None;
                counter.read_high = false;
                counter.latched = None;
            }
            _ => {}
        }
    }

    /// Handle a read of port 40h-42h. Port 43h and other ports read 0xff.
    pub fn read(&mut self, at: Tick, port: u16) -> u8 {
        match port {
            0x40..=0x42 => self.channels[usize::from(port - 0x40)].read_count(at),
            _ => 0xff,
        }
    }

    /// Set a channel's gate input. A rising edge restarts counting from the reload value.
    pub fn set_gate(&mut self, channel: Channel, at: Tick, gate: bool) {
        let state = &mut self.channels[channel as usize].state;
        if gate && !state.gate {
            state.epoch = at;
        }
        state.gate = gate;
    }

    /// A snapshot of a channel's waveform, valid until the channel is reprogrammed.
    pub fn channel(&self, channel: Channel) -> ChannelState {
        self.channels[channel as usize].state
    }

    /// The channel's current count, without latching or moving the read flip-flop.
    pub fn counter(&self, channel: Channel, at: Tick) -> u16 {
        self.channel(channel).counter(at)
    }

    /// The channel's output level.
    pub fn output(&self, channel: Channel, at: Tick) -> bool {
        self.channel(channel).output(at)
    }

    /// Ticks in `from..to` during which the channel's output is high.
    pub fn high_time(&self, channel: Channel, from: Tick, to: Tick) -> u64 {
        self.channel(channel).high_time(from, to)
    }

    /// IRQ 0 requests (channel 0 rising edges) at times in `from+1..=to`.
    ///
    /// Counts only edges since channel 0 was last programmed; call before reprogramming.
    pub fn irq0_edges(&self, from: Tick, to: Tick) -> u64 {
        self.channel(Channel::Ch0).rising_edges(from, to)
    }

    /// The next IRQ 0 request strictly after `after`, if any.
    pub fn next_irq0(&self, after: Tick) -> Option<Tick> {
        self.channel(Channel::Ch0).next_rising_edge(after)
    }
}
