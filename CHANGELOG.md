# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/sunsided/game-toolkit-dosbus/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/sunsided/game-toolkit-dosbus/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/sunsided/game-toolkit-dosbus/compare/03a2908...v0.1.0
