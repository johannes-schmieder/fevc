# SPDX-License-Identifier: GPL-3.0-only
param([Parameter(Mandatory=$true)][ValidateRange(1, 99999)][int]$ReturnCode)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Get-Location).Path
$stage = 'unknown'
$stagePath = Join-Path $root 'windows-ci.stage'
$allowed = @('startup','build','build_spi','build_toolchain','build_native',
    'pe_import_export','rust_workspace','rust_backend','build_package',
    'clean_install','lifecycle','point_mean','projection_mean','observation_component',
    'individual_component','match_component','pooled_deletion','mcse_modes',
    'mcse_attachments','centering_mean','centering_exact','centering_jla',
    'centering_options','registry_idle','project_receipt','complete')
if (Test-Path -LiteralPath $stagePath) {
    $item = Get-Item -LiteralPath $stagePath
    if ($item.Length -le 128 -and -not ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        $candidate = (Get-Content -LiteralPath $stagePath -Raw).Trim()
        if ($candidate -cin $allowed) { $stage = $candidate }
    }
}
# No arbitrary exception, path, environment or Stata output is copied here.
@{schema='FEVC-WINDOWS-FAILURE-V1'; stage=$stage; stata_rc=$ReturnCode; status='FAIL'} |
    ConvertTo-Json | Set-Content -Encoding ASCII (Join-Path $root 'windows-project-failure.json')
# Let the fixed controller stop waiting immediately, while preserving failure.
'WINDOWS_CI=FAIL' | Set-Content -Encoding ASCII (Join-Path $root 'windows-ci.status')
