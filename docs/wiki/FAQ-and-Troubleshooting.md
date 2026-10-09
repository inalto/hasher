# FAQ and Troubleshooting

## Choosing and understanding hashes

### Which algorithm should I use?

- To check a **download against a published hash**: whatever the publisher uses, usually **SHA-256** (on by default).
- To detect **tampering** with your own files: SHA-256, SHA-512, SHA3-256 or BLAKE3. Avoid MD5 and SHA-1 for security decisions.
- To check for **accidental corruption** as fast as possible: CRC32 or xxHash64, or BLAKE3 if you want something cryptographic and fast.
- For **eMule/eDonkey** files: ED2K.

The comparison table is on [[Algorithms]], the security classes at the end of [[Verifying Hashes]].

### Is ED2K compatible with eMule links?

Yes. Hasher uses the eMule convention (9,728,000-byte chunks, MD4 of the chunk hashes, with the empty-chunk rule for exact multiples), so the 32 hex characters are the hash eMule shows. A full link has the form `ed2k://|file|<name>|<size in bytes>|<hash>|/`; Hasher shows both the exact size and the hash, but does not build the link or compute AICH hashes.

### Why are CRC32 or xxHash values reversed compared with another tool?

Hasher prints numeric checksums in the canonical **big-endian** form (as in SFV files and `xxhsum`). Some tools print the same number little-endian or as a decimal. Compare the bytes, not the text. CRC64 is **CRC-64/XZ**, not the other common CRC-64 variants.

## Verification problems

### The hash I pasted shows "No match" / "Hash doesn't match"

Go through this list:

1. **Is the algorithm computed?** The pasted hash is compared only with algorithms that were calculated. Tick the algorithm in the top-bar selector and press **Recompute**.
2. **Is it the right file?** Compare the size with the one published. A hash for the archive does not apply to the files extracted from it, and vice versa.
3. **Did the download finish?** A truncated or corrupted file never matches. Download again, ideally over HTTPS.
4. **Was text modified?** Hashes of text files change if line endings are converted (CRLF/LF) or a BOM is added or removed.
5. **Copy/paste artefacts**: stray characters are ignored only if they are spaces, tabs, colons or dashes; quotes or other symbols make the text *Not a hexadecimal hash*. The *Invalid length* message means the pasted text has a number of characters that no supported algorithm produces.
6. **32 or 64 characters can mean several algorithms.** Hasher tries all computed candidates, but a 64-character BLAKE3 hash is only compared if BLAKE3 was computed.

### A checksum file says FAILED or MISSING

- **MISSING** means the listed file was not found. File names are resolved relative to the **folder of the checksum file**: keep the checksum file next to the files, or use absolute paths.
- **FAILED** means the file exists but differs. Check for an incomplete download, then re-download. For a `.sfv` the value is a CRC32.
- A **single** dropped file with a checksum extension is verified, not hashed. To hash such a file itself, drop it together with another file.
- `SHA256SUMS.txt` is not recognised automatically (`.txt`): rename it to `SHA256SUMS.sha256`, or paste one line from it. See [[Verifying Hashes]].

## Speed

### Why is hashing slow?

- **MD2** runs at roughly 7-10 MB/s, and since every selected algorithm sees every block, one slow algorithm slows everything. It is off by default and carries a *slow* badge; untick it. **SHA3-512** is also noticeably slower than SHA-2 or BLAKE3.
- **Disk or network speed**: a hard disk or network share reads at 100 MB/s or less whatever the algorithm. Use *Files in parallel = 1* on spinning disks ([[Settings]]).
- **Many algorithms on few cores**: with all 22 selected, a small CPU becomes the limit. Use the quick selector to compute only what you need.
- **Debug builds** (if you compiled from source with plain `cargo build`) are much slower than `cargo build --release`.
- Real-time antivirus scanning can slow reads of large files.

### How much memory does it use?

