# Ryshift — design

Drag any file, hold **Shift**, and a wheel of options pops up under your cursor. Drop the file on a slice and the result appears right next to the original. Free and open source, for **macOS, Windows and Linux (Ubuntu)**.


---

## 1. What it does

### The core gesture
1. Start dragging a file anywhere: Desktop, Finder, Downloads stack.
2. While still dragging, press **Shift**. A round wheel fades in centred on the cursor.
3. Move onto a slice. It lights up and the cursor shows a green **+**.
4. Let go. The file is processed and the new file lands beside the original, e.g. `Page 001.pdf` → `Page 001.jpg`.
5. Drop on the centre, or let go anywhere outside the wheel, to cancel. The file stays where it was.

### Second way in: select + shortcut (no dragging)
1. Click a file (or select several) in Finder / Explorer / Files.
2. Press the global shortcut: **⌥ Space** on Mac, **Alt + Space**-style combo on Windows and Linux (changeable in Settings).
3. The wheel opens at the mouse. **Click** a slice, or press its number key (1–8). **Tab** switches wheels and **Esc** cancels.

How it reads "which file is selected":
- **Mac:** asks Finder through Apple Events (one-time "allow Ryshift to control Finder" prompt).
- **Windows:** reads the selection of the active Explorer window through the Shell COM API (`IShellWindows`); no prompt.
- **Ubuntu:** the GNOME extension reads it from the Files app (Nautilus) and the desktop. On other desktops it uses whatever is on the clipboard after **Ctrl+C**.

The original file is never touched. Name clashes get ` 2`, ` 3` … like Finder does.

### Five wheels
Each wheel shows at most **8 slices** so it stays fast to hit. A slice marked **⋯** opens a second ring with the rest.

| Wheel | How to get it | What it's for |
|---|---|---|
| **Convert** | Shift | Change the format |
| **Tools** | Tap Shift again | Change the file itself (resize, crop, compress…) |
| **AI** | Tap Shift a third time | On-device smarts: read, summarise, translate, transcribe |
| **Share** | Hold **Option** (Mac) / **Alt** (Windows, Linux) instead of Shift | Send it somewhere |
| **Organise** | Hold Option / Alt + tap again | Rename, zip, move, tag |

While dragging, **scroll** (or tap the trigger again) to cycle wheels. The name of the current wheel and the file size sit in the centre hub.

### Every option, by file type
Bold = in v1. The rest comes in later milestones.

**Images** (PNG, JPG, HEIC, WEBP, TIFF, BMP, GIF, AVIF, SVG, RAW)
- Convert: **PNG, JPG, HEIC, WEBP, PDF**, TIFF, BMP, AVIF, GIF, ICO / ICNS (app icon set), SVG trace
- Tools: **Compress, Resize (50% · 1080 · 4K · custom), Crop, Rotate / flip, Add background, Strip metadata**, Remove background, Annotate, Redact, Watermark, Round corners, Add border / device frame, Upscale 2×, Combine into one PDF, Make collage / grid, Pick colour palette, Blur
- AI: **Copy text (OCR)**, Describe / alt-text, Smart rename, Translate text in image, Remove object
- Social presets: **Instagram post 4:5, Story / Reel 9:16, YouTube thumbnail 1280×720**, LinkedIn banner, X header

**PDF**
- Convert: **JPG, PNG (per page), TXT**, DOCX, Markdown, HTML, EPUB
- Tools: **Compress, Split pages, Merge with…, Rotate pages, Extract pages**, Reorder, Delete pages, Password-protect / unlock, Sign, Fill form, Flatten, Add page numbers, Watermark, Redact, Strip metadata, Make searchable (OCR)
- AI: **Summarise**, Ask a question, Translate, Extract tables → CSV, Make flashcards, Smart rename

**Documents** (DOCX, RTF, TXT, HTML, ODT, MD, Pages)
- Convert: **PDF, DOCX, TXT, RTF, HTML, Markdown**, EPUB
- Tools: Word count, Combine, Fix encoding / line endings
- AI: **Summarise**, Translate, Fix grammar, Turn into slides outline

**Spreadsheets / data** (CSV, XLSX, JSON, Numbers)
- Convert: **CSV ↔ XLSX ↔ JSON**, Markdown table, HTML table, PDF
- Tools: Remove duplicate rows, Split by column, Merge CSVs
- AI: Quick chart (PNG), Summarise the data

