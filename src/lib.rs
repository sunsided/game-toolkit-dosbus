//! Shared time and I/O contracts for DOS hardware emulation.
//!
//! The host owns machine time and passes [`Tick`] timestamps to [`IoDevice`]s.
//! Video, sound and other devices can share this interface without depending on
//! one another. There is no allocator, host I/O or global clock, and no device
//! emulation except [`Pit`]: the 8253 timer is part of the machine clock, so it
//! lives here as a passive value type that the host routes ports to.
//!
//! The PC crystal is represented exactly as 315/22 MHz. [`TickAccumulator`]
//! converts repeated CPU-cycle or audio-frame charges without rounding drift.
//!
//! ```
//! use game_toolkit_dosbus::{Tick, TickAccumulator};
//!
//! let mut now = Tick::ZERO;
//! let mut cpu = TickAccumulator::new(33_000_000);
//! now += cpu.advance(100);
//! now += cpu.advance(200);
//! assert_eq!(now, Tick(130));
//! ```
//!
//! [`IoDevice`] is object safe and requires `Send`, allowing the host to own
//! devices directly or through trait objects. The interface does not prescribe
//! threading, port routing, bus access costs, interrupt delivery or DMA memory.

#![no_std]
#![forbid(unsafe_code)]

mod clock;
mod device;
mod pit;
mod port;
mod signal;

pub use clock::{Tick, TickAccumulator};
pub use device::IoDevice;
pub use pit::{Channel, ChannelState, Mode, Pit};
pub use port::PortRange;
pub use signal::{DmaChannel, DmaDirection, DmaRequest, DmaWidth, IrqLine, IrqRequest};
