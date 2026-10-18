# Shiftwheel

Drag a file, press **Shift**, and a wheel of options pops up under your cursor. Drop the file on a slice and the converted file appears right next to the original.

`Page 001.pdf` → drag, Shift, drop on **JPG** → `Page 001.jpg`

- **No windows to open, no upload sites.** It works from Finder, the desktop, or anywhere else you can drag a file.
- **Everything stays on your computer.** No account, no network, no telemetry.
- **Free and open source (MIT).** macOS, Windows and Ubuntu (X11); Ubuntu on Wayland is next.

## What it can do (v0.1)

| You drag | Convert wheel | Tools wheel |
|---|---|---|
| Images: PNG, JPG, WEBP, HEIC, TIFF, BMP, GIF, ICO | PNG, JPG, WEBP, PDF, HEIC (Mac), TIFF, BMP, GIF | Compress, Half size, Rotate, Strip metadata, Square 1:1, Story 9:16 |
| PDF | JPG, PNG (one per page), TXT | — |

Drag several files at once and the action runs on all of them. Originals are never touched, and a name that's taken becomes `Name 2.jpg`, like Finder does.

The full plan (five wheels, 100+ actions, video, audio, documents, AI tools) is in [docs/DESIGN.md](docs/DESIGN.md).

## How to use it

**Drag + Shift**
1. Start dragging a file.
2. While it's moving, press **Shift**. The wheel opens around the cursor.
3. Press **Shift again** to switch between the Convert and Tools wheels.
4. Drop on a slice. Drop on the centre, or anywhere else, to cancel.

**Select + shortcut** (no dragging)
1. Select files in Finder or File Explorer. On Ubuntu, copy them with Ctrl+C.
2. Press **⌥ Space** on Mac, or **Ctrl + Alt + Space** on Windows and Ubuntu. The wheel opens at the mouse.
3. Click a slice or press its number (1–8). **Tab** switches wheels and **Esc** closes it.

The first time you use the shortcut, macOS asks whether Shiftwheel may talk to Finder. It needs that to read which files you selected.

## Platform status

| | Drag + Shift | Select + shortcut | Engine / CLI |
|---|---|---|---|
| macOS 12+ | ✅ from Finder, the desktop, or any app | ✅ Finder selection, ⌥ Space | ✅ |
| Windows 10/11 | ✅ from File Explorer and the desktop | ✅ Explorer selection, Ctrl + Alt + Space | ✅ |
| Ubuntu on X11 | ✅ from any app that drags files (Files, desktop, browsers) | ✅ copy files (Ctrl+C), then Ctrl + Alt + Space | ✅ |
| Ubuntu on Wayland (the default) | Coming in M3 (GNOME Shell extension) | Coming in M3 | ✅ |

Windows and Ubuntu support is new and has been checked by CI builds only. If the wheel misbehaves on your machine, please open an issue.

On Ubuntu, pick **Ubuntu on Xorg** on the login screen (gear icon) to use Shiftwheel before the Wayland extension lands.

## Build from source

You need [Rust](https://rustup.rs) and Node.js 18+. On Linux, also install the [Tauri system packages](https://v2.tauri.app/start/prerequisites/#linux).

```bash
git clone https://github.com/alexriderspy/shiftwheel
cd shiftwheel
scripts/fetch-pdfium.sh     # downloads the PDF engine into lib/
cd app && npm install
npm run dev                 # run it
npm run build               # make Shiftwheel.app / .dmg (or .exe, .deb, .AppImage)
```

Shiftwheel lives in the menu bar. There's no main window: use the menu bar icon to quit.

## Command line

Every action also works from the terminal:

```bash
cargo run -p shiftwheel-cli -- list scan.pdf          # which actions fit this file
cargo run -p shiftwheel-cli -- run convert.jpg scan.pdf
cargo run -p shiftwheel-cli -- run tool.half photo.png
```

## How it works

- **`engine/`** is a Rust library that does the actual converting. It doesn't know about windows.
- **`app/`** is a [Tauri 2](https://v2.tauri.app) menu bar app. The wheel itself is plain HTML/CSS/JS in `app/ui/`.
- **`app/src-tauri/src/platform/`** is the only OS-specific code. Each OS spots "a file is being dragged and Shift went down" by polling global state, without hooks or special permissions:
  - **macOS** watches the system drag pasteboard.
  - **Windows** watches for a drag that starts in Explorer and reads the dragged files from Explorer's selection.
  - **Linux (X11)** watches the XDND drag selection.

  The wheel is a transparent window that accepts the drop as a copy, so the original file never moves.

To tweak the wheel's look without building the app, serve the repo (`python3 -m http.server`) and open `docs/dev/wheel-preview.html`.

## Contributing

New formats are the easiest way to help. Add a case in `engine/src/lib.rs`, the code in its own module, and a test in `engine/tests/`. Run `cargo test -p shiftwheel-engine` before opening a PR.

## License

MIT
