# Screenshots

This directory holds the page screenshots referenced by the README.

## Ground rules

- Screenshots must come from a **real run** of LlamaDesk against a **real llama.cpp server**. Never mock, edit, synthesise, or hand-draw an image.
- Capture the actual window, at a readable size, with real data on screen.
- Do not include anything private: no personal paths, tokens, hostnames, or third-party windows in the frame.
- Size the window to **1280×800** before capturing — that matches the default window in `src-tauri/tauri.conf.json`.

## What to capture

Five pages, one image each, saved with these names:

| Page | Route | File | Suggested state |
|------|-------|------|-----------------|
| Dashboard | `/` | `dashboard.png` | Server connected, at least one loaded model |
| Slots | `/slots` | `slots.png` | A loaded model with visible slots |
| Performance | `/performance` | `performance.png` | At least one window with recorded history |
| Hardware | `/hardware` | `hardware.png` | NVIDIA GPUs present, real telemetry |
| Settings | `/settings` | `settings.png` | The settings form with the server URL shown |

## Capturing manually

1. Start your llama.cpp server and open LlamaDesk (`cargo tauri dev`, or a built binary).
2. Open the page from the sidebar.
3. Focus the LlamaDesk window and capture it (for example, a desktop screenshot tool's "active window" mode, or the whole screen with LlamaDesk maximised).
4. Save the image here under the name in the table above.

## Using `scripts/capture-screenshots.sh`

The optional helper script automates the screen capture. It:

- discovers an available display and the capture tooling on your system;
- waits for you to bring the LlamaDesk window up and press Enter for each page;
- captures the whole screen (the display root) into this directory.

No available capture backend can target a single window, so the image is always the full screen. Keep the LlamaDesk window visible and unobstructed, ideally maximised, before pressing Enter for each page.

It is best-effort: on a system with no usable display or no supported screenshot tool it prints what is missing and exits without failing the build. Run it only when you are ready to produce the images:

```bash
scripts/capture-screenshots.sh
```

The script also accepts `--display DISPLAY` to choose the X display, `--check` to report the detected backend and exit, and `-h`/`--help` for usage.

Because the exact window and its contents depend on your own live server, always review the resulting images against the ground rules above before committing them.
