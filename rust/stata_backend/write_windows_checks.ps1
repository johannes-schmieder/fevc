# SPDX-License-Identifier: MIT
# Called only after the source-bound Windows driver's complete Stata assertions.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Get-Location).Path
$identity = Get-Content -LiteralPath (Join-Path $root 'windows-input-identity.json') -Raw | ConvertFrom-Json
if ($identity.production_commit -notmatch '^[a-f0-9]{40}$' -or $identity.harness_sha256 -notmatch '^[a-f0-9]{64}$') { throw 'Invalid harness identity' }
$build = Get-Content -LiteralPath (Join-Path $root 'windows-build.json') -Raw | ConvertFrom-Json
$installed = Join-Path $root 'windows-plus/f/fevc_rust_windows_x64.plugin'
$installedHash = (Get-FileHash $installed -Algorithm SHA256).Hash.ToLowerInvariant()
if ($installedHash -ne $build.sha256) { throw 'Installed plugin differs from the audited candidate' }
if (($identity.PSObject.Properties.Name -contains 'candidate_sha256') -and
    $installedHash -ne $identity.candidate_sha256) { throw 'Installed plugin differs from CI artifact' }
if ($identity.schema -ne 'FEVC-WINDOWS-INPUT-V3') { throw 'Grouped qualification requires input V3' }
$contract = Get-Content (Join-Path $PSScriptRoot 'windows-qualification-groups.json') -Raw | ConvertFrom-Json
$group = [string]$identity.qualification_group
if ($group -cnotin $contract.groups.PSObject.Properties.Name) { throw 'Unknown qualification group' }
$expected = @($contract.common) + @($contract.groups.$group)
$completed = @(Get-Content -LiteralPath (Join-Path $root 'windows-completed-tests.txt'))
if ($completed.Count -ne $expected.Count -or
    @($completed | Select-Object -Unique).Count -ne $completed.Count -or
    @(Compare-Object $completed $expected).Count -ne 0) { throw 'Incomplete or duplicate tests' }
$gates = @{}
foreach ($gate in $expected) { $gates[$gate] = 'PASS' }
if ($build.runtime_profile -notin @('smoke', 'full')) { throw 'Missing runtime profile' }
$scopes = @('HOSTED_CI_PASS','PRIVATE_BUILD_AND_TEST_PASS')
if ($group -eq 'smoke') { $scopes += 'PRIVATE_BUILD_ONLY_SMOKE' }
if ($build.rust_test_scope -notin $scopes) { throw 'Missing Rust test provenance' }
$gates['rust_workspace'] = $build.rust_test_scope
$gates['rust_backend'] = $build.rust_test_scope
$measurements = @()
if ($group -eq 'performance') {
    $path = Join-Path $root 'windows-performance.csv'
    $item = Get-Item -LiteralPath $path
    if ($item.Length -gt 8192 -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Invalid measurement file' }
    foreach ($line in Get-Content -LiteralPath $path) {
        $fields = $line.Split(',')
        if ($fields.Count -ne 14) { throw 'Invalid measurement width' }
        $row = @()
        foreach ($field in $fields) {
            $value = [double]::Parse($field, [Globalization.CultureInfo]::InvariantCulture)
            if ([double]::IsNaN($value) -or [double]::IsInfinity($value)) { throw 'Nonfinite measurement' }
            $row += $value
        }
        if ($row[0] -ne [Math]::Truncate($row[0]) -or $row[1] -ne [Math]::Truncate($row[1])) { throw 'Noninteger measurement cell' }
        $row[0] = [int]$row[0]; $row[1] = [int]$row[1]
        $measurements += ,$row
    }
    if ($measurements.Count -ne 12) { throw 'Incomplete measurements' }
}
$receipt = @{schema='FEVC-WINDOWS-CHECKS-V4'; gates=$gates; stata_version='19'; stata_flavor='MP';
  runtime_profile=$build.runtime_profile; production_commit=$identity.production_commit;
  harness_sha256=$identity.harness_sha256; binary_sha256=$installedHash; status='PASS';
  qualification_group=$group; campaign_sha256=$identity.campaign_sha256;
  performance_arm=$identity.performance_arm; performance_block=$identity.performance_block;
  completed_tests=$expected; measurements=$measurements}
$json = $receipt | ConvertTo-Json -Depth 8 -Compress
if ([Text.Encoding]::ASCII.GetByteCount($json) -gt 8190) { throw 'Project receipt exceeds collector bound' }
$json | Set-Content -Encoding ASCII (Join-Path $root 'windows-project-checks.json')
'complete' | Set-Content -Encoding ASCII (Join-Path $root 'windows-ci.stage')
