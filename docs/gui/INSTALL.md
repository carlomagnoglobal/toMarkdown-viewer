# toMarkdown Viewer — Installation Guide (Windows · macOS · Linux)

Two ways to get the app:

1. **Download a prebuilt bundle** from the `GUI build` CI workflow (built for every `gui-v*` release tag)
2. **Build from source** with Rust + Tauri

Bundles are unsigned on every platform — each OS shows a one-time warning the first time you open the app; the steps to get past it are below.

---

## 1. Download a prebuilt bundle

Bundles are produced by the [GUI build workflow](../../.github/workflows/gui-release.yml) and uploaded as workflow artifacts:

1. Open the repository's **Actions → GUI build** page and pick the run for the release tag (e.g. `gui-v0.2.0`).
2. Download the artifact for your OS: `toMarkdown-Viewer-macos`, `toMarkdown-Viewer-linux`, or `toMarkdown-Viewer-windows`.
3. Unzip it and install per the platform section below.

With the GitHub CLI:

```sh
gh run list --workflow "GUI build"                 # find the run id for the tag
gh run download <run-id> -n toMarkdown-Viewer-macos   # or -linux / -windows
```

### macOS

The artifact contains a `.dmg` (and a `.app.tar.gz`).

1. Open the `.dmg` and drag **toMarkdown Viewer** into **Applications**.
2. First launch — the app is not notarized, so **right-click the app → Open → Open** (a plain double-click shows "cannot be opened because the developer cannot be verified").
   If Gatekeeper still refuses (macOS 15+): **System Settings → Privacy & Security → "toMarkdown Viewer was blocked" → Open Anyway**, or clear the quarantine flag:
   ```sh
   xattr -dr com.apple.quarantine "/Applications/toMarkdown Viewer.app"
   ```
3. `.md` files can now be opened with the app from Finder (Open With → toMarkdown Viewer; "Always Open With" to make it the default).

Requirements: macOS 10.15+ (Intel or Apple Silicon matching the build).

### Linux

The artifact contains an **AppImage** (any distro) and a **.deb** (Debian/Ubuntu).

**AppImage** (no installation):

```sh
chmod +x toMarkdown-Viewer_*.AppImage
./toMarkdown-Viewer_*.AppImage
```

If it fails to start on a minimal system, install the WebKitGTK runtime (see build prerequisites below — the runtime packages are the same names without `-dev`).

**Debian / Ubuntu (.deb):**

```sh
sudo apt install ./toMarkdown-Viewer_*.deb     # resolves webkit2gtk dependencies
to-markdown-gui                                 # or launch from your app menu
```

Requirements: WebKitGTK 4.1 (Ubuntu 22.04+, Debian 12+, Fedora 36+ or equivalents).

### Windows

The artifact contains an **.msi** installer and a portable **.exe** (NSIS).

1. Run the `.msi` (or the setup `.exe`).
2. **SmartScreen** will warn because the installer is unsigned: click **More info → Run anyway**.
3. If the app window is blank on first launch, install the **Microsoft Edge WebView2 Runtime** (preinstalled on Windows 11 and most Windows 10 systems; otherwise download the "Evergreen Bootstrapper" from Microsoft).

Requirements: Windows 10 1803+ with WebView2.

---

## 2. Build from source

### Prerequisites (all platforms)

- **Rust 1.88+** — `curl https://sh.rustup.rs -sSf | sh` (or [rustup.rs](https://rustup.rs) installers on Windows)
- The repository: `git clone https://github.com/carlomagnoglobal/toMarkdownMCP && cd toMarkdownMCP`
- For packaging only: **tauri-cli** — `cargo install tauri-cli --locked`

No Node.js/npm is needed — the frontend is static.

### Platform prerequisites

**macOS**

```sh
xcode-select --install        # Xcode Command Line Tools
```

**Linux (Debian/Ubuntu)**

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf libgtk-3-dev build-essential
```

Fedora: `sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel patchelf`
Arch: `sudo pacman -S webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg patchelf base-devel`

**Windows**

- Visual Studio **Build Tools** with the "Desktop development with C++" workload (MSVC toolchain)
- WebView2 Runtime (preinstalled on Windows 11)

### Run or package

```sh
# Run directly (all platforms) — the GUI is excluded from default builds
cargo run -p to_markdown_gui

# Package installers/bundles for the current OS
cd gui
cargo tauri build
# → target/release/bundle/  (dmg+app on macOS, AppImage+deb on Linux, msi+exe on Windows)
```

---

## After installing

- Open a folder (vault) or a file, or just drag one onto the window.
- See the [User Guide](USER_GUIDE.md) for everything the app can do, and [GUI.md](GUI.md) for architecture/build details.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| macOS: "app is damaged / developer cannot be verified" | Right-click → Open, or `xattr -dr com.apple.quarantine <app>` |
| Windows: SmartScreen blocks the installer | More info → Run anyway |
| Windows: blank window | Install the WebView2 Runtime |
| Linux: AppImage won't start | Install the WebKitGTK 4.1 runtime packages for your distro |
| Linux: `error while loading shared libraries: libwebkit2gtk` | Same as above (`libwebkit2gtk-4.1-0` on Debian/Ubuntu) |
| Build: `failed to run custom build command for glib-sys` | Linux build prerequisites missing (see above) |
| Build: linker `link.exe` not found (Windows) | Install VS Build Tools C++ workload, restart the shell |
