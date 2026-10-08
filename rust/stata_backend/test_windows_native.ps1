# SPDX-License-Identifier: GPL-3.0-only
param([string]$OutputDirectory = (Join-Path (Get-Location).Path 'windows-native-launcher-test'))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'invoke_windows_native.ps1')
New-Item -ItemType Directory -Path $OutputDirectory -ErrorAction Stop | Out-Null
foreach ($expectedExit in @(0, 17)) {
    $prefix = Join-Path $OutputDirectory ('exit-{0}' -f $expectedExit)
    # /s removes exactly the outer quotes after /c; cmd owns stream redirection.
    $command = '"echo FEVC_NATIVE_STDOUT&echo FEVC_NATIVE_STDERR 1>&2&exit /b {0}"' -f $expectedExit
    $failure = ''
    try {
        Invoke-FevcWindowsNative -FilePath $env:ComSpec `
            -ArgumentList @('/d', '/s', '/c', $command) -LogPrefix $prefix
    } catch {
        $failure = $_.Exception.Message
    }
    if ($expectedExit -eq 0 -and $failure -ne '') { throw 'Native stderr success regression failed' }
    if ($expectedExit -ne 0 -and $failure -cne ('FEVC_NATIVE_EXIT={0}' -f $expectedExit)) {
        throw 'Native nonzero exit regression failed'
    }
    if ((Get-Content -LiteralPath ($prefix + '.stdout.log') -Raw).Trim() -cne 'FEVC_NATIVE_STDOUT' -or
        (Get-Content -LiteralPath ($prefix + '.stderr.log') -Raw).Trim() -cne 'FEVC_NATIVE_STDERR') {
        throw 'Native output preservation regression failed'
    }
}
'FEVC_WINDOWS_NATIVE_IO=PASS'
