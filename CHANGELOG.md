# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-11-08

### Added

- Add `Pit`, a passive 8253 timer model with BIOS power-on defaults: count and
  control-word writes, counter latch, lo/hi read flip-flop, channel gates,
  output levels, high time, and IRQ 0 timing through `irq0_edges` and `next_irq0`.
  `Pit::channel` returns a `Copy` `ChannelState` snapshot. Modes 0, 2 and 3 are
  modelled; 6 and 7 alias 2 and 3, and 1, 4 and 5 run as mode 3.

## [0.1.1] - 2026-11-08

### Added

- Derive `Hash` for `IrqRequest`, `DmaDirection` and `DmaRequest`, and `PartialOrd`/`Ord`
  for `IrqLine`, `DmaChannel` and `PortRange`.

### Fixed

- `Tick::from_secs_f64` rounds to the nearest tick, so it round-trips `Tick::as_secs_f64`
  instead of sometimes losing a tick to float error.
- `Tick::as_nanos_f64` converts in one step instead of rounding twice.

## [0.1.0] - 2026-11-08

### Added

- Add a dependency-free, `no_std` shared interface for DOS hardware devices.
- Represent the PC crystal exactly with `Tick` and retain fractional timing
  between CPU-cycle or audio-frame charges with `TickAccumulator`.
- Add inclusive port ranges, timestamped device I/O, autonomous advancement,
  IRQ levels and DMA request descriptors.

[Unreleased]: https://github.com/sunsided/game-toolkit-dosbus/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/sunsided/game-toolkit-dosbus/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/sunsided/game-toolkit-dosbus/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/sunsided/game-toolkit-dosbus/compare/03a2908...v0.1.0
