param(
    [string]$BuildDirectory = (Join-Path $PSScriptRoot "../target/x86_64-pc-windows-msvc/release")
)
$ErrorActionPreference = "Stop"
$source = Join-Path (Resolve-Path $BuildDirectory).Path "padmux.exe"
$trial = Join-Path $env:TEMP ("padmux-gui-check-" + [guid]::NewGuid())
New-Item -ItemType Directory $trial | Out-Null
$exe = Join-Path $trial "padmux.exe"
Copy-Item $source $exe
# TEMP-only configuration/profile, no controller acquisition or OS input.
$cases = @(
    @{Name="startup"; Flags=@("--startup-test", "--self-test", "--shell-test"); Marker="shell self-test passed"},
    @{Name="updater"; Flags=@("--update-test"); Marker="updater IPC self-test passed"}
)
foreach ($case in $cases) {
    $log = Join-Path $trial ($case.Name + ".log")
    $errors = Join-Path $trial ($case.Name + ".err")
    $process = Start-Process $exe -ArgumentList (@("--diagnostic") + $case.Flags + @("--seconds", "30")) -RedirectStandardOutput $log -RedirectStandardError $errors -PassThru
    # Keep the native handle alive so async GUI exit status remains available.
    $ownedHandle = $process.Handle
    if (!$process.WaitForExit(40000)) { throw "GUI check exceeded its own timeout; logs: $trial" }
    if ($process.ExitCode -ne 0 -or (Get-Item $errors).Length -ne 0 -or !(Select-String -Path $log -SimpleMatch $case.Marker)) {
        Get-Content $log
        Get-Content $errors
        throw "GUI check failed: $($case.Name); logs: $trial"
    }
    Write-Output "GUI check passed: $($case.Name)"
}
# Retain logs for review; no user settings/profile are written or removed.
Write-Output "GUI check logs: $trial"
