[CmdletBinding()]
param(
    [int]$TargetProcessId,
    [ValidateRange(5,3600)][int]$DurationSeconds = 60,
    [ValidateRange(100,10000)][int]$IntervalMilliseconds = 1000,
    [string]$OutputDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'logs')
)
$ErrorActionPreference = 'Stop'
if (-not $TargetProcessId) {
    $matches = @(Get-Process -Name stereo-nudge -ErrorAction SilentlyContinue)
    if ($matches.Count -ne 1) { throw 'Supply -TargetProcessId when there is not exactly one running stereo-nudge process.' }
    $TargetProcessId = $matches[0].Id
}
$target = Get-Process -Id $TargetProcessId
$target.Refresh()
$startCpu = $target.TotalProcessorTime.TotalSeconds
$clock = [Diagnostics.Stopwatch]::StartNew()
$samples = [Collections.Generic.List[object]]::new()
do {
    $target.Refresh()
    if ($target.HasExited) { throw 'The target process exited during measurement.' }
    $samples.Add([pscustomobject]@{
        TimestampUtc = [DateTime]::UtcNow.ToString('o')
        ElapsedSeconds = $clock.Elapsed.TotalSeconds
        WorkingSetBytes = $target.WorkingSet64
        PrivateCommitBytes = $target.PrivateMemorySize64
        LifetimePeakWorkingSetBytes = $target.PeakWorkingSet64
        CpuSeconds = $target.TotalProcessorTime.TotalSeconds
    })
    $remaining = $DurationSeconds * 1000 - $clock.Elapsed.TotalMilliseconds
    if ($remaining -gt 0) { Start-Sleep -Milliseconds ([int][Math]::Min($IntervalMilliseconds, $remaining)) }
} while ($clock.Elapsed.TotalSeconds -lt $DurationSeconds)
$target.Refresh()
$elapsed = $clock.Elapsed.TotalSeconds
$cpuSeconds = $target.TotalProcessorTime.TotalSeconds - $startCpu
$working = $samples | Measure-Object -Property WorkingSetBytes -Minimum -Maximum -Average
$private = $samples | Measure-Object -Property PrivateCommitBytes -Minimum -Maximum -Average
$summary = [ordered]@{
    ProcessId = $TargetProcessId
    DurationSeconds = [Math]::Round($elapsed,3)
    Samples = $samples.Count
    AverageWorkingSetMiB = [Math]::Round($working.Average / 1MB,3)
    MinimumWorkingSetMiB = [Math]::Round($working.Minimum / 1MB,3)
    MaximumWorkingSetMiB = [Math]::Round($working.Maximum / 1MB,3)
    AveragePrivateCommitMiB = [Math]::Round($private.Average / 1MB,3)
    MaximumPrivateCommitMiB = [Math]::Round($private.Maximum / 1MB,3)
    LifetimePeakWorkingSetMiB = [Math]::Round($target.PeakWorkingSet64 / 1MB,3)
    CpuTimeMilliseconds = [Math]::Round($cpuSeconds * 1000,3)
    AverageCpuPercentOneCore = [Math]::Round(100 * $cpuSeconds / $elapsed,4)
    AverageCpuPercentWholeMachine = [Math]::Round(100 * $cpuSeconds / $elapsed / [Environment]::ProcessorCount,4)
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$stamp = Get-Date -Format yyyyMMdd-HHmmss
$samples | Export-Csv -LiteralPath (Join-Path $OutputDirectory "runtime-$stamp.csv") -NoTypeInformation
$summary | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $OutputDirectory "runtime-$stamp.json") -Encoding utf8
[pscustomobject]$summary
