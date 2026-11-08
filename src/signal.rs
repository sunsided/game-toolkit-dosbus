/// An ISA interrupt request line, IRQ 0 through IRQ 15.
///
/// Includes IRQ 2, the AT cascade line; machine-specific reservations belong to
/// the host's interrupt controller.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IrqLine(u8);

impl IrqLine {
    /// Validate a physical IRQ line number.
    pub const fn new(line: u8) -> Option<Self> {
        if line < 16 { Some(Self(line)) } else { None }
    }

    /// Physical IRQ line number.
    pub const fn number(self) -> u8 {
        self.0
    }
}

/// The current level driven by a device on an IRQ line.
///
/// Repeated observation is not acknowledgement. The host's PIC decides how a
/// level becomes an interrupt; the device's own I/O protocol clears its source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IrqRequest {
    /// The configured interrupt line.
    pub line: IrqLine,
    /// Whether the device is currently asserting the line.
    pub asserted: bool,
}

/// Width of one ISA DMA transfer unit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DmaWidth {
    /// An 8-bit transfer on channels 0 through 3.
    Byte,
    /// A 16-bit transfer on channels 5 through 7.
    Word,
}

/// An ISA DMA data channel: 0 through 3 or 5 through 7.
///
/// Channel 4 cascades the two AT DMA controllers and cannot transfer device data.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DmaChannel(u8);

impl DmaChannel {
    /// Validate a DMA data channel, excluding the cascade channel.
    pub const fn new(channel: u8) -> Option<Self> {
        match channel {
            0..=3 | 5..=7 => Some(Self(channel)),
            _ => None,
        }
    }

    /// Physical DMA channel number.
    pub const fn number(self) -> u8 {
        self.0
    }

    /// Width implied by the selected channel.
    pub const fn width(self) -> DmaWidth {
        if self.0 < 4 {
            DmaWidth::Byte
        } else {
            DmaWidth::Word
        }
    }
}

/// Direction of a DMA transfer, named from the device's perspective.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DmaDirection {
    /// The controller reads memory and delivers data to the device.
    MemoryToDevice,
    /// The controller reads the device and writes data to memory.
    DeviceToMemory,
}

/// An asserted DMA request (DRQ), without a memory address or transfer count.
///
/// The host's DMA controller owns address, count, page wrapping, masking and
/// terminal-count behavior. This descriptor only reports the device's request;
/// it neither performs nor acknowledges a transfer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DmaRequest {
    /// DMA channel configured for this device.
    pub channel: DmaChannel,
    /// Whether data flows into or out of the device.
    pub direction: DmaDirection,
}
