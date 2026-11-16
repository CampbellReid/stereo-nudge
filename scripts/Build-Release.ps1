# Copyright (C) 2026 Campbell Reid
# SPDX-License-Identifier: GPL-3.0-or-later
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [string]$Revision,
    [string]$IsccPath
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$cargoPath = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
$rustcPath = Join-Path (Split-Path $cargoPath) 'rustc.exe'
Push-Location $root
try {
    if (-not $Revision) {
        $Revision = & git -c "safe.directory=$($root.Replace('\','/'))" rev-parse HEAD
        if ($LASTEXITCODE -ne 0) { throw 'Supply -Revision when building outside a Git checkout.' }
    }
    if ($Revision -notmatch '^[0-9a-f]{40}$') { throw 'Revision must be a full Git commit SHA.' }
    $setup = & (Join-Path $PSScriptRoot 'Build-Installer.ps1') -Version $Version -IsccPath $IsccPath
    $dist = Join-Path $root 'dist'
    $installer = Join-Path $dist 'stereo-nudge-windows-x64-setup.exe'
    Copy-Item -LiteralPath $setup.FullName -Destination $installer -Force
    # Copy an explicit source allowlist, never local logs, settings or Git credentials.
    $stage = Join-Path $root ('target\release-package\'+[Guid]::NewGuid().ToString('N'))
    $name = "stereo-nudge-$Version-source"
    $sourceRoot = Join-Path $stage $name
    New-Item -ItemType Directory -Path $sourceRoot -Force | Out-Null
    foreach ($entry in @('Cargo.toml','Cargo.lock','LICENSE','README.md','.gitignore','.cargo','.github','src','scripts','installer','docs')) {
        Copy-Item -LiteralPath (Join-Path $root $entry) -Destination $sourceRoot -Recurse -Force
    }
    $compiler = & $rustcPath --version --verbose
    if ($LASTEXITCODE -ne 0) { throw 'Could not record the Rust compiler version.' }
    [ordered]@{ version = $Version; revision = $Revision; rustc = @($compiler); repository = 'https://github.com/CampbellReid/stereo-nudge' } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $sourceRoot 'BUILD-INFO.json') -Encoding utf8
    Copy-Item -LiteralPath (Join-Path $dist 'legal') -Destination (Join-Path $sourceRoot 'third-party-notices') -Recurse
    Push-Location $sourceRoot
    try {
        $vendorConfig = & $cargoPath vendor --locked --offline --versioned-dirs vendor
        if ($LASTEXITCODE -ne 0) { throw 'Could not vendor the corresponding source dependencies.' }
        Add-Content -LiteralPath '.cargo\config.toml' -Value ("`n"+($vendorConfig -join "`n")) -Encoding utf8
        # Check the archive's portable Cargo configuration before publishing it.
        & $cargoPath metadata --locked --offline --format-version 1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Source bundle Cargo configuration is invalid.' }
    } finally { Pop-Location }
    $source = Join-Path $dist "$name.tar.gz"
    & tar -czf $source -C $stage $name
    if ($LASTEXITCODE -ne 0) { throw 'Source archive creation failed.' }
    $checksums = Join-Path $dist 'SHA256SUMS.txt'
    @($installer, $source) | ForEach-Object {
        $hash = (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $(Split-Path -Leaf $_)"
    } | Set-Content -LiteralPath $checksums -Encoding ascii
    [pscustomobject]@{ Version = $Version; Installer = $installer; Source = $source; Checksums = $checksums }
} finally { Pop-Location }
