# SPDX-License-Identifier: GPL-3.0-only
# Invoked only by the owner-controlled guarded windows-ci.do profile.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = (Get-Location).Path
$statusPath = Join-Path $projectRoot 'windows-build.status'
if (Test-Path $statusPath) { throw 'Build status already exists' }
$backend = Join-Path $projectRoot 'rust/stata_backend'
$spi = Join-Path $backend 'stata-spi'
New-Item -ItemType Directory -Path $spi -ErrorAction Stop | Out-Null
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
foreach ($line in Get-Content (Join-Path $backend 'stata-spi.sha256')) {
    if ($line -notmatch '^([0-9a-f]{64})\s+(stplugin\.[ch])$') { throw 'Invalid SPI manifest' }
    $expected = $Matches[1]
    $name = $Matches[2]
    $destination = Join-Path $spi $name
    Invoke-WebRequest -UseBasicParsing -Uri "https://www.stata.com/plugins/$name" -OutFile $destination
    if ((Get-FileHash $destination -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
        throw 'SPI hash mismatch'
    }
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $vs) { throw 'MSVC installation unavailable' }
$devcmd = Join-Path $vs 'Common7/Tools/VsDevCmd.bat'
$env:RUSTFLAGS = '-C target-feature=+crt-static'
$env:CARGO_BUILD_JOBS = '2'
$env:VCKSS_STATA_SPI_DIR = $spi
$buildLog = Join-Path $projectRoot 'windows-build.log'
# Install only the project's pinned public compiler; never install licensed tools.
& rustup toolchain install 1.85.1 --profile minimal --no-self-update *> $buildLog
if ($LASTEXITCODE -ne 0) { throw 'Pinned Rust toolchain unavailable' }
$command = '""{0}" -arch=x64 -host_arch=x64 && cargo +1.85.1 build --release --locked --manifest-path rust/stata_backend/Cargo.toml"' -f $devcmd
& cmd.exe /d /s /c $command *>> $buildLog
if ($LASTEXITCODE -ne 0) { throw 'Native build failed; preserve bounded compiler diagnostics' }
$candidate = Join-Path $backend 'target/release/vckss_stata.dll'
$bytes = [IO.File]::ReadAllBytes($candidate)
if ($bytes.Length -lt 256 -or $bytes[0] -ne 77 -or $bytes[1] -ne 90) { throw 'Not a PE candidate' }
$pe = [BitConverter]::ToInt32($bytes, 60)
if ($pe -lt 64 -or $pe + 26 -gt $bytes.Length -or
    [BitConverter]::ToUInt32($bytes, $pe) -ne 0x00004550 -or
    [BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664 -or
    [BitConverter]::ToUInt16($bytes, $pe + 24) -ne 0x20b) { throw 'Not an x86-64 PE32+ candidate' }
$plugin = 'fevc_rust_windows_x64.plugin'
Copy-Item $candidate (Join-Path $projectRoot "fevc/$plugin")
$stage = Join-Path $projectRoot 'windows-package'
New-Item -ItemType Directory -Path $stage -ErrorAction Stop | Out-Null
$manifest = Get-Content (Join-Path $projectRoot 'fevc/fevc.pkg')
foreach ($line in $manifest) {
    if ($line.StartsWith('f ')) {
        $name = $line.Substring(2).Trim()
        if ($name -notmatch '^[A-Za-z0-9_.-]+$') { throw 'Unsafe package path' }
        Copy-Item (Join-Path $projectRoot "fevc/$name") (Join-Path $stage $name)
    }
}
Copy-Item (Join-Path $projectRoot 'fevc/stata.toc') $stage
Copy-Item $candidate (Join-Path $stage $plugin)
$manifest + "f $plugin" | Set-Content -Encoding ASCII (Join-Path $stage 'fevc.pkg')
$digest = (Get-FileHash $candidate -Algorithm SHA256).Hash.ToLowerInvariant()
@{schema='FEVC-WINDOWS-BUILD-V1'; target='x86_64-pc-windows-msvc';
  binary=$plugin; sha256=$digest; crt='static'; pe_audit='PASS';
  toolchain='1.85.1'; status='BUILD_ONLY_NOT_QUALIFICATION'} |
  ConvertTo-Json | Set-Content -Encoding ASCII (Join-Path $projectRoot 'windows-build.json')
'FEVC_WINDOWS_BUILD=PASS' | Set-Content -Encoding ASCII $statusPath
