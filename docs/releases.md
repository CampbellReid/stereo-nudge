# Releases and rebuilding

Copyright (C) 2026 Campbell Reid. GPL-3.0-or-later.

## Download and install

The [latest installer](https://github.com/CampbellReid/stereo-nudge/releases/latest/download/stereo-nudge-windows-x64-setup.exe)
is a single file containing the app, uninstaller, documentation and licence notices.
It installs per user without administrator privileges. Startup is optional.
Reinstalling a newer version preserves the saved device choices.

GitHub Release downloads follow repository visibility: while the repository is
private, only people with access can download them. Making the repository public
is a separate, manual decision. The workflow never changes repository visibility.

Each release contains:

- `stereo-nudge-windows-x64-setup.exe`: the installer, with a stable filename for the latest-download link.
- `stereo-nudge-VERSION-source.tar.gz`: the app, build scripts, locked dependency source and build information.
- `SHA256SUMS.txt`: SHA-256 hashes for both downloads.

In PowerShell, compare `(Get-FileHash .\stereo-nudge-windows-x64-setup.exe -Algorithm SHA256).Hash`
with its entry in `SHA256SUMS.txt`.

## Automation

`.github/workflows/build.yml` checks formatting, runs Clippy and tests, builds the
release executable and installer, and tests install/startup/uninstall behavior
on a clean Windows runner. Only a successful build on `main` can publish a release.
The release job attaches all files to a draft, then publishes it so readers get
a complete download. Pull requests and other branches get build artifacts only.

The version is `MAJOR.MINOR.RUN_NUMBER`. Major and minor come from `Cargo.toml`;
the patch is GitHub's monotonically increasing workflow run number. A rerun keeps
the same number and does not replace an already published release. Workflow
dispatch on `main` also produces a new release. The tag targets the exact input
commit. The release build passes its generated version into the app and installer;
the checked-in Cargo version remains the development baseline.

Builds queue by branch to avoid overlapping publication. The job token can write
repository contents only in the publish job. There is no app-side update checker,
network telemetry or background updater.

## Build from a release source archive

Install stable Rust with the x64 MSVC target, the Visual Studio C++ build tools
with a Windows SDK, and [Inno Setup 6.7.3](https://github.com/jrsoftware/issrc/releases/tag/is-6_7_3).
`BUILD-INFO.json` records the exact Rust compiler version, release version and
source commit used for that download. For the same compiler, install the recorded
toolchain with rustup and make it active in the extracted directory.

The source archive contains `vendor/` and a Cargo configuration pointing to it,
so compiling the app's Rust dependencies does not need crates.io access.
After extracting the archive, open PowerShell in its top directory:

```powershell
$build = Get-Content .\BUILD-INFO.json -Raw | ConvertFrom-Json
cargo test --locked --offline
cargo clippy --locked --offline --all-targets -- -D warnings
.\scripts\Build-Installer.ps1 -Version $build.version
```

The resulting installer is `dist\stereo-nudge-VERSION-windows-x64-setup.exe`.
The app and installer show the same version. Toolchain and installer timestamps
can affect the resulting bytes; these are rebuild instructions, not a claim of
bit-for-bit reproducibility.

From a Git checkout, `scripts/Build-Release.ps1 -Version 0.3.123` creates all three
release assets locally. It vendors only locked dependencies and copies an explicit
source allowlist, excluding settings, diagnostic captures, logs and Git metadata.
The packaging tools `git` and `tar` must be on PATH.
