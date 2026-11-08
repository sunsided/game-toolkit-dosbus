# game-toolkit-dosbus

Shared clock and I/O device interfaces for
[game-toolkit-dosvideo](https://github.com/sunsided/game-toolkit-dosvideo) and
`game-toolkit-dosaudio`. Other emulators and frontends can use the same interfaces.

The crate is `no_std`, forbids unsafe code, and has no dependencies or allocator
requirement. It provides time, port ranges and hardware request descriptors. The
host supplies bus routing, access costs, interrupt delivery and DMA transfers.

```rust
use game_toolkit_dosbus::{Tick, TickAccumulator};

let mut now = Tick::ZERO;
let mut cpu = TickAccumulator::new(33_000_000);
now += cpu.advance(100);
now += cpu.advance(200);
assert_eq!(now, Tick(130));
```

`Tick` counts cycles of the PC crystal at exactly 315/22 MHz. Integer conversions
round down. `TickAccumulator` retains the fractional ticks between calls, so
charging CPU cycles individually gives the same result as charging them together.
Use a separate accumulator for each source clock, including audio sample rates or
nanosecond bus charges. Ordinary tick arithmetic panics on overflow in every
build profile; checked methods return `None`.

Devices implement `IoDevice`: a stable list of inclusive `PortRange`s, timestamped
`outb` and `inb`, and optional autonomous advancement and IRQ/DMA observations.
Devices synchronize before changing state. The host supplies nondecreasing
timestamps, charges time before dispatch, and advances devices during idle periods.
Accesses at the same timestamp take effect in call order.

`IrqRequest` reports the device's configured line and current level, including
deassertion. Observing a request never acknowledges it. `DmaRequest` reports an
asserted DRQ and its direction; the DMA controller owns addressing, counts and
page wrapping. `DmaChannel` validates data channels and derives byte/word width;
channel 4 is reserved for cascade.

`Pit` is the one device model in the crate: a passive 8253 timer, because the PIT
is the crystal divided by 12 and every DOS machine needs it. It is a value type,
not an `IoDevice`. The host routes ports 40h-43h to it, drives channel 2's gate
from port 61h bit 0, and reads outputs, IRQ 0 timing (`next_irq0`, `irq0_edges`)
and high time from it. `Pit::channel` returns a `Copy` snapshot of one channel
for use on another thread, such as speaker synthesis. Keep exactly one `Pit` per
machine: if a sound device embeds it, route 40h-43h only to that device. Modes 0,
2 and 3 are modelled; 1, 4 and 5 currently run as mode 3.

This initial crate establishes the shared interfaces. The companion crates still
need to adopt them; dosvideo's existing clock and speaker are unchanged.

Requires Rust 1.95 or later. Run `task test` and `task check`, or use Cargo directly:

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo doc --no-deps --locked
```

Licensed under the [EUPL-1.2](LICENSE).
