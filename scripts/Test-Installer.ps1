# Copyright (C) 2026 Campbell Reid
# SPDX-License-Identifier: GPL-3.0-or-later
[CmdletBinding()]
param([Parameter(Mandatory)][string]$SetupPath)
$ErrorActionPreference = 'Stop'
$appKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{9A4919CB-90E6-4DA3-BAB0-490E1971952D}_is1'
$startupLink = Join-Path ([Environment]::GetFolderPath('Startup')) 'Stereo Nudge.lnk'
$menuDir = Join-Path ([Environment]::GetFolderPath('Programs')) 'Stereo Nudge'
if ((Test-Path -LiteralPath $appKey) -or (Test-Path -LiteralPath $startupLink) -or (Test-Path -LiteralPath $menuDir)) {
    throw 'This lifecycle test requires no existing Stereo Nudge installation or shortcuts.'
}
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('StereoNudge-Installer-Test-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot | Out-Null
$shell = New-Object -ComObject WScript.Shell
function Run-Setup([string]$Tasks) {
    $arguments = '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /DIR="'+$testRoot+'" /TASKS="'+$Tasks+'" /LOG="'+(Join-Path $testRoot ('setup-'+$Tasks+'.log'))+'"'
    $process = Start-Process -FilePath $SetupPath -ArgumentList $arguments -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Setup failed with exit code $($process.ExitCode)" }
}
try {
    Run-Setup 'startup'
    $registration = Get-ItemProperty -LiteralPath $appKey
    if ($registration.Publisher -ne 'Campbell Reid') { throw 'Installer author attribution is missing.' }
    $expectedVersion = (Get-Item -LiteralPath $SetupPath).VersionInfo.ProductVersion.Trim()
    if ($registration.DisplayVersion -ne $expectedVersion) { throw 'Installed version differs from the installer version.' }
    if ((Get-Content -LiteralPath (Join-Path $testRoot 'LICENSE') -Raw) -notmatch 'GNU GENERAL PUBLIC LICENSE') { throw 'The GPL licence is missing.' }
    if ((Get-Content -LiteralPath (Join-Path $testRoot 'Project.url') -Raw) -notmatch 'URL=https://github.com/CampbellReid/stereo-nudge') { throw 'Installed project link is incorrect.' }
    foreach ($notice in @('THIRD-PARTY-NOTICES.txt','licenses\rust\COPYRIGHT-library.html')) {
        if (-not (Test-Path -LiteralPath (Join-Path $testRoot $notice))) { throw "Missing third-party notice: $notice" }
    }
    if (-not (Test-Path -LiteralPath $startupLink)) { throw 'Startup option did not create its shortcut.' }
    $shortcut = $shell.CreateShortcut($startupLink)
    if ($shortcut.TargetPath -ne (Join-Path $testRoot 'stereo-nudge.exe')) { throw 'Startup shortcut targets the wrong executable.' }
    if ($shortcut.Arguments -notmatch '--seconds 0' -or $shortcut.Arguments -notmatch '--startup-wait-seconds 60') { throw 'Startup shortcut arguments are incorrect.' }
    if ($shortcut.Arguments -match '--address') { throw 'Startup must use saved tray choices, not force a device address.' }
    Run-Setup ''
    if (Test-Path -LiteralPath $startupLink) { throw 'Clearing the startup option did not remove its shortcut.' }
    Run-Setup 'startup'
    if (-not (Test-Path -LiteralPath $startupLink)) { throw 'Startup option could not be re-enabled.' }
    $uninstaller = Join-Path $testRoot 'unins000.exe'
    $process = Start-Process -FilePath $uninstaller -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Uninstall failed with exit code $($process.ExitCode)" }
    if ((Test-Path -LiteralPath $startupLink) -or (Test-Path -LiteralPath $appKey) -or (Test-Path -LiteralPath (Join-Path $testRoot 'stereo-nudge.exe'))) { throw 'Uninstaller left installed application or startup state behind.' }
    'PASS: version, attribution, GPL/notices, per-user install, startup on/off/re-enable, and uninstall.'
} finally {
    # Test logs remain in the named temp directory. Setup never changes device choices.
    [Runtime.InteropServices.Marshal]::ReleaseComObject($shell) | Out-Null
    "Test logs: $testRoot"
}
