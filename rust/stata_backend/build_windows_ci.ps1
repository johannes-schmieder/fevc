# SPDX-License-Identifier: GPL-3.0-only
# Invoked only by the owner-controlled guarded windows-ci.do profile.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = (Get-Location).Path
$statusPath = Join-Path $projectRoot 'windows-build.status'
if (Test-Path $statusPath) { throw 'Build status already exists' }
$identityPath = Join-Path $projectRoot 'windows-input-identity.json'
$identity = Get-Content -LiteralPath $identityPath -Raw | ConvertFrom-Json
$providedCandidate = $identity.PSObject.Properties.Name -contains 'candidate_sha256'
$rustTestScope = 'PRIVATE_BUILD_AND_TEST_PASS'
if ($providedCandidate) {
    if ($identity.candidate_sha256 -notmatch '^[a-f0-9]{64}$' -or
        $identity.hosted_build_run_id -notmatch '^[0-9]+$' -or
        $identity.hosted_rust_run_id -notmatch '^[0-9]+$') { throw 'Invalid hosted candidate identity' }
    # Hosted compilation, source tests and their identities are verified before
    # transfer. Keep the licensed process bounded to auditing and runtime tests.
    $rustTestScope = 'HOSTED_CI_PASS'
}
$backend = Join-Path $projectRoot 'rust/stata_backend'
$spi = Join-Path $backend 'stata-spi'
if (-not $providedCandidate) {
    'build_spi' | Set-Content -Encoding ASCII (Join-Path $projectRoot 'windows-ci.stage')
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
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $vs) { throw 'MSVC installation unavailable' }
$devcmd = Join-Path $vs 'Common7/Tools/VsDevCmd.bat'
$env:RUSTFLAGS = '-C target-feature=+crt-static'
$env:CARGO_BUILD_JOBS = '2'
$env:VCKSS_STATA_SPI_DIR = $spi
$buildLog = Join-Path $projectRoot 'windows-build.log'
if ($providedCandidate) {
    $candidate = Join-Path $projectRoot 'windows-candidate/fevc_rust_windows_x64.plugin'
    if ((Get-FileHash $candidate -Algorithm SHA256).Hash.ToLowerInvariant() -ne $identity.candidate_sha256) {
        throw 'Transferred Windows candidate hash mismatch'
    }
} else {
    # Install only the project's pinned public compiler, never licensed tools.
    'build_toolchain' | Set-Content -Encoding ASCII (Join-Path $projectRoot 'windows-ci.stage')
    & rustup toolchain install 1.85.1 --profile minimal --no-self-update *> $buildLog
    if ($LASTEXITCODE -ne 0) { throw 'Pinned Rust toolchain unavailable' }
    $command = '""{0}" -arch=x64 -host_arch=x64 && cargo +1.85.1 build --release --locked --manifest-path rust/stata_backend/Cargo.toml"' -f $devcmd
    'build_native' | Set-Content -Encoding ASCII (Join-Path $projectRoot 'windows-ci.stage')
    & cmd.exe /d /s /c $command *>> $buildLog
    if ($LASTEXITCODE -ne 0) { throw 'Native build failed; preserve bounded compiler diagnostics' }
    $candidate = Join-Path $backend 'target/release/vckss_stata.dll'
}
'pe_import_export' | Set-Content -Encoding ASCII (Join-Path $projectRoot 'windows-ci.stage')
$environment = & cmd.exe /d /s /c "`"$devcmd`" -arch=x64 -host_arch=x64 >nul && set"
if ($LASTEXITCODE -ne 0) { throw 'MSVC audit environment unavailable' }
foreach ($line in $environment) {
    if ($line -match '^([A-Za-z_][A-Za-z0-9_]*)=(.*)$') {
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process')
    }
}
$audit = & ./rust/stata_backend/audit_windows_plugin.ps1 -Binary $candidate
if (-not $providedCandidate) {
    foreach ($test in @(@('rust_workspace','rust/Cargo.toml'), @('rust_backend','rust/stata_backend/Cargo.toml'))) {
        $test[0] | Set-Content -Encoding ASCII (Join-Path $projectRoot 'windows-ci.stage')
        $testCommand = '""{0}" -arch=x64 -host_arch=x64 && cargo +1.85.1 test --workspace --all-targets --locked --manifest-path {1}"' -f $devcmd, $test[1]
        & cmd.exe /d /s /c $testCommand *>> $buildLog
        if ($LASTEXITCODE -ne 0) { throw 'Locked Rust test gate failed' }
    }
}
'build_package' | Set-Content -Encoding ASCII (Join-Path $projectRoot 'windows-ci.stage')
$plugin = 'fevc_rust_windows_x64.plugin'
Copy-Item $candidate (Join-Path $projectRoot $plugin)
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
  toolchain='1.85.1'; exports=$audit.exports; dependencies=$audit.dependencies;
  rust_test_scope=$rustTestScope;
  status='BUILD_ONLY_NOT_QUALIFICATION'} |
  ConvertTo-Json | Set-Content -Encoding ASCII (Join-Path $projectRoot 'windows-build.json')
'FEVC_WINDOWS_BUILD=PASS' | Set-Content -Encoding ASCII $statusPath
