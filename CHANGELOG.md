# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-16

### Added

- **Dashboard** — live overview: connection state, server role and build, the dynamic model list, and per-model decode slots.
- **Slots** — one panel per loaded model with live processing state, context size, prompt-token counters, and next-token details; idle `n_remain = -1` decodes as `unlimited`, and slots/metrics are only polled for models the server reports as `loaded`.
- **Performance** — per-model metrics parsed from the server's Prometheus endpoint (prompt/generation throughput, active/deferred requests, token totals, KV-cache), plus time-series history charts over selectable windows.
- **Hardware** — NVIDIA GPU telemetry via `nvidia-smi`: utilization, VRAM used/total, temperature, and power draw/limit.
- **Settings** — server URL, the four polling intervals, and history retention, validated and applied live without a restart.
- SQLite-backed local history, readable even while the server is offline.
- System tray icon with a live status line, a menu, and close-to-tray behaviour.
- Frontend test suite (Vitest) alongside the Rust unit and integration tests.
- Build, install, and uninstall scripts plus a `Makefile` for repeatable local packaging.
- Documentation: README, contribution guide, and security policy.

[Unreleased]: https://github.com/GlennLab/LlamaDesk/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/GlennLab/LlamaDesk/releases/tag/v0.1.0
