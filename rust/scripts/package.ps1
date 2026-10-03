param(
    [string]$BuildDirectory = (Join-Path $PSScriptRoot "../target/x86_64-pc-windows-msvc/release"),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot "../dist")
)
$ErrorActionPreference = "Stop"
$repository = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$build = (Resolve-Path $BuildDirectory).Path
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$output = (Resolve-Path $OutputDirectory).Path
$stage = Join-Path $output (".padmux-rust-package-" + [guid]::NewGuid().ToString())
$package = Join-Path $stage "padmux-windows-x64"
New-Item -ItemType Directory $package | Out-Null
try {
    foreach ($name in @("padmux.exe", "padmux-updater.exe")) {
        Copy-Item (Join-Path $build $name) (Join-Path $package $name)
    }
    foreach ($name in @("LICENSE", "README.md", "README.ja.md", "THIRD_PARTY_NOTICES.md", "LICENSES/WebView2-LICENSE.txt", "LICENSES/WebView2-NOTICE.txt")) {
        $target = Join-Path $package $name
        New-Item -ItemType Directory -Force (Split-Path $target) | Out-Null
        Copy-Item (Join-Path $repository $name) $target
    }
    Copy-Item (Join-Path $repository "rust/THIRD_PARTY_LICENSES.txt") $package
    Add-Content -Encoding UTF8 (Join-Path $package "README.md") "`nThis package uses the Rust backend. Windows 10 or later and Microsoft Edge WebView2 Runtime are required. The executables use a static CRT; Visual C++ Redistributable is not required."
    Add-Content -Encoding UTF8 (Join-Path $package "README.ja.md") "`nこのパッケージはRustバックエンドです。Windows 10以降とMicrosoft Edge WebView2 Runtimeが必要です。CRTは静的リンクされており、Visual C++ Redistributableは不要です。"
    $vswhere = "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/vswhere.exe"
    $visualStudio = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (!$visualStudio) { throw "MSVC installation not found" }
    $dumpbin = Get-ChildItem "$visualStudio/VC/Tools/MSVC/*/bin/Hostx64/x64/dumpbin.exe" | Sort-Object FullName | Select-Object -Last 1 -ExpandProperty FullName
    foreach ($name in @("padmux.exe", "padmux-updater.exe")) {
        $exe = Join-Path $package $name
        $headers = & $dumpbin /headers $exe
        if ($LASTEXITCODE -or !($headers -match "8664 machine") -or !($headers -match "2 subsystem")) { throw "Expected Windows x64 GUI executable: $name" }
        $dependencies = & $dumpbin /dependents $exe
        if ($LASTEXITCODE) { throw "Could not inspect dependencies: $name" }
        if ($dependencies -match "(?i)\b(vcruntime[0-9_]*|msvcp[0-9_]*|ucrtbased)\.dll\b") { throw "Visual C++ Redistributable dependency: $name" }
    }
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = Join-Path $stage "padmux-rust-windows-x64.zip"
    # Framework PowerShell may emit backslashes with CreateFromDirectory.
    # Write standard ZIP entry names explicitly on both Windows PS and pwsh.
    $stream = [System.IO.File]::Open($archive, [System.IO.FileMode]::CreateNew)
    try {
        $zip = [System.IO.Compression.ZipArchive]::new($stream, [System.IO.Compression.ZipArchiveMode]::Create)
        try {
            foreach ($file in Get-ChildItem -LiteralPath $package -Recurse -File) {
                $relative = $file.FullName.Substring($package.Length + 1).Replace('\', '/')
                [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $file.FullName, "padmux-windows-x64/$relative", [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
            }
        } finally { $zip.Dispose() }
    } finally { $stream.Dispose() }
    $zip = [System.IO.Compression.ZipFile]::OpenRead($archive)
    try {
        foreach ($name in @("padmux.exe", "padmux-updater.exe", "LICENSE", "README.md", "README.ja.md", "THIRD_PARTY_NOTICES.md", "THIRD_PARTY_LICENSES.txt", "LICENSES/WebView2-LICENSE.txt", "LICENSES/WebView2-NOTICE.txt")) {
            if ($zip.Entries.FullName -notcontains "padmux-windows-x64/$name") { throw "Missing ZIP entry: $name" }
        }
    } finally { $zip.Dispose() }
    $digest = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $updater = Join-Path $package "padmux-updater.exe"
    $valid = Start-Process -FilePath $updater -ArgumentList @("--verify-archive", ('"' + $archive + '"'), $digest) -Wait -PassThru
    if ($valid.ExitCode -ne 0) { throw "Rust updater rejected package: $($valid.ExitCode)" }
    $invalid = Start-Process -FilePath $updater -ArgumentList @("--verify-archive", ('"' + $archive + '"'), ('0' * 64)) -Wait -PassThru
    if ($invalid.ExitCode -eq 0) { throw "Rust updater accepted wrong digest" }
    $destination = Join-Path $output "padmux-rust-windows-x64.zip"
    Move-Item -Force $archive $destination
    "$digest  padmux-rust-windows-x64.zip" | Set-Content -Encoding ASCII "$destination.sha256"
    Write-Output "Verified Rust package: $destination"
} finally {
    # Only this invocation's unique staging directory is owned and removed.
    Remove-Item -Recurse -Force $stage
}