**Video** (MOV, MP4, MKV, WEBM, AVI)
- Convert: **MP4, MOV, GIF, M4A / MP3 (audio only)**, WEBM
- Tools: **Compress (for WhatsApp / email / Discord limits), Trim**, Resize to 9:16 / 1:1 / 16:9, Mute, Speed 0.5× / 1.5× / 2×, Extract frames, Contact sheet, Grab thumbnail, Rotate, Burn in subtitles
- AI: **Transcribe → SRT / TXT**, Auto-subtitles, Summarise, Find silences

**Audio** (WAV, M4A, MP3, AIFF, FLAC)
- Convert: **M4A, MP3, WAV**, AIFF, FLAC
- Tools: **Trim**, Normalise loudness, Remove silence, Fade in / out, Speed change, Merge
- AI: **Transcribe**, Summarise, Remove background noise

**Code / text files**
- Convert: Markdown → PDF / HTML, JSON ↔ YAML, Code → image (pretty screenshot)
- Tools: Format / prettify, Minify, Base64 encode / decode
- AI: Explain this file, Add comments

**Archives & folders** (ZIP, RAR, 7Z, TAR, any folder)
- Tools: **Zip, Unzip**, 7Z, Password zip, Flatten folder, List contents
- Organise: Batch rename, Sort into subfolders by type / date

**Any file**
- Share: **AirDrop, Copy to clipboard, Copy path, Messages / Mail**, Upload & copy link (needs your own storage, off by default), Make QR code, Send to iPhone
- Organise: **Rename, Duplicate, Move to… (pinned folders), Tag colour**, Compress to ZIP, Show in Finder, Open with…, Move to Trash
- Info: Checksum (SHA-256), Show size / dimensions / duration

### Your own slices
- **Pin and reorder**: drag slices around in Settings and hide the ones you never use.
- **Custom slice**: any shell command, Shortcut (Apple Shortcuts app) or AppleScript with `$FILE` in it. It shows up as a slice for the file types you pick.
- **Recipes**: chain actions into one slice, e.g. *HEIC → JPG → Resize 1080 → Strip metadata*.
- **Per-type defaults**: "PDFs always open the Tools wheel first".

Dragging **several files** runs the action on all of them (batch convert).

### Tool panels
Tools that need input open a small floating panel, then save and close:
- **Crop**: drag handles; presets Free, 1:1, 4:5, 9:16, 16:9 and A4; exact pixel boxes.
- **Add background**: gradient and solid swatches, padding, corner radius, shadow, aspect ratio. Saves as `… With Background.png`. Good for screenshots in videos and tweets.
- **Annotate**: arrows, boxes, highlighter, text.
- **Redact**: draw boxes, or *Auto* to blur emails, phone numbers and faces it finds on-device.
- **Resize**: presets plus width / height boxes with a lock.
- **Trim** (video / audio): a scrubber with in and out handles.
- **Ask a question** (PDF / doc): a small chat box answered by the on-device model.

### Tray app
- A small icon in the menu bar (Mac) or system tray (Windows, Linux) holds settings: on/off, trigger key (Shift / Option·Alt / Ctrl), which slices appear, JPG quality, output folder (beside original or a fixed folder), "open result after".
- First launch walks through the one permission it needs (see §3).
- No accounts, no network, no telemetry. Everything runs on-device.

---

## 2. Look and feel
- The wheel is ~220 pt across with 5–8 slices, a frosted, see-through look (native blur on Mac and Windows 11, a tinted translucent fill on Linux) and a soft tinted glow on the hovered slice.
- One shared look on all three systems, with small native touches: rounded corners and the accent colour follow the system.
- It springs open over about 150 ms and shrinks into the drop point on release.
- Icons come from one bundled open icon set (Lucide), labels are short (`JPG`, `CROP`), and on Macs there's light trackpad haptic feedback as you cross slices.
- Panels follow the system light/dark appearance.

---

## 3. How it works (tech)

### The big picture
One app, three systems. It splits into three layers:

