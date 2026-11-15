# SPDX-License-Identifier: MIT
param([Parameter(Mandatory=$true)][string]$Binary, [switch]$Incumbent052)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$bytes = [IO.File]::ReadAllBytes($Binary)
if ($bytes.Length -lt 256 -or $bytes[0] -ne 77 -or $bytes[1] -ne 90) { throw 'Not a PE binary' }
$pe = [BitConverter]::ToInt32($bytes, 60)
if ($pe -lt 64 -or $pe + 26 -gt $bytes.Length -or
    [BitConverter]::ToUInt32($bytes, $pe) -ne 0x00004550 -or
    [BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664 -or
    [BitConverter]::ToUInt16($bytes, $pe + 24) -ne 0x20b) { throw 'Not x86-64 PE32+' }
$exports = (& dumpbin /exports $Binary | Out-String)
if ($LASTEXITCODE -ne 0) { throw 'Export inspection failed' }
$header = Get-Content -Raw (Join-Path $PSScriptRoot 'include/vckss_rust.h')
$symbols = @([regex]::Matches($header, '\b(vckss_rust_[A-Za-z0-9_]+)\s*\(') |
    ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique) + @('pginit', 'stata_call')
if ($symbols.Count -le 2) { throw 'No Rust exports parsed' }
if ($Incumbent052) {
    $incumbentHash = '2279846f19eaa065f6e8cc93002d058620773cc21e6b0c8e8b476b050dcf08c8'
    if ((Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $incumbentHash) {
        throw 'Comparison incumbent binary mismatch'
    }
    $additions = @('vckss_rust_engine_leaveout_residual_diagnostic_v1',
                   'vckss_rust_engine_repeat_measurement_diagnostic_v1',
                   'vckss_rust_engine_repeat_measurement_leading_v1')
    if ($symbols.Count -ne 149 -or @($additions | Where-Object { $_ -notin $symbols }).Count -ne 0) {
        throw 'Comparison incumbent export contract mismatch'
    }
    $symbols = @($symbols | Where-Object { $_ -notin $additions })
}

foreach ($symbol in $symbols) {
    if ($exports -notmatch ('(?m)\b' + [regex]::Escape($symbol) + '\b')) { throw "Missing export: $symbol" }
}
$imports = (& dumpbin /dependents $Binary | Out-String)
if ($LASTEXITCODE -ne 0) { throw 'Dependency inspection failed' }
$dependencies = @([regex]::Matches($imports, '(?im)^\s*([A-Za-z0-9_.-]+\.dll)\s*$') |
    ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique)
if ($dependencies.Count -eq 0) { throw 'No dependencies parsed' }
foreach ($dependency in $dependencies) {
    if ($dependency -notmatch '^(?i:KERNEL32|ADVAPI32|WS2_32|USERENV|BCRYPT|NTDLL|api-ms-win-core-[A-Za-z0-9_-]+)\.dll$') {
        throw "Unexpected dependency (static CRT required): $dependency"
    }
}
[pscustomobject]@{exports=$symbols; dependencies=$dependencies}
