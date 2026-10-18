<p align="center">
  <img src="docs/logo.svg" width="128" alt="Ryshift logo" />
</p>

<h1 align="center">Ryshift</h1>

<p align="center">Drag a file, press Shift, drop it on a wheel.</p>

Start dragging any file, press **Shift**, and a wheel of options opens under your cursor. Drop the file on a slice and the result is saved next to the original. Works on macOS, Windows and Ubuntu.

## What it does

- **Convert** images between PNG, JPG, WEBP, HEIC, TIFF, BMP, GIF and PDF. Turn PDFs into images or text.
- **Compress** or **strip metadata** in one drop.
- **Edit:** crop, rotate, adjust colours, annotate, add a background, redact.
- **Batch:** drop several files at once.
- **Offline:** no account, no uploads.

Tap Shift again mid-drag to switch between the Convert and Tools wheels. Or select a file and press ⌥ Space (Ctrl + Alt + Space on Windows and Ubuntu).

## Tech stack

- **Engine and CLI:** Rust (`image`, Pdfium)
- **App:** Tauri 2, with the wheel and editors in plain HTML, CSS and JS
- **Glass:** Liquid Glass on macOS, Acrylic on Windows
- **Ubuntu on Wayland:** a GNOME Shell extension

## Install

Download it from the [latest release](https://github.com/alexriderspy/ryshift/releases/latest). Per-system notes and build steps are in [docs/DEVELOPING.md](docs/DEVELOPING.md).

## License

MIT