Very little. Files are streamed in 1 MiB blocks (one buffer per worker), never loaded whole, so multi-gigabyte files are fine.

## Platform issues

### "Created" shows n/a on Linux

The creation (birth) time is not recorded by every file system or kernel interface; Hasher shows *n/a* when the system does not provide it (it relies on the OS's `statx` data). It is a property of the file system, not a bug. *Modified* is always available.

### The window does not open on Linux

Run it from a terminal to see the message, with logging if useful:

```sh
RUST_LOG=debug ./hasher
```

- A message about a missing library (for example `error while loading shared libraries: libgtk-3.so.0`), or a start-up failure mentioning `libxkbcommon` or OpenGL: install **GTK 3**, **libxkbcommon** and **Mesa OpenGL** ([[Installation]]). `ldd ./hasher | grep "not found"` lists the libraries the loader cannot find.
- `GLIBC_2.35 not found`: your distribution is older than the build machine (Ubuntu 22.04). Build from source ([[Building from Source]]).
- **OpenGL problems** (blank window, crash on start, virtual machines, remote desktops): force software rendering with `LIBGL_ALWAYS_SOFTWARE=1 ./hasher`.
- **Wayland vs X11**: Hasher supports both. If one misbehaves, force X11 with `env -u WAYLAND_DISPLAY ./hasher`.
- No display at all (SSH without forwarding): Hasher is a graphical program and needs a desktop session; for terminals use `sha256sum`, `md5sum`, etc.

### macOS says the app is damaged or from an unidentified developer

The app is not notarized. Remove the quarantine flag: `xattr -dr com.apple.quarantine Hasher.app`, or right-click → **Open** the first time. See [[Installation]].

### Windows SmartScreen blocks hasher.exe

The executable is not code-signed. Choose **More info → Run anyway**, or unblock the downloaded zip (right-click → Properties → Unblock) before extracting it.

### Drag and drop does not work

Dropping from a program running with higher privileges (for example an elevated Explorer) onto a normal-privilege window is blocked by Windows; run both at the same level or use **Ctrl+O**. On Linux, make sure the file manager and Hasher run in the same kind of session (X11/Wayland).

## Settings

### Where are my settings?

In `hasher.settings.json`: next to the executable if that folder is writable (portable mode), otherwise in the user configuration folder. The exact path is shown at the bottom of the Settings window; the lookup order is in [[Installation]].

### How do I reset the settings?

Close Hasher and delete `hasher.settings.json`. It is recreated with defaults. You can also edit single keys; see [[Settings]]. A corrupted file is ignored and replaced at the next save.

### Hidden files or links are missing from a folder

Folder scans skip hidden files (names starting with `.`, or the Hidden attribute on Windows) and symbolic links by default. Enable *Include hidden files* and *Follow symbolic links* in Settings. Files you drop individually are always included.

## Usage

### Can I use Hasher from the command line?

Only partly. The binary accepts **file and folder paths** (opened as if dropped) and two options for automated screenshots, `--screenshot <file.png>` and `--screenshot-frames <n>`. There is **no console hashing mode**: results are shown in the window. In a terminal use `sha256sum`, `md5sum`, `b2sum`, ... and verify their output files with Hasher, or the other way round with the checksum files Hasher exports ([[Exporting]]).

### Does Hasher send my files or data anywhere?

No. It has no network code; the only external actions are opening your file manager on request and opening the company website if you click the credit link.

### Why does the application stay in English even though my system is French, German or Spanish?

All five languages are fully translated. If a single string still appears in English, that key is missing from the translation file and falls back to English: please report it. See [[Settings]].

## Reporting a problem

Open an issue at <https://github.com/inalto/hasher/issues> with: the Hasher version (**Help → About**), your OS, the steps to reproduce, and any message printed when you start Hasher from a terminal with `RUST_LOG=debug`. Please do not attach files you cannot share; the hash values and sizes are usually enough.
