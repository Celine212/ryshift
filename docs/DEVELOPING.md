# Developing Ryshift

## Platform status

| | Drag + Shift | Select + shortcut | Engine / CLI |
|---|---|---|---|
| macOS 12+ | ✅ from Finder, the desktop, or any app | ✅ Finder selection, ⌥ Space | ✅ |
| Windows 10/11 | ✅ from File Explorer and the desktop | ✅ Explorer selection, Ctrl + Alt + Space | ✅ |
| Ubuntu on X11 | ✅ from any app that drags files (Files, desktop, browsers) | ✅ copy files (Ctrl+C), then Ctrl + Alt + Space | ✅ |
| Ubuntu on Wayland (the default) | ✅ with the Ryshift GNOME extension (GNOME 45+, Ubuntu 23.10+) | ✅ copy files (Ctrl+C), then Ctrl + Alt + Space | ✅ |

Windows and Ubuntu support is new and has been checked by CI builds only. If the wheel misbehaves on your machine, please open an issue.

### Ubuntu on Wayland

Wayland doesn't let normal apps watch the mouse or place their own windows, so on Wayland Ryshift needs a small GNOME Shell extension. The extension spots "dragging + Shift", tells the app over D-Bus, and moves the wheel to your pointer. The file is still dropped on the app's own window, so the extension never touches files.

```bash
scripts/pack-gnome-extension.sh
gnome-extensions install --force dist/ryshift@alexriderspy.github.io.shell-extension.zip
```

Log out and back in, then turn on **Ryshift** in the Extensions app. The Ryshift app must be running too. CI also builds the extension zip on every push (the `ryshift-gnome-extension` artifact).

## Install

Download the file for your system from the [latest release](https://github.com/alexriderspy/ryshift/releases/latest).

**macOS** (Apple Silicon: `Ryshift_…_aarch64.dmg`, Intel: `Ryshift_…_x64.dmg`)
1. Open the DMG and drag **Ryshift** into **Applications**.
2. The first time, **right-click Ryshift → Open**, then **Open** again. Builds aren't notarised yet, so a normal double-click is blocked once.
3. Ryshift appears in the menu bar. The first time you press ⌥ Space, allow it to control Finder.

**Windows 10/11** (`Ryshift_…_x64-setup.exe` or `.msi`)
1. Run the installer. If SmartScreen says "Windows protected your PC", click **More info → Run anyway** (builds aren't signed yet).
2. Ryshift starts in the system tray.

**Ubuntu** (`ryshift_…_amd64.deb` or `Ryshift_…_amd64.AppImage`)

```bash
sudo apt install ./ryshift_*_amd64.deb
```

Or make the AppImage executable (`chmod +x Ryshift_*.AppImage`) and run it. On the default **Wayland** session, also install the GNOME extension from the same release (see [Ubuntu on Wayland](#ubuntu-on-wayland)).

## Releasing (maintainers)

Bump `version` in `app/src-tauri/tauri.conf.json`, then push a tag:

```bash
git tag v0.2.0
git push origin v0.2.0
```

The **Release** workflow builds every installer (macOS Apple Silicon and Intel, Windows `.exe` and `.msi`, Ubuntu `.deb` and AppImage) plus the GNOME extension zip, and attaches them to a **draft** release to check and publish by hand. Run the workflow manually ("Run workflow") to test the builds without a release. Code signing turns on by adding the secrets listed in `.github/workflows/release.yml`.

## Build from source

You need [Rust](https://rustup.rs) and Node.js 18+. On Linux, also install the [Tauri system packages](https://v2.tauri.app/start/prerequisites/#linux).

```bash
git clone https://github.com/alexriderspy/ryshift
cd ryshift
scripts/fetch-pdfium.sh     # downloads the PDF engine into lib/
cd app && npm install
npm run dev                 # run it
npm run build               # make Ryshift.app / .dmg (or .exe, .deb, .AppImage)
```

Ryshift lives in the menu bar. There's no main window: use the menu bar icon to quit.

## Command line

Every one-click action also works from the terminal, plus a few extras (`tool.half`, `tool.rotate`, `tool.square`, `tool.story`):

```bash
cargo run -p ryshift-cli -- list scan.pdf          # which actions fit this file
cargo run -p ryshift-cli -- run convert.jpg scan.pdf
cargo run -p ryshift-cli -- run tool.half photo.png
```

## How it works

- **`engine/`** is a Rust library that does the actual converting. It doesn't know about windows.
- **`app/`** is a [Tauri 2](https://v2.tauri.app) menu bar app. The wheel itself is plain HTML/CSS/JS in `app/ui/`.
- **`app/src-tauri/src/platform/`** is the only OS-specific code. Each OS spots "a file is being dragged and Shift went down" by polling global state, without hooks or special permissions:
  - **macOS** watches the system drag pasteboard.
  - **Windows** watches for a drag that starts in Explorer and reads the dragged files from Explorer's selection.
  - **Linux (X11)** watches the XDND drag selection.
  - **Linux (Wayland)** relies on the GNOME Shell extension in `gnome-extension/`, which calls the app's D-Bus service (`io.github.alexriderspy.Ryshift`).

  The wheel is a transparent window that accepts the drop as a copy, so the original file never moves.

To work on the UI without building the app, serve the repo (`python3 -m http.server`) and open:

- `docs/dev/wheel-preview.html?demo` to see the wheel
- `docs/dev/editor-preview.html?mode=crop` (or `edit`, `annotate`, `addbg`, `redact`) for an editor panel
- `docs/dev/ui-tests.html` for the automated UI tests (28 of them: wheel hit-testing, every editor panel, checked pixel by pixel)

## Contributing

New formats are the easiest way to help. Add a case in `engine/src/lib.rs`, the code in its own module, and a test in `engine/tests/`. Before opening a PR, run `cargo test -p ryshift-engine` and open `docs/dev/ui-tests.html`.

## Look

The wheel sits on real OS glass: Liquid Glass on macOS 26+, the frosted HUD blur on older Macs, Acrylic on Windows, and a translucent stand-in on Linux. Colours and type follow the Rynovi brand (purple `#6f5be0`, ink `#17330f`, Inter Tight).
