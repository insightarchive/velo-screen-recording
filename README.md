# Velo Recorder

A fast, lightweight screen recorder for Windows, built with Rust and Tauri. No framework on the
front end: plain HTML, CSS and JavaScript.

## What it does

- Records a display or a window with system sound and microphone, to MP4 (H.264, GPU encoded).
- **Studio mode** keeps the raw video and a timeline of mouse and key activity beside it, so the
  automatic zoom, cursor, background and framing can be edited later without touching the footage.
- **Quick mode** saves a finished video straight away.
- While recording, a small always-on-top bar shows the time and controls. It is excluded from
  capture, so it never appears in the video.
- Global hotkeys: `Alt+Shift+R` start or stop, `Alt+Shift+P` pause or resume.

## Layout

| Path | What it holds |
|---|---|
| `src/` | The UI (HTML, CSS, JS). Runs in a browser too, with simulated data, for design work. |
| `src-tauri/` | The app and the Windows recording engine (capture, audio, input hooks, tray, hotkeys). |
| `crates/velo-director` | Auto-zoom: turns input events into zoom segments and a smooth camera path. |
| `crates/velo-core` | Clock, audio mixer, config and project files. Platform independent and unit tested. |
| `vendor/windows-capture` | Vendored with one patch: `VideoEncoder::pause()` and `resume()`. |

## Developing

```
npm install
npm run dev      # needs Rust and the Tauri prerequisites on Windows
cargo test -p velo-director -p velo-core
```

The installer is built by GitHub Actions (`.github/workflows/build.yml`); download it from the
workflow run's artifacts.
