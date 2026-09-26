# SPDX-License-Identifier: GPL-3.0-only
# Build-only test distribution: licensed Stata validation is performed by the owner.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Get-Location).Path
$commit = (& git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $commit -notmatch '^[0-9a-f]{40}$') { throw 'Missing source identity' }
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $vs) { throw 'MSVC unavailable' }
$devcmd = Join-Path $vs 'Common7/Tools/VsDevCmd.bat'
$environment = & cmd.exe /d /s /c "`"$devcmd`" -arch=x64 -host_arch=x64 >nul && set"
if ($LASTEXITCODE -ne 0) { throw 'MSVC environment setup failed' }
foreach ($line in $environment) {
    if ($line -match '^([A-Za-z_][A-Za-z0-9_]*)=(.*)$') {
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process')
    }
}
$env:RUSTFLAGS = '-C target-feature=+crt-static'
& cargo +1.85.1 test --manifest-path rust/stata_backend/Cargo.toml --locked --lib
if ($LASTEXITCODE -ne 0) { throw 'Standalone Rust tests failed' }
& cargo +1.85.1 build --manifest-path rust/stata_backend/Cargo.toml --locked --release
if ($LASTEXITCODE -ne 0) { throw 'Windows compilation failed' }
$library = Join-Path $root 'rust/stata_backend/target/release/vckss_stata.dll'
$audit = & ./rust/stata_backend/audit_windows_plugin.ps1 -Binary $library
$symbols = $audit.exports
$dependencies = $audit.dependencies
$output = Join-Path $root 'windows-test-output'
if (Test-Path $output) { throw 'Output directory already exists' }
$package = Join-Path $output 'fevc-windows-test'
New-Item -ItemType Directory -Path $package | Out-Null
$manifest = @(Get-Content 'fevc/fevc.pkg')
foreach ($line in $manifest) {
    if ($line.StartsWith('f ')) {
        $name = $line.Substring(2).Trim()
        if ($name -notmatch '^[A-Za-z0-9_.-]+$') { throw 'Unsafe package path' }
        Copy-Item (Join-Path $root "fevc/$name") (Join-Path $package $name)
    }
}
$plugin = 'fevc_rust_windows_x64.plugin'
Copy-Item $library (Join-Path $package $plugin)
Copy-Item 'fevc/stata.toc' $package
$manifest = @($manifest | ForEach-Object {
    if ($_ -eq 'f LICENSE' -or $_ -eq 'f THIRD_PARTY_NOTICES.txt') { 'F ' + $_.Substring(2) }
    elseif ($_ -like 'd GPL-3.0-only prerelease source package;*') { 'd Windows test candidate; licensed Stata testing pending.' }
    else { $_ }
}) + @("f $plugin")
$manifest | Set-Content -Encoding utf8 (Join-Path $package 'fevc.pkg')
@'
version 18.0
set more off
capture log close fevc_windows_test
log using fevc-windows-test.log, text replace name(fevc_windows_test)
which fevc
fevc_rust probe
assert r(progress_api)==2
foreach example in exact_controls jla_controls {
    fevc_run `example' using fevc.sthlp
    assert "`e(backend_selected)'"=="rust"
    assert "`e(status)'"=="KSS_POINT_ESTIMATES_ONLY"
}
display "FEVC WINDOWS SMOKE PASS"
log close fevc_windows_test
'@ | Set-Content -Encoding ascii (Join-Path $package 'test-windows.do')
@"
FEVC Windows x86-64 test candidate

This binary passed compilation, Rust unit tests, and PE export/dependency checks.
It has NOT yet been tested in licensed Windows Stata. No AWS machine was used.

Extract this folder, start a fresh Stata 18+ session, and install using its path:
  net install fevc, replace from("C:/path/to/fevc-windows-test")
Then run:
  do "C:/path/to/fevc-windows-test/test-windows.do"
The log should end with FEVC WINDOWS SMOKE PASS. The help examples preserve data.
The included test is a smoke check, not full scientific/platform qualification.

Rust 1.85.1; x86_64-pc-windows-msvc; static CRT; GPL-3.0-only.
Corresponding source and locked dependencies:
  https://github.com/johannes-schmieder/fevc/tree/$commit
"@ | Set-Content -Encoding ascii (Join-Path $package 'README-WINDOWS.txt')
$inventory = @(Get-ChildItem $package -File | Sort-Object Name | ForEach-Object {
    @{name=$_.Name; sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
})
@{schema='FEVC-WINDOWS-TEST-PACKAGE-V1'; status='BUILD_ONLY_NOT_STATA_TESTED';
  source_commit=$commit; target='x86_64-pc-windows-msvc'; toolchain='1.85.1'; crt='static';
  rust_unit_tests='PASS'; pe_audit='PASS'; exports=$symbols; dependencies=$dependencies; files=$inventory} |
  ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 (Join-Path $package 'build-receipt.json')
Compress-Archive -Path $package -DestinationPath (Join-Path $output 'fevc-windows-test.zip')
# Upload only the complete archive and its checksum, not duplicate loose files.
$zip = Join-Path $output 'fevc-windows-test.zip'
((Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant() + '  fevc-windows-test.zip') |
  Set-Content -Encoding ascii (Join-Path $output 'SHA256SUMS')
Remove-Item -Recurse -Force $package
Write-Output 'FEVC_WINDOWS_TEST_PACKAGE_BUILD_PASS'
