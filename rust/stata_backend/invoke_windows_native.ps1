# SPDX-License-Identifier: GPL-3.0-only
# Windows PowerShell 5.1 treats redirected native stderr as PowerShell errors.
# Process-level redirection preserves informational stderr and the real exit code.
function Invoke-FevcWindowsNative {
    param(
        [Parameter(Mandatory=$true)][string]$FilePath,
        [Parameter(Mandatory=$true)][string[]]$ArgumentList,
        [Parameter(Mandatory=$true)][string]$LogPrefix
    )
    $executable = (Get-Command -Name $FilePath -CommandType Application -ErrorAction Stop).Source
    $process = Start-Process -FilePath $executable -ArgumentList $ArgumentList `
        -WorkingDirectory (Get-Location).Path -Wait -PassThru -NoNewWindow `
        -RedirectStandardOutput ($LogPrefix + '.stdout.log') `
        -RedirectStandardError ($LogPrefix + '.stderr.log') -ErrorAction Stop
    try {
        $nativeExit = $process.ExitCode
        if ($null -eq $nativeExit) { throw 'FEVC_NATIVE_EXIT_UNAVAILABLE' }
        if ($nativeExit -ne 0) { throw ('FEVC_NATIVE_EXIT={0}' -f $nativeExit) }
    } finally {
        $process.Dispose()
    }
}
