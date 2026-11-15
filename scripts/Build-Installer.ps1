[CmdletBinding()]
param([string]$IsccPath)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $IsccPath) {
    $candidates = @(
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe')
    )
    $IsccPath = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $IsccPath -or -not (Test-Path -LiteralPath $IsccPath)) { throw 'Install Inno Setup 6, or supply -IsccPath.' }
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargoCommand) { $cargoPath = $cargoCommand.Source }
else { $cargoPath = (Get-Item -LiteralPath (Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe')).FullName }
Push-Location $root
try {
    & $cargoPath build --locked --release
    if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed.' }
    $versionLine = Get-Content -LiteralPath (Join-Path $root 'Cargo.toml') | Where-Object { $_ -match '^version\s*=' } | Select-Object -First 1
    $version = ($versionLine -split '"')[1]
    & $IsccPath "/DAppVersion=$version" (Join-Path $root 'installer\stereo-nudge.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
    $dist = Join-Path $root 'dist'
    Copy-Item -LiteralPath (Join-Path $root 'target\release\stereo-nudge.exe') -Destination (Join-Path $dist 'stereo-nudge.exe') -Force
    foreach ($name in @('LICENSE','README.md')) { Copy-Item -LiteralPath (Join-Path $root $name) -Destination (Join-Path $dist $name) -Force }
    New-Item -ItemType Directory -Path (Join-Path $dist 'docs') -Force | Out-Null
    Get-ChildItem -LiteralPath (Join-Path $root 'docs') -Filter '*.md' | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $dist ('docs\'+$_.Name)) -Force }
    Get-Item -LiteralPath (Join-Path $root "dist\stereo-nudge-$version-windows-x64-setup.exe")
} finally { Pop-Location }
