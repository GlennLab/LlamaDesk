# LlamaDesk

LlamaDesk is a desktop application that **monitors** a local [llama.cpp](https://github.com/ggml-org/llama.cpp) inference server and, optionally, the NVIDIA GPUs on the same machine. It presents live server status, the dynamic list of models and their slots, per-model performance metrics, GPU telemetry, and locally stored history in a single window.

LlamaDesk is a **read-only observer**. It never starts, stops, or reconfigures an inference server, and it never binds a network interface. All HTTP requests to the server are made by the Rust backend; the webview never talks to the server directly. If the server is not running, LlamaDesk simply reports that it is unreachable and keeps the last data it has.

Built with [Tauri 2](https://v2.tauri.app/) (Rust backend + React/TypeScript frontend) and SQLite for history.

## Features

- **Live server status** — connection state (`connected` / `disconnected` / `error`), server role and build, polled on a configurable interval.
- **Dynamic model and slot list** — all models reported by the server, with their load status; slots are shown per loaded model.
- **Per-model performance metrics** — prompt and generation throughput, active and deferred requests, decoded-token counters, and KV-cache usage, parsed from the server's Prometheus endpoint.
- **GPU statistics** — utilization, VRAM used/total, temperature, and power draw/limit per NVIDIA GPU, queried through `nvidia-smi`.
- **Historical charts** — server and GPU samples persisted every few seconds and rendered as time-series charts over selectable windows (15 minutes to 24 hours).
- **Settings** — server URL, polling intervals, and history retention, validated and persisted, applied live without restarting.
- **Graceful degradation** — a failure in any one polling slice is recorded as a non-fatal error and never clears the data from the other slices or the last good value of the failed one.
- **Offline history** — once samples have been recorded, the Performance charts remain readable even while the server is unreachable.

## Screenshots

<!-- TODO: add screenshots. Do not fabricate images — capture them from a running build. -->

> **Placeholder — screenshots to be added.** Capture each page from a live run and place the files under `docs/screenshots/`.

| Page | Screenshot |
|------|------------|
| Dashboard | <!-- TODO: screenshot --> |
| Slots | <!-- TODO: screenshot --> |
| Performance | <!-- TODO: screenshot --> |
| Hardware | <!-- TODO: screenshot --> |
| Settings | <!-- TODO: screenshot --> |

## Requirements

LlamaDesk targets **Linux** (verified on Zorin OS 18.1 / Ubuntu noble, x86_64) and requires the following.

### System packages (Ubuntu / Debian)

These correspond to the actual crate dependencies: GTK 3 and WebKitGTK 4.1 with libsoup 3 (pulled in by Tauri), SQLite (used by `sqlx`), and OpenSSL (used by `reqwest`'s default TLS backend), plus a C toolchain.

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libsoup-3.0-dev \
  libsqlite3-dev \
  libssl-dev
```

### Toolchains

| Tool | Version | Source |
|------|---------|--------|
| Rust / Cargo | 1.90 or newer (MSRV declared in `src-tauri/Cargo.toml`; 1.96 verified) | `rustup` |
| Node.js / npm | Node 22 (22.23.3 verified) | [nodejs.org](https://nodejs.org/) |
| Tauri CLI | 2.x (`cargo-tauri` present in `~/.cargo/bin`) | `cargo install tauri-cli --version "^2"` |
| NVIDIA driver | Optional. Provides `nvidia-smi`. | distribution package manager |

### The inference server

A **running llama.cpp server or router is required for any live data**. LlamaDesk does not install, launch, or manage it — start llama.cpp yourself (for example, a router listening on `http://localhost:8080`) before or after opening the app. Without it, the dashboard reports `Disconnected` and only persisted history is available.

`nvidia-smi` is optional. When the binary is absent, or when a GPU reports `[N/A]` for a field, that value is shown as `—` and the rest of the application continues to work.

## Install & build

Clone the repository and install the frontend dependencies:

```bash
git clone <repository-url> LlamaDesk
cd LlamaDesk
npm install
```

### Development

Run the full desktop application with hot reload:

```bash
cargo tauri dev
```

This starts the Vite dev server on port **1420** (strict) and launches the Tauri window, which loads the frontend from it. To run the frontend alone in a browser (no backend, so no live data), use:

```bash
npm run dev
```

### Production bundle

```bash
cargo tauri build
```

The build first runs `npm run build` (TypeScript compile + Vite build into `dist/`), then compiles the Rust binary in release mode. Output lands under the root Cargo workspace at `target/release/`:

- `target/release/llamadesk` — the standalone executable.
- `target/release/bundle/` — installer artifacts. `tauri.conf.json` sets `bundle.targets` to `"all"`, so on Ubuntu/Debian this produces the platform-supported `.deb`, `.rpm`, and `.AppImage` bundles (the exact set depends on the host's bundling tools — RPM bundling requires `rpmbuild`).

> **Note on the CLI:** the Tauri CLI is not declared as an npm dependency in this project, so `npm run tauri ...` is not available. Use `cargo tauri ...` (or install the npm CLI with `npm install --save-dev @tauri-apps/cli` if you prefer `npx tauri ...`).

### Install / deploy

For a repeatable local build and install the repository ships helper scripts and a `Makefile`. `make help` lists every target.

Quick path — per-user install, no sudo:

```bash
make build          # frontend + app; produces deb + AppImage on Debian-family
make install-user   # installs under ~/.local
```

#### `scripts/build.sh`

Builds the frontend (`npm run build`) and the application (`cargo tauri build`). On Debian/Ubuntu it bundles `deb,appimage` by default; other distributions default to `appimage`, so the build does not fail when `rpmbuild` is missing. Override the set with `--bundles` or build everything with `--all`:

```bash
scripts/build.sh                    # deb + AppImage on Debian-family
scripts/build.sh --bundles deb      # just the .deb
scripts/build.sh --all              # deb + rpm + AppImage (needs rpmbuild)
```

Artifacts land under `target/release/` (the root Cargo workspace) and `target/release/bundle/{deb,rpm,appimage}/`. The script prints each artifact path with its size and fails if the binary was not produced.

#### `scripts/install.sh`

Installs from a built artifact and writes an XDG `.desktop` entry plus icon.

```bash
scripts/install.sh --user                            # ~/.local (default), no sudo
scripts/install.sh --system                          # /opt/llamadesk + /usr/local/bin launcher (sudo)
scripts/install.sh --user --prefix /opt/llamadesk    # custom prefix
scripts/install.sh --dry-run                         # show what would happen, change nothing
```

Payload preference: an existing `.deb` (installed with `dpkg -i` for a system install, or extracted and relocated for `--user` and custom prefixes), else the built AppImage, else the plain binary. The installer is idempotent, validates and refreshes the desktop database when `desktop-file-validate` and `update-desktop-database` are present (it says so and skips otherwise), and never deletes your data or restarts a running server.

**Tray dependency:** the system tray needs `libayatana-appindicator3`. The installer checks for the WebKitGTK 4.1 / GTK 3 / libsoup 3 / appindicator shared libraries and, if any are missing, warns without failing:

```bash
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0 libayatana-appindicator3-1
```

#### Uninstall

```bash
scripts/uninstall.sh --user            # remove a user install
scripts/uninstall.sh --system          # remove a system install
scripts/uninstall.sh --user --purge    # also delete settings + history
```

Uninstall removes exactly what the installer created (binary/launcher, `.desktop`, icon) and refreshes the desktop database. Your settings (`~/.config/com.llamadesk.app/`) and history (`~/.local/share/com.llamadesk.app/`) are kept unless you pass `--purge`. It is safe to run when nothing is installed.

## Usage

LlamaDesk has five pages, reachable from the sidebar.

### Dashboard

The live overview: connection state, server URL, last-updated time, server role and build, the number of known models, the list of currently loaded models, headline metrics for the first loaded model (active/deferred requests, free slots, prompt and generation throughput), and per-GPU VRAM usage.

### Slots

One panel per loaded model, listing its decode slots with live processing state, context size, prompt-token counters, and next-token details. Slots are only reported for models the server marks as loaded.

### Performance

Time-series charts of persisted history: prompt throughput, generation throughput, active requests, deferred requests, per-GPU utilization, and per-GPU VRAM. Choose a window from 15 minutes up to 24 hours and refresh on demand. Because history is stored locally, these charts remain available when the server is offline.

### Hardware

Per-GPU detail cards plus totals (GPU count, total VRAM, VRAM used, average utilization) computed only from values the driver actually reported.

### Settings

- **Server URL** — the base URL of the llama.cpp server or router. It must be an **absolute `http` or `https` URL** with an explicit scheme; a bare `host:port` is rejected. The default is `http://localhost:8080`.
- **Polling intervals** — server, slots, hardware, and storage intervals in milliseconds. Values are clamped to safe floors (server/slots/hardware ≥ 250 ms, storage ≥ 1000 ms).
- **Retention (hours)** — how long history is kept, from 1 hour to one year.
- **Test connection** — probes the configured (or a one-off) URL and reports the role, build, and model count.
- **Prune history now** — deletes samples older than the retention window and reports how many rows were removed.

Settings are written to a JSON file in the Tauri app **config** directory:

```text
~/.config/com.llamadesk.app/config.json
```

### Where history is stored

Samples are written to a SQLite database in the Tauri app **data** directory:

```text
~/.local/share/com.llamadesk.app/llamadesk.db
```

(The exact directory is resolved by Tauri from the app identifier `com.llamadesk.app` and the `XDG_CONFIG_HOME` / `XDG_DATA_HOME` environment variables; the paths above are the conventional Linux locations.)

The database runs in WAL mode, so sibling `-wal` and `-shm` files may also appear. It contains two tables:

- `server_samples` — one row per storage tick (prompt/generation throughput, active/deferred requests, KV-cache ratio, token totals).
- `gpu_samples` — one row per GPU per storage tick (utilization, VRAM used/total, temperature, power draw).

A sample is persisted on the **storage interval** (default 5 s). Retention is enforced by a **prune pass run once per hour**, and can also be triggered manually from Settings. Deleting the database file is safe: storage is best-effort and history simply restarts empty.

## Architecture

LlamaDesk follows a layered structure: pure domain models, provider-agnostic ports (traits), concrete infrastructure adapters, and an application layer that orchestrates polling and exposes Tauri commands to the React frontend.

```text
React frontend (src/)
  pages, components, hooks, types
        │  Tauri invoke / events
        ▼
Tauri backend (src-tauri/src/)
  app/        orchestration — monitor loops, commands, shared state
  ports/      traits: InferenceProvider, HardwareMonitor, HistoryStore
  domain/     pure models + error types (no I/O)
  infra/      concrete adapters
    llama/    HTTP client for the llama.cpp API
    gpu/      nvidia-smi reader
    db/       SQLite history store
    config.rs config load/save/validate
```

Data flow: the GPU monitor and the llama.cpp client feed the application's monitor service, which updates an in-memory snapshot and persists samples. The snapshot is published to the frontend as an event and is also readable on demand through commands.

### Tauri command surface

Invoke names and signatures are frozen in `the project contract`; all JSON is camelCase and missing values are `null` (never coerced to `0`).

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_snapshot` | — | `MonitorSnapshot` |
| `get_settings` | — | `AppConfig` |
| `update_settings` | `{ config: AppConfig }` | `AppConfig` (validated, persisted, applied live) |
| `test_connection` | `{ url?: string }` | `ConnectionTest { ok, message, role?, buildInfo?, modelCount? }` |
| `get_server_history` | `{ fromMs: number, toMs: number }` | `ServerSample[]` |
| `get_gpu_history` | `{ fromMs: number, toMs: number }` | `GpuSample[]` |
| `prune_history` | — | `number` (rows removed) |

Command errors reject with a string message.

### `monitor:snapshot` event

The backend emits `monitor:snapshot` with a `MonitorSnapshot` payload whenever the snapshot changes materially (a timestamp-only change does not trigger an emit). The frontend's `SnapshotProvider` subscribes once and shares it with every page. The payload aggregates:

```jsonc
{
  "connection": "connected",              // "connected" | "disconnected" | "error"
  "health": { "status": "ok" },           // null until /health responds
  "props": { "role": "router", "buildInfo": "b2794-62df1815", ... }, // null until /props responds
  "models": [ /* ModelInfo */ ],
  "loadedModels": ["<modelId>"],
  "slots": { "<modelId>": [ /* Slot */ ] },
  "metrics": { "<modelId>": { /* ServerMetrics */ } },
  "gpus": [ /* GpuStats */ ],
  "gpuAvailable": true,
  "serverUrl": "http://localhost:8080",
  "lastUpdated": "2026-10-15T23:00:00.000Z",
  "errors": [ { "slice": "slots", "message": "…" } ]
}
```

Monitoring runs four independent polling slices with their own intervals (defaults shown): server 2 s, slots 1 s, hardware 2 s, storage 5 s. All intervals are configurable in Settings.

## Compatibility / behaviour notes

These are properties of the llama.cpp API that LlamaDesk accounts for:

- **`/slots?model=` and `/metrics?model=` require the `model` query parameter.** Calling them without it returns HTTP 400. LlamaDesk always passes the model id.
- **Unloaded models block.** On a server with model autoload enabled, requesting slots or metrics for a model whose status is not `loaded` can hang instead of returning. LlamaDesk therefore **only polls `slots` and `metrics` for models the server reports as loaded**, and bounds every request with a timeout. A loaded-to-unloaded race surfaces as one non-fatal timeout in the affected slice and does not erase that model's last-known data.
- **Missing is not zero.** Every metric is optional end to end. A value that the server or driver did not report is `null` and is rendered as `—`, never as `0`. This preserves the distinction between "zero throughput" and "no reading".
- **Metrics are Prometheus text.** `/metrics` returns Prometheus exposition format (`llamacpp:*`), which the backend parses into typed fields; unparsed names are preserved in a `raw` map.
- **GPU index order is not guaranteed.** GPUs are always identified by the index `nvidia-smi` reports; LlamaDesk never assumes an order or a count.
- **`nvidia-smi` is optional.** Its absence is a normal degraded state: `gpuAvailable` is `false`, GPU pages show an explanatory empty state, and server monitoring continues.

## Troubleshooting

**The dashboard shows "Disconnected".**
LlamaDesk is not connected to a server. Confirm that llama.cpp is running and reachable at the configured URL (Settings → Server URL, default `http://localhost:8080`). Use **Test connection** to see the exact result. If you run llama.cpp on another host or port, update the URL — it must be an absolute `http`/`https` URL.

**GPUs do not appear, or values show `—`.**
`nvidia-smi` is not installed or is not on `PATH`, or the driver returned `[N/A]` for a field. Install/repair the NVIDIA driver, or verify with `nvidia-smi` in a terminal. When `nvidia-smi` is missing, the Hardware page reports that NVIDIA monitoring is unavailable; this is expected and non-fatal.

**Build fails on webkit2gtk / gtk / soup.**
The GTK, WebKitGTK 4.1, and libsoup 3 development packages are missing or too old. Install the packages from the Requirements section (`libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`). Ensure you are using the `4.1` WebKitGTK package — the older `4.0` line is not what Tauri 2 uses here.

**Port 8080 is already in use.**
LlamaDesk never binds any port; it only makes outbound requests. A conflict on 8080 means another process (or a second llama.cpp instance) holds it, so your server never started or listens elsewhere. Free the port or start llama.cpp on a different port and set that URL in Settings.

**The dev window is blank or a port error appears.**
The Vite dev server uses port 1420 with `strictPort`, so it fails if 1420 is taken. Stop the process using it and run `cargo tauri dev` again.

## Development

Quality gates for the Rust backend (run from the repository root; the Cargo workspace member is `src-tauri`):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`cargo test` runs the unit and integration tests and skips the live-router tests, which are all `#[ignore]`d so CI without a server stays green. To run them against a real llama.cpp router:

```bash
cargo test --test live_router -- --ignored --nocapture
```

The live tests default to `http://localhost:8080` and can be pointed elsewhere with the `LLAMADESK_TEST_URL` environment variable.

Frontend type-check and production build:

```bash
npm run build
```

## License

LlamaDesk is declared as **MIT** in the crate manifest (`src-tauri/Cargo.toml`, `license = "MIT"`). No standalone `LICENSE` file is currently included in this repository; add one before distributing the project.
