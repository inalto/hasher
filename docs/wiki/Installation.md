# Installation

Hasher is distributed as a **portable package**: there is no installer, no service, no registry entry and no administrator rights are needed. Download the package for your system from the [Releases page](https://github.com/inalto/hasher/releases).

| Platform | Package | Contents |
|---|---|---|
| Windows (x86_64) | `hasher-<version>-windows-x86_64.zip` | `hasher.exe`, `README.md`, `LICENSES-fonts\` |
| macOS (Intel + Apple Silicon) | `hasher-<version>-macos-universal.zip` | `Hasher.app` (universal binary) |
| Linux (x86_64) | `hasher-<version>-linux-x86_64.tar.gz` | folder with `hasher`, `README.md`, `hasher.desktop`, `icon.png`, `LICENSES-fonts/` |

Every release also contains `SHA256SUMS.txt`, see [Verify your download](#verify-your-download).

## Windows

1. Unzip the archive anywhere (Desktop, `C:\Tools`, a USB stick...).
2. Double-click `hasher.exe`.

The executable is **not digitally signed**, so the first time you run it Microsoft SmartScreen may show *"Windows protected your PC"*. Click **More info** and then **Run anyway**. Alternatively, right-click the downloaded `.zip` → **Properties** → tick **Unblock** *before* extracting it.

## macOS

1. Unzip the archive and move `Hasher.app` wherever you like (for example *Applications*).
2. The app is **unsigned and not notarized** (it only carries an ad-hoc signature so that it can run on Apple Silicon), so Gatekeeper blocks the first launch. Remove the quarantine attribute once:

   ```sh
   xattr -dr com.apple.quarantine Hasher.app
   ```

   or right-click `Hasher.app` → **Open** → **Open**, or use *System Settings → Privacy & Security → Open Anyway*.

The bundle requires macOS 10.15 (Catalina) or later and runs natively on both Intel and Apple Silicon.

## Linux

```sh
tar xzf hasher-<version>-linux-x86_64.tar.gz
cd hasher-<version>-linux-x86_64
./hasher
```

**Runtime requirements.** GTK 3 (used for the native open/save dialogs), an X11 or Wayland session, OpenGL (Mesa is fine) and `libxkbcommon`. They are present on any desktop distribution; on a minimal system install them, for example:

| Distribution | Packages |
|---|---|
| Debian / Ubuntu | `libgtk-3-0 libgl1 libxkbcommon0` |
| Fedora / RHEL / AlmaLinux / Rocky | `gtk3 mesa-libGL libxkbcommon` |

The release binary is built on Ubuntu 22.04, so it needs **glibc 2.35 or newer**.

### Add Hasher to the application menu (optional)

The `.desktop` file starts the program with `Exec=hasher %F`, so `hasher` must be on your `PATH`. For a per-user installation:

```sh
install -Dm755 hasher         ~/.local/bin/hasher
install -Dm644 icon.png       ~/.local/share/icons/hicolor/256x256/apps/hasher.png
install -Dm644 hasher.desktop ~/.local/share/applications/hasher.desktop
```

Make sure `~/.local/bin` is in your `PATH`, then log out and in again (or run `update-desktop-database ~/.local/share/applications`) if the entry does not show up right away.

## Where settings are stored

Hasher keeps a single file, `hasher.settings.json`. It is looked up in this order:

1. The path in the **`HASHER_SETTINGS_PATH`** environment variable, if set (useful for tests and for keeping several profiles).
2. **Portable mode**: next to the executable, if the file already exists there or the folder is writable. On macOS, inside `Hasher.app`, "next to the executable" means the folder that *contains* `Hasher.app`.
3. The per-user configuration folder, under `MartiniMultimedia/Hasher/`:

   | OS | Folder |
   |---|---|
   | Windows | `%APPDATA%\MartiniMultimedia\Hasher\` |
   | macOS | `~/Library/Application Support/MartiniMultimedia/Hasher/` |
   | Linux | `~/.config/MartiniMultimedia/Hasher/` (or `$XDG_CONFIG_HOME/...`) |

4. The current directory, if the system has no configuration folder.

The **Settings** window (gear icon) shows the exact path in use at the bottom, with a badge: *Portable* or *User profile*. Details and the file format are on the [[Settings]] page.

> Consequence of the rule above: if you put the executable in a folder you can write to (for example `~/.local/bin`, or `/Applications` on a Mac where your account is an administrator), the settings file is created *there*. Put the executable in a read-only location such as `/usr/local/bin` to use the per-user folder instead, or set `HASHER_SETTINGS_PATH`.

## Verify your download

`SHA256SUMS.txt` is in GNU format (`hash  file name`). The easiest ways to check an archive:

- With Hasher itself: drop the downloaded archive onto the window, then **copy the matching line (or just the hash) from `SHA256SUMS.txt` and press Ctrl+V**. The algorithm is recognised from the length and you get a green *Match* or a red *No match*. See [[Verifying Hashes]].
- On the command line: `sha256sum -c --ignore-missing SHA256SUMS.txt` (Linux), `shasum -a 256 -c SHA256SUMS.txt` (macOS) or `Get-FileHash <file>` (PowerShell).

## Updating

Hasher has no built-in updater and makes no network connection of its own. To update, download the new package and replace the executable (or `Hasher.app`). In portable mode keep `hasher.settings.json` where it is: it lives next to the executable, so copy it over if you unpack into a new folder. Release notes are on the Releases page and in `CHANGELOG.md`.

## Uninstalling

Delete the folder (Windows/Linux) or `Hasher.app` (macOS), then the settings file if you do not want to keep it (see the table above). If you installed the menu entry on Linux also remove `~/.local/bin/hasher`, `~/.local/share/applications/hasher.desktop` and `~/.local/share/icons/hicolor/256x256/apps/hasher.png`. Nothing else is written to your system.

Next: [[User Guide]].
