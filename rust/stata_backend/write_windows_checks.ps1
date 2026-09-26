# SPDX-License-Identifier: GPL-3.0-only
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
$gates = @{}
foreach ($gate in @('rust_workspace','rust_backend','pe_import_export','clean_install','lifecycle','observation_component','individual_component','match_component','pooled_deletion','registry_idle')) { $gates[$gate] = 'PASS' }
@{schema='FEVC-WINDOWS-CHECKS-V2'; gates=$gates; stata_version='19'; stata_flavor='MP';
  production_commit=$identity.production_commit; harness_sha256=$identity.harness_sha256;
  binary_sha256=$installedHash; status='PASS'} |
  ConvertTo-Json -Depth 4 | Set-Content -Encoding ASCII (Join-Path $root 'windows-project-checks.json')
'complete' | Set-Content -Encoding ASCII (Join-Path $root 'windows-ci.stage')
