# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add a dependency-free, `no_std` shared interface for DOS hardware devices.
- Represent the PC crystal exactly with `Tick` and retain fractional timing
  between CPU-cycle or audio-frame charges with `TickAccumulator`.
- Add inclusive port ranges, timestamped device I/O, autonomous advancement,
  IRQ levels and DMA request descriptors.

[Unreleased]: https://github.com/sunsided/game-toolkit-dosbus/compare/03a2908...HEAD