```
┌────────────────────────────────────────────────────────┐
│  Wheel + panels + settings UI    (one web UI, shared)    │  ← looks the same everywhere
├────────────────────────────────────────────────────────┤
│  App shell: tray, windows, updates   (Tauri 2, Rust)     │  ← shared
├──────────────┬────────────────┬────────────────────────┤
│ Mac watcher  │ Windows watcher│ Linux watcher          │  ← small native part per OS:
│ (Swift/ObjC) │ (Win32)        │ (X11 / GNOME Shell ext)│    "is a file being dragged + Shift?"
├──────────────┴────────────────┴────────────────────────┤
│  Engine: every action      (Rust + bundled tools)        │  ← shared, also usable as a CLI
└────────────────────────────────────────────────────────┘
```

- **Tauri 2** (Rust plus the system's built-in web view) gives a ~10 MB app instead of Electron's ~150 MB, with tray, transparent always-on-top windows and auto-update on all three systems.
- The **engine** is a plain Rust library plus a CLI (`ryshift convert photo.heic --to jpg`). It's testable without any UI, and Linux users who can't get the overlay still get every feature.

### Spotting "file being dragged + Shift", per system
This is the only hard, OS-specific part.

| System | How it detects the drag | How the wheel receives the drop | Works? |
|---|---|---|---|
| **macOS** | Change count on the system drag pasteboard (`NSPasteboard(name: .drag)`) plus a global mouse-drag monitor; Shift read from `NSEvent.modifierFlags` | Transparent, non-activating `NSPanel` at the cursor registered as a drag destination; Finder hands over file URLs | ✅ Full |
| **Windows 10/11** | Poll `GetAsyncKeyState` for the left button and Shift; a drag is the button going down on an Explorer window or the desktop and moving past the system drag threshold. The dragged files are that window's selection, read through `IShellWindows` → `IShellBrowser` → `IFolderView` | Topmost window shown with `SW_SHOWNOACTIVATE` (so Explorer keeps the drag); WebView2's drop target receives Explorer's `CF_HDROP` as a copy | ✅ Explorer and desktop drags |
| **Ubuntu on X11** | One `QueryPointer` request gives the position, button and Shift state; a drag is `XdndSelection` changing hands (or being re-acquired) after the button went down. Its `text/uri-list` gives the files | Always-on-top window at the cursor; WebKitGTK is the XDND drop target | ✅ Full |
| **Ubuntu on Wayland** (the default since 22.04) | Wayland blocks normal apps from watching global input or placing windows, so a small **GNOME Shell extension** (part of this repo) polls the pointer from inside the Shell (`global.get_pointer()`: position, button, Shift) | The extension calls the app's D-Bus service (`Show`, `Cycle`, `MouseUp`) and moves the app's wheel window to the pointer; the file is then dropped on that window as a normal Wayland drop, and the wheel asks for its slices once the file enters | ✅ via the extension (GNOME 45+) |
| **Other Linux desktops** (KDE, Sway…) | No overlay at first | Fallbacks: right-click **Open with → Ryshift**, a drop-zone window you can pin, and the CLI | ⚠️ Partial; KDE support is a later add-on |

### Engine: what powers each action
The same tools are bundled on every system, so results match everywhere.

| Area | Library / tool |
|---|---|
| Images | `libvips` (fast resize/convert), `libheif` (HEIC/AVIF), `libwebp`, `resvg` (SVG) |
| PDF | `pdfium` (render, text, split/merge), `qpdf` (compress, encrypt, unlock) |
| Docs | `pandoc` (DOCX ↔ MD ↔ HTML ↔ EPUB ↔ RTF), headless LibreOffice optional for perfect DOCX → PDF |
| Video / audio | `ffmpeg` (bundled LGPL build) |
| Spreadsheets | `calamine` (read XLSX), `rust_xlsxwriter`, `csv` crate |
| Archives | `libarchive` (zip, 7z, tar, rar read) |
| Remove background | `rembg`-style U²-Net model through ONNX Runtime |
| OCR | Apple Vision on Mac, Windows.Media.Ocr on Windows, Tesseract on Linux |
| Transcription | `whisper.cpp` (Metal on Mac, CPU / Vulkan elsewhere) |
| AI: summarise, translate, Q&A, rename | Apple Foundation Models on Mac when present; otherwise a small local model through `llama.cpp` (downloaded on first use, ~1–2 GB, opt-in). Optional: your own API key |
| Share | AirDrop and Messages (Mac only), Windows Share sheet / Nearby Share, Linux: KDE Connect / copy link |
| Custom slices | Shortcuts / AppleScript on Mac, PowerShell on Windows, shell scripts everywhere |

Heavy pieces (ffmpeg, LibreOffice, AI models) download **on first use** so the installer stays small.

### Mac-only extras
- AirDrop, Messages, Apple Shortcuts and Foundation Models only exist on Mac.
- Windows and Linux get their closest equivalents (Nearby Share, PowerShell, local llama model).

### Plugin-style actions
Every slice is one small Rust module:
```rust
pub trait Action {
    fn id(&self) -> &str;                 // "convert.jpg"
    fn title(&self) -> &str;              // "JPG"
    fn icon(&self) -> &str;               // Lucide icon name
    fn wheel(&self) -> Wheel;             // Convert, Tools, Ai, Share, Organise
    fn platforms(&self) -> Platforms;     // ALL, or e.g. MAC_ONLY
    fn accepts(&self, mime: &Mime) -> bool;
    fn run(&self, inputs: &[PathBuf], opts: &Opts) -> Result<Vec<PathBuf>>; // new files
}
```
Adding a format is one file plus a test. This is the main way outside contributors will help.

### Permissions and install
| System | Asks for | Install via |
|---|---|---|
| macOS | Nothing for dragging (it only reads global state); "control Finder" the first time you use the shortcut | DMG on GitHub Releases, Homebrew cask |
| Windows | Nothing special (the hook needs no admin) | MSI / `.exe` installer, `winget` |
| Ubuntu | Nothing on X11; one-time GNOME extension on Wayland | `.deb`, AppImage, Flatpak (Flathub) |

---

## 4. Repo layout
```
ryshift/
├─ engine/              Rust library: all actions, file-type detection, naming
├─ cli/                 `ryshift` command line, same actions
├─ app/                 Tauri 2 app: tray, settings, updater
│  └─ ui/               wheel + panels (plain HTML/CSS/JS, no build step), shared on all OSes
├─ watchers/
│  ├─ macos/            drag pasteboard + NSPanel drop target
│  ├─ windows/          WH_MOUSE_LL hook + RegisterDragDrop window
│  └─ linux-x11/        XInput2 + XDND target
├─ gnome-extension/     Wayland overlay for Ubuntu (JavaScript)
├─ tests/fixtures/      one sample file per type, golden outputs
├─ .github/workflows/   CI builds + tests on macOS, Windows, Ubuntu; release installers
├─ docs/                demo GIF, CONTRIBUTING, adding-an-action.md
└─ README.md, LICENSE (MIT)
```

---

## 5. Milestones
| # | Ships | You can… |
|---|---|---|
| **M1** ✅ | Engine + CLI (images, PDF → images), Tauri shell, **Mac watcher**, Convert wheel | Do the reel's PDF → JPG demo on Mac |
| **M2** ✅ | **Windows watcher + Linux X11 watcher**, CI building all three | Same demo on Windows and Ubuntu (X11) |
| **M3** ✅ | **GNOME Shell extension** for Ubuntu Wayland | Works on stock Ubuntu |
| **M4** | Tools wheel: Compress, Resize, Crop, Add background, Strip metadata; docs conversions; batch | Full match with the reel, on all three |
| **M5** | Video / audio, Remove background, Redact, Annotate, data formats, archives | Go past the reel |
| **M6** | AI wheel, Share and Organise wheels, custom slices and recipes | More than a converter |
| **M7** | Installers (DMG, MSI, deb, AppImage, Flatpak), Homebrew / winget / Flathub, auto-update, demo video | Launch publicly |

## 6. Open questions
- Name: **Ryshift** (placeholder). Alternatives: Dropwheel, Flick, Spindrop.
- **Signing:** an unsigned app works but shows scary warnings.
  - Mac: notarising needs an Apple Developer account ($99/yr).
  - Windows: a code-signing certificate, or Azure Trusted Signing (~$10/month).
  - Linux: no signing needed.
  - Fine to launch unsigned with clear "how to open" notes and add signing later.
- **Testing:** I can build and test the Mac version here. Windows and Ubuntu builds get compiled and tested by GitHub Actions on every push, but someone should try the drag feel on a real Windows PC and an Ubuntu machine (or a VM) before launch.
- **Ubuntu Wayland:** the GNOME extension route is the only way to get the "wheel at the cursor" feel there. If the extension is too fiddly, Wayland users get the drop-zone and right-click fallbacks.
