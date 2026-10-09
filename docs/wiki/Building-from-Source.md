# Building from Source

Hasher is a single Cargo crate (library + binary). There is no Node.js, no WebView and no code generator: a Rust toolchain and the platform's native build tools are enough.

## Prerequisites

Common to all platforms: [Rust](https://rustup.rs) **stable 1.95 or newer** (`rust-version` in `Cargo.toml`), installed with `rustup`, and Git.

### Linux

You need a C toolchain, `pkg-config` and the development files of GTK 3 (native file dialogs through `rfd`), libxkbcommon, Wayland and OpenGL. Two helper scripts install everything, including Xvfb for headless screenshots:

```sh
scripts/dev-deps-debian.sh     # Debian / Ubuntu: build-essential, libgtk-3-dev, libxkbcommon-dev,
                               #   libwayland-dev, libgl1-mesa-dev, libxcb-*-dev, xvfb ...
scripts/dev-deps-el9.sh        # AlmaLinux / RHEL / Rocky 9: gcc, gtk3-devel, libxkbcommon-devel,
                               #   wayland-devel, mesa-libGL-devel, Xvfb ...
```

### Windows

Install the [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) (2022) with the **Desktop development with C++** workload: the MSVC toolchain and the Windows 10/11 SDK, which provides `link.exe` and `rc.exe`. Use the default `x86_64-pc-windows-msvc` Rust host. `build.rs` uses `rc.exe` (through `winresource`) to embed the icon and version information in `hasher.exe`; if it is missing the build still succeeds with a warning but the executable has no icon.

### macOS

Install the Xcode Command Line Tools: `xcode-select --install`.

## Build and run

```sh
git clone https://github.com/inalto/hasher.git
cd hasher
cargo build --release          # target/release/hasher   (target\release\hasher.exe on Windows)
cargo run --release -- some-file.iso
```

- **Release profile**: `opt-level = 3`, fat LTO, `codegen-units = 1`, `strip = true`, `panic = "abort"`. Linking with fat LTO needs a few GB of RAM; on a small machine use `CARGO_BUILD_JOBS=2`.
- **Dev profile** (`cargo build`): `opt-level = 1` for the crate and `opt-level = 2` for dependencies, so hashing is reasonably fast even in debug builds. The binary is `target/debug/hasher`.
- Release builds on Windows use the GUI subsystem (no console window); debug builds keep the console, handy for `RUST_LOG=debug`.

## Checks

```sh
cargo test                                    # unit tests + tests/*.rs
cargo fmt --all --check                       # rustfmt.toml: max_width = 120
cargo clippy --all-targets -- -D warnings     # must be warning-free
```

CI runs exactly these three commands ([[CI-CD and Self-Hosted Runners]]). Some integration tests compare Hasher with system tools (`md5sum`, `sha256sum`, `b2sum`, `python3`, `openssl`); tools that are not installed are simply skipped. See [[Architecture]] for the test strategy.

## Headless screenshots (Linux)

Hasher can render one frame and save it, which is how the images of this wiki are made. `scripts/screenshot.sh` starts Xvfb (if display `:99` is not already served), forces Mesa software rendering (`LIBGL_ALWAYS_SOFTWARE=1`), points the settings to a temporary file so your real ones are untouched, and gives up after 90 seconds.

```sh
cargo build                                              # the script uses target/debug/hasher
scripts/screenshot.sh /tmp/empty.png                     # empty window
HASHER_THEME=dark scripts/screenshot.sh /tmp/list.png a.iso b.iso c.iso
scripts/screenshot.sh /tmp/slow.png --screenshot-frames 120 big.bin
```

Command-line options of the binary:

```text
hasher [--screenshot <file.png>] [--screenshot-frames <n>] [file or folder ...]
```

`--screenshot` saves the window to a PNG and exits, once at least `--screenshot-frames` frames (default 30) have been drawn and, if files were given, their hashing has finished; every other argument is a path to open. Hasher has **no command-line hashing mode**.

Environment variables for the script and the test hooks:

| Variable | Meaning |
|---|---|
| `HASHER_THEME=light\|dark` | Force the theme (the binary also accepts `system`) |
| `HASHER_BIN`, `XVFB_DISPLAY`, `XVFB_SCREEN` | Binary to run (default `target/debug/hasher`), display number (`99`), geometry (`1280x800x24`) |
| `HASHER_SETTINGS_PATH` | Settings file to use (the script sets a temporary one) |
| `HASHER_VERIFY_INPUT=<hash>` | Pre-fill the Verify panel (and open it) |
| `HASHER_UI=a,b,...` | Scripted UI states: `settings`, `help`, `shortcuts`, `about`, `verify`, `overlay`, `algos`, `cancel`, `nowait`, and, once hashing is done, `export`, `select=0+1`, `expand=0` |

```sh
HASHER_UI=settings scripts/screenshot.sh docs/wiki/images/settings.png
HASHER_UI=select=0+1 scripts/screenshot.sh docs/wiki/images/verify.png a.bin b.bin
```

## Packaging

Packages are written to `dist/` (git-ignored). All scripts take the version without the `v`.

```sh
cargo build --release
scripts/package-linux.sh 0.1.0                  # dist/hasher-0.1.0-linux-x86_64.tar.gz
pwsh scripts/package-windows.ps1 0.1.0          # dist\hasher-0.1.0-windows-x86_64.zip
```

For a macOS universal app (macOS only: needs `lipo`, `iconutil`, `ditto`, optionally `codesign`):

```sh
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
mkdir -p target/universal
lipo -create -output target/universal/hasher \
  target/x86_64-apple-darwin/release/hasher target/aarch64-apple-darwin/release/hasher
scripts/bundle-macos.sh 0.1.0 target/universal/hasher   # dist/Hasher.app + hasher-0.1.0-macos-universal.zip
```

`bundle-macos.sh` builds the `.icns` with `scripts/make-icns.sh`, writes `Info.plist` (bundle id `net.martini-multimedia.hasher`, minimum macOS 10.15) and applies an ad-hoc signature. The app is neither signed with a Developer ID nor notarized. In practice you rarely run these by hand: tagging a release does it all ([[CI-CD and Self-Hosted Runners]]).

## Project layout

```text
hasher/
├── Cargo.toml, Cargo.lock       dependencies, release profile (LTO, strip)
├── build.rs                     Windows only: icon and version info (winresource)
├── rustfmt.toml                 max_width = 120
├── assets/
│   ├── brand/                   logo.svg, mark.svg
│   ├── icons/                   icon.svg, icon-16 ... icon-1024.png, icon.ico
│   ├── fonts/                   Inter, JetBrains Mono + licences
│   └── linux/                   hasher.desktop
├── locales/                     it.yml en.yml fr.yml de.yml es.yml (rust-i18n)
├── src/
│   ├── main.rs                  thin binary: logger + CliArgs + ui::run
│   ├── lib.rs                   library crate: constants, i18n! setup
│   ├── backend/                 no UI code
│   │   ├── algo.rs              Algo enum, ids, lengths, groups, hex normalisation
│   │   ├── hasher.rs            StreamHasher trait and implementations (incl. ED2K)
│   │   ├── engine.rs            sessions, worker threads, events, cancellation
│   │   ├── checksum_file.rs     .md5/.sha256/.sfv parser (GNU, BSD, SFV)
│   │   ├── fileinfo.rs          metadata and per-OS attributes
│   │   ├── settings.rs          Settings + portable persistence
│   │   └── export/              txt_fmt csv_fmt json_fmt xlsx_fmt docx_fmt checksum_fmt
│   └── ui/                      egui: mod (HasherApp), theme, anim, widgets, topbar, dropzone,
│                                single, list, verify, settings, help, export, format, statusbar
├── tests/                       hash_vectors.rs engine.rs checksum_file.rs export.rs
├── scripts/                     dev-deps-*.sh, screenshot.sh, package-*.sh/.ps1,
│                                bundle-macos.sh, make-icns.sh
├── docs/PIANO.md                development plan (Italian)
├── docs/wiki/                   the source of this wiki (synced by CI)
└── .github/                     workflows/ci.yml release.yml wiki.yml, dependabot.yml
```
