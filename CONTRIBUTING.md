# Contributing to LlamaDesk

Thanks for helping improve LlamaDesk. This guide covers the local setup, the quality gates, and the pull-request process.

## Prerequisites

Install the system packages and toolchains listed in the [Requirements](./README.md#requirements) section of the README. In short: a Linux host, Rust 1.90+, Node 22, and the Tauri CLI (`cargo install tauri-cli --version "^2"`).

Clone and install the frontend dependencies:

```bash
git clone https://github.com/GlennLab/LlamaDesk.git
cd LlamaDesk
npm ci
```

## Running the app

```bash
cargo tauri dev
```

This starts the Vite dev server on port 1420 and opens the Tauri window. A live llama.cpp server is **not** required to develop the UI — without one, the app reports `Disconnected` and renders empty states.

The Tauri CLI (`@tauri-apps/cli ^2.12.3`) is pinned in devDependencies and exposed via the root `"tauri"` script, so `npm run tauri dev|build` works alongside `cargo tauri`.

To work on the frontend alone in a browser (no backend, so no live data):

```bash
npm run dev
```

## Quality gates

Every change must pass all of these. Run the Rust gates from the repository root (the Cargo workspace member is `src-tauri`):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                     # 112 tests
```

And the frontend gates at the root:

```bash
npm run test:run               # 163 tests
npm run build                  # tsc type-check + Vite build
```

`make test` runs the backend and frontend suites together, and `make lint` runs clippy plus the frontend build.

### Live-router tests

`src-tauri/tests/live_router.rs` holds **five integration tests** that talk to a real llama.cpp server. They are all marked `#[ignore]`, so a normal `cargo test` (and CI without a server) never runs them and stays green. Run them against a live server with:

```bash
cargo test --test live_router -- --ignored --nocapture
```

They default to `http://localhost:8080`; point them elsewhere with `LLAMADESK_TEST_URL`.

## Code layout

LlamaDesk is layered: pure domain models, provider-agnostic ports (traits), concrete infrastructure adapters, and an application layer that orchestrates polling and exposes Tauri commands.

```text
src-tauri/src/
  domain/   pure models + error types (no I/O)
  ports/    traits: InferenceProvider, HardwareMonitor, HistoryStore
  infra/    concrete adapters — llama/ (HTTP client), gpu/ (nvidia-smi), db/ (SQLite)
  app/      orchestration — monitor loops, Tauri commands, shared state, system tray

src/
  pages/        one component per page (Dashboard, Slots, Performance, Hardware, Settings)
  components/   shared UI (cards, charts, layout, primitives)
  hooks/        useMonitor.ts — snapshot, settings, and history hooks
  lib/          api.ts — typed wrapper over the Tauri IPC boundary
  types/        index.ts — frontend mirror of the backend contract
  test/         fixtures, setup, and the Tauri API stub
```

Data flow: the GPU monitor and the llama.cpp client feed the monitor service, which updates an in-memory snapshot and persists samples. The snapshot is published to the frontend as the `monitor:snapshot` event and is also readable on demand through commands.

### Where new tests go

- **Rust unit tests** — a `#[cfg(test)] mod tests` block in the module beside the code.
- **Rust integration tests** — `src-tauri/tests/`, exercising the public crate API.
- **Frontend tests** — co-located with the source as `*.test.tsx` / `*.test.ts`.

Cover error paths and edge cases (missing values are `null`, never `0`), not just the happy path.

## Commit convention

Use [Conventional Commits](https://www.conventionalcommits.org/). The types in use here are:

| Type | Use for |
|------|---------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `chore` | Maintenance, dependencies, tooling |
| `docs` | Documentation only |
| `test` | Tests only |
| `refactor` | Restructuring with no behaviour change |
| `perf` | Performance work |
| `ci` | CI/CD changes |

Example:

```text
feat(slots): render idle n_remain as unlimited
```

Keep commits atomic — one logical change each.

## Pull requests

1. Branch from `master` with a clear name: `feature/…`, `fix/…`, `docs/…`, `refactor/…`, or `chore/…`.
2. Keep changes surgical. Fix the thing you set out to fix; do not refactor unrelated code.
3. Run every quality gate above until they pass.
4. Never commit secrets, credentials, or generated build output.
5. Open the PR with a short description of the change and a reference to the issue it addresses.
6. Real data only — never commit mock data into the app; develop against a real llama.cpp server.

## Read-only by design

LlamaDesk is a **read-only observer**. It never starts, stops, or reconfigures a server, never writes to one, and never binds a network port. This is a deliberate product constraint, not an oversight: the monitor must be safe to point at a production inference server.

PRs that make the app write to, start, reconfigure, or manage a server will be rejected. If you want a control-plane feature, it belongs in a separate tool.
