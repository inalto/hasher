# CI/CD and Self-Hosted Runners

Three GitHub Actions workflows live in `.github/workflows/`:

| Workflow | File | Runs on | Purpose |
|---|---|---|---|
| **CI** | `ci.yml` | pushes to `main`, pull requests to `main` | Format, lint, tests; release-profile build on three OSes |
| **Release** | `release.yml` | tags `v*`, or manually | Builds the packages and publishes a GitHub Release |
| **Wiki sync** | `wiki.yml` | pushes to `main` that touch `docs/wiki/**`, or manually | Publishes `docs/wiki/` to the GitHub wiki |

Dependency updates are handled by Dependabot (see the end of this page).

## CI

Two jobs:

1. **`check` - fmt / clippy / test** on a **GitHub-hosted `ubuntu-latest`** runner, always. It installs the system libraries (`libgtk-3-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libxcb-*-dev`, `libssl-dev`, `pkg-config`), the stable toolchain with `rustfmt` and `clippy`, restores the cargo cache and runs `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`. It runs for pull requests too, so it is safe for forks (hosted runners are disposable).
2. **`build-matrix` - build (linux / windows / macos)**: `cargo build --release` on the three platforms, **only for pushes** (never for pull requests). No tests here; the goal is to catch platform-specific compile errors early. Runners come from the repository variables below.

A new push to the same branch cancels the previous CI run (`concurrency`).

## Release

Triggered by pushing a tag `vX.Y.Z`, or by hand (**Actions → Release → Run workflow**).

| Job | What it does |
|---|---|
| `build` (matrix) | **linux-x86_64** on `ubuntu-latest` inside a `rockylinux:8` container (glibc 2.28, for wide compatibility; the container is skipped when `RUNNER_LINUX` is set) → `hasher-<version>-linux-x86_64.tar.gz`; **windows-x86_64** on `windows-latest` → `hasher-<version>-windows-x86_64.zip`; **macos-universal** on `macos-latest`: builds `x86_64-apple-darwin` and `aarch64-apple-darwin`, joins them with `lipo` and bundles `Hasher.app` → `hasher-<version>-macos-universal.zip`. Each package is uploaded as a workflow artifact. |
| `release` | Needs `build`. Downloads all artifacts, writes **`SHA256SUMS.txt`** (`sha256sum hasher-*`) and creates the **GitHub Release** with `generate_release_notes: true`. Runs only for tags or when `publish` is set. |

The version used in the file names comes from the tag (`v1.2.3` → `1.2.3`); for manual runs it is read from `Cargo.toml`. If a tag and `Cargo.toml` disagree the workflow prints a warning. Note that the version **shown in the About box and written in exports** is the one in `Cargo.toml`.

**Manual run.** `workflow_dispatch` has one boolean input, **`publish`** (default off). With it off, the packages are only available as artifacts on the run page (useful to test). With it on, the workflow also creates/updates the release `v<version from Cargo.toml>`.

### How to cut a release

1. Update `version` in `Cargo.toml` (and `Cargo.lock` by building once) and add the section to `CHANGELOG.md`; commit and push to `main`; wait for CI to pass.
2. Tag and push:

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. Watch **Actions → Release**. Three artifacts appear, then the Release page with the archives, `SHA256SUMS.txt` and generated notes. Edit the notes if needed.

## Wiki sync

`wiki.yml` clones `<repo>.wiki.git` with the workflow token, **deletes everything** in it and copies `docs/wiki/` over, then commits and pushes if anything changed. Consequences:

- **One-time prerequisite**: GitHub creates the wiki repository only when its first page exists. Open the repository's **Wiki** tab and click *Create the first page* once (any content); otherwise the workflow fails with "The wiki repository does not exist yet".
- Edit the pages **in the main repository** (`docs/wiki/`), not in the web editor: web edits are overwritten at the next sync.
- Images go in `docs/wiki/images/` and are referenced as `images/<name>.png`.

## Self-hosted runners

By default every job uses a GitHub-hosted runner. To use your own machines (for faster builds, special hardware or to save minutes), define **repository variables** at *Settings → Secrets and variables → Actions → Variables → New repository variable*:

| Variable | Used by | Unset means |
|---|---|---|
| `RUNNER_LINUX` | CI `build-matrix`, Release `build` | `ubuntu-latest` (CI) / `ubuntu-22.04` (Release) |
| `RUNNER_WINDOWS` | same | `windows-latest` |
| `RUNNER_MACOS` | same | `macos-latest` |

Each variable holds **one runner label** (for example `hasher-windows`); the workflows write `runs-on: ${{ vars.RUNNER_WINDOWS || 'windows-latest' }}`. The `check` job and the final `release` job always stay on GitHub-hosted `ubuntu-latest`.

### Registering a runner

1. On GitHub: **Settings → Actions → Runners → New self-hosted runner**. Choose the OS and architecture; the page shows the exact download and configuration commands with a short-lived token.
2. On the machine, follow those steps, adding a **custom label** during configuration: `--labels hasher-windows` (or `hasher-macos`, `hasher-linux`).
3. Install it as a **service** so that it survives reboots: on Windows pass `--runasservice` to `config.cmd` (or use the service option it offers); on Linux/macOS run `sudo ./svc.sh install` then `sudo ./svc.sh start`.
4. Check that the runner shows as *Idle* in the Runners list, then create the variable (`RUNNER_WINDOWS` = `hasher-windows`, etc.).

### Software required on the runner

| Runner | Needed |
|---|---|
| All | **rustup** with the **stable** toolchain on the service user's `PATH`; Git |
| Windows | **Visual Studio Build Tools 2022** with *Desktop development with C++* (MSVC and the Windows 10/11 SDK, providing `link.exe` and `rc.exe`); **Git for Windows** (the workflow uses `bash` for one step); **PowerShell 7** (`pwsh`), which the packaging step requires |
| macOS | **Xcode Command Line Tools** (`xcode-select --install`: `lipo`, `codesign`, `iconutil`, `ditto`); `rustup target add x86_64-apple-darwin aarch64-apple-darwin` (the workflow also installs them if missing) |
| Linux | The development packages of [[Building from Source]] (run `scripts/dev-deps-debian.sh` or `scripts/dev-deps-el9.sh`), because the "install system dependencies" step runs only on GitHub-hosted runners. The Rocky Linux 8 container is not used on a self-hosted runner, so the binary will need a glibc at least as new as the runner's: build on the oldest distribution you want to support. |

### Security

Self-hosted runners execute workflow code on **your** machine, with your network and files in reach.

- **Never expose a self-hosted runner to untrusted pull requests.** In this repository the pull-request workflow (`check`) uses only hosted runners, and `build-matrix` and Release start only on pushes to `main` and on tags, which need write access. But a pull request can *edit the workflow file* and point `runs-on` at your label.
- Therefore either keep the repository **private**, or under **Settings → Actions → General → Fork pull request workflows** require approval for all outside contributors, and limit who has write access.
- Run the runner service as an unprivileged user, on a dedicated or disposable machine/VM if possible, and keep it updated.

## Dependabot

`.github/dependabot.yml` checks weekly for two ecosystems: **Cargo** (all crates grouped into a single pull request, `rust-dependencies`) and **GitHub Actions** (versions of the actions used in the workflows). CI validates each Dependabot pull request like any other.
