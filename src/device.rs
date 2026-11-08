use crate::{DmaRequest, IrqRequest, PortRange, Tick};

/// A device on the host's timestamped, byte-wide I/O bus.
///
/// The host charges bus time before calling a device and dispatches only ports
/// advertised by [`Self::ports`]. Devices synchronize to `at` before observing
/// or changing hardware state, so a write cannot affect earlier output.
///
/// Timestamps supplied to a device must be nondecreasing across all calls;
/// multiple accesses at the same timestamp take effect in call order. The host
/// owns this invariant. Implementations may reject a backwards timestamp.
///
/// Hosts call [`Self::advance_to`] when time passes without port I/O, so DMA,
/// timers and other externally observable state can progress. Request queries
/// observe state at `at` without acknowledging it. A device may calculate that
/// state analytically; otherwise the host must first call `advance_to(at)`.
///
/// Port decoding, overlapping port arbitration (such as the PIT control port),
/// unmapped read values and bus costs belong to the host. The trait does not
/// allocate or require an allocator, but implementations may use one.
pub trait IoDevice: Send {
    /// Ports decoded by this device. Keep this map stable while attached.
    fn ports(&self) -> &[PortRange];

    /// Progress autonomous hardware activity up to `at` without accessing a port.
    ///
    /// The default is suitable for devices with no autonomous activity.
    fn advance_to(&mut self, _at: Tick) {}

    /// Write a byte after synchronizing hardware state to `at`.
    fn outb(&mut self, at: Tick, port: u16, value: u8);

    /// Read a byte after synchronizing hardware state to `at`.
    fn inb(&mut self, at: Tick, port: u16) -> u8;

    /// Current IRQ line and level, or `None` if the device has no IRQ output.
    ///
    /// A device with an IRQ output returns `Some` with `asserted: false` when
    /// deasserted, so a host can clear its contribution to a shared line.
    fn irq_request(&self, _at: Tick) -> Option<IrqRequest> {
        None
    }

    /// Whether this device is asserting its IRQ output at `at`.
    fn irq_pending(&self, at: Tick) -> bool {
        self.irq_request(at).is_some_and(|request| request.asserted)
    }

    /// Current asserted DRQ, or `None` when no DMA transfer is requested.
    ///
    /// The descriptor carries no transfer implementation. The host and its
    /// concrete DMA-capable devices agree on how to exchange transfer units.
    fn dma_request(&self, _at: Tick) -> Option<DmaRequest> {
        None
    }
}
