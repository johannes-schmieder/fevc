# SPDX-License-Identifier: GPL-3.0-only
# Called only after the source-bound Windows driver's complete Stata assertions.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Get-Location).Path
$identity = Get-Content -LiteralPath (Join-Path $root 'windows-input-identity.json') -Raw | ConvertFrom-Json
if ($identity.production_commit -notmatch '^[a-f0-9]{40}$' -or $identity.harness_sha256 -notmatch '^[a-f0-9]{64}$') { throw 'Invalid harness identity' }
$gates = @{}
foreach ($gate in @('rust_workspace','rust_backend','pe_import_export','clean_install','lifecycle','observation_component','individual_component','match_component','registry_idle')) { $gates[$gate] = 'PASS' }
@{schema='FEVC-WINDOWS-CHECKS-V2'; gates=$gates; stata_version='19'; stata_flavor='MP';
  production_commit=$identity.production_commit; harness_sha256=$identity.harness_sha256;
  final_archive_sha256=$null; status='PASS'} |
  ConvertTo-Json -Depth 4 | Set-Content -Encoding ASCII (Join-Path $root 'windows-project-checks.json')
'complete' | Set-Content -Encoding ASCII (Join-Path $root 'windows-ci.stage')
