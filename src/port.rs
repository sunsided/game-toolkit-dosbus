/// A nonempty inclusive range of 16-bit I/O ports.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PortRange {
    start: u16,
    end: u16,
}

impl PortRange {
    /// Construct an inclusive range, usable in static device port maps.
    ///
    /// # Panics
    /// Panics if `start > end`.
    pub const fn new(start: u16, end: u16) -> Self {
        assert!(start <= end, "reversed I/O port range");
        Self { start, end }
    }

    /// Construct a range for exactly one port.
    pub const fn single(port: u16) -> Self {
        Self::new(port, port)
    }

    /// First port in the range, inclusive.
    pub const fn start(self) -> u16 {
        self.start
    }

    /// Last port in the range, inclusive.
    pub const fn end(self) -> u16 {
        self.end
    }

    /// Whether this range includes `port`.
    pub const fn contains(self, port: u16) -> bool {
        self.start <= port && port <= self.end
    }

    /// Whether this range and `other` share any port.
    pub const fn overlaps(self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}
