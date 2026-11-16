# Copyright (C) 2026 Campbell Reid
# SPDX-License-Identifier: GPL-3.0-or-later
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OutDirectory,
    [Parameter(Mandatory)][string]$InnoLicensePath,
    [Parameter(Mandatory)][string]$CargoPath
)
$ErrorActionPreference = 'Stop'
$metadataText = & $CargoPath metadata --locked --offline --format-version 1
if ($LASTEXITCODE -ne 0) { throw 'Could not read locked dependency metadata.' }
$metadata = $metadataText | ConvertFrom-Json
New-Item -ItemType Directory -Path $OutDirectory -Force | Out-Null
$text = [Text.StringBuilder]::new()
[void]$text.AppendLine(@'
Stereo Nudge - third-party notices
=================================

Stereo Nudge: Copyright (C) 2026 Campbell Reid. GPL-3.0-or-later.
The following components retain their original copyrights and licences.
Dual MIT/Apache-2.0 dependencies are used under their MIT option here.
Build-time dependencies are included for completeness.

Rust standard library notices and licence texts are installed in licenses/rust.
The matching release source archive includes locked, vendored Rust dependencies.
Source and releases: https://github.com/CampbellReid/stereo-nudge

'@)
foreach ($package in ($metadata.packages | Where-Object source | Sort-Object name)) {
    [void]$text.AppendLine("`n$($package.name) $($package.version) - $($package.license)")
    $directory = Split-Path $package.manifest_path
    $licenses = @(Get-ChildItem -LiteralPath $directory -File | Where-Object { $_.Name -match '^license[-_](mit|unicode)$' })
    if (-not $licenses.Count -or $package.license -notmatch 'MIT') { throw "Review dependency licence: $($package.name)" }
    foreach ($file in $licenses) {
        [void]$text.AppendLine("`n$($file.Name):")
        [void]$text.AppendLine([IO.File]::ReadAllText($file.FullName))
    }
}
[void]$text.AppendLine("`nInstaller: Inno Setup by Jordan Russell and Martijn Laan`nhttps://jrsoftware.org/")
[void]$text.AppendLine([IO.File]::ReadAllText($InnoLicensePath))
[IO.File]::WriteAllText((Join-Path $OutDirectory 'THIRD-PARTY-NOTICES.txt'), $text.ToString())
$rustc = Join-Path (Split-Path $CargoPath) 'rustc.exe'
$sysroot = & $rustc --print sysroot
if ($LASTEXITCODE -ne 0) { throw 'Could not locate Rust standard library notices.' }
$rustDocs = Join-Path $sysroot 'share\doc\rust'
$rustNotices = Join-Path $OutDirectory 'rust'
New-Item -ItemType Directory -Path $rustNotices -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $rustDocs 'COPYRIGHT-library.html') -Destination $rustNotices -Force
Copy-Item -LiteralPath (Join-Path $rustDocs 'licenses') -Destination $rustNotices -Recurse -Force
