"""Explicit CLT qualification must not hide an ordinary Xcode/license failure."""
from pathlib import Path
import os
import subprocess

SCRIPT = Path(__file__).resolve().parents[3] / 'rust/stata_backend/qualify_macos.sh'


def run_identity(tmp_path, *, developer='', compiler=None, xcode_status=0):
    compiler = compiler or '/Library/Developer/CommandLineTools/usr/bin/clang'
    xcrun = tmp_path / 'xcrun'
    xcrun.write_text('#!/bin/bash\ncase "$1" in\n'
                      f'--find) echo "{compiler}";;\n'
                      '--show-sdk-path) echo /test/sdk;;\n'
                      '--show-sdk-version) echo 26.2;;\nesac\n')
    xcode = tmp_path / 'xcodebuild'
    xcode.write_text(f'#!/bin/bash\necho called > "{tmp_path}/xcode-called"\n'
                     f'echo Xcode-test\nexit {xcode_status}\n')
    xcrun.chmod(0o755)
    xcode.chmod(0o755)
    function = SCRIPT.read_text().split('apple_toolchain_identity() {', 1)[1].split('\n}\n', 1)[0]
    shell = 'set -euo pipefail\nfail() { echo "$*" >&2; exit 1; }\napple_toolchain_identity() {' + function + '\n}\n'
    shell += 'apple_toolchain_identity\nprintf "%s\\n" "$apple_toolchain_kind" "$xcode_version" "$apple_sdk_version"\n'
    environment = dict(os.environ, PATH=f'{tmp_path}:/usr/bin:/bin', DEVELOPER_DIR=developer)
    return subprocess.run(['/bin/bash', '-c', shell], text=True, capture_output=True, env=environment)


def test_explicit_clt_does_not_invoke_xcode(tmp_path):
    result = run_identity(tmp_path, developer='/Library/Developer/CommandLineTools', xcode_status=69)
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == ['COMMAND_LINE_TOOLS', 'NOT_USED_COMMAND_LINE_TOOLS', '26.2']
    assert not (tmp_path / 'xcode-called').exists()


def test_default_xcode_failure_stays_a_failure(tmp_path):
    result = run_identity(tmp_path, xcode_status=69)
    assert result.returncode != 0
    assert (tmp_path / 'xcode-called').exists()
    assert 'COMMAND_LINE_TOOLS' not in result.stdout


def test_clt_must_resolve_its_own_compiler(tmp_path):
    result = run_identity(tmp_path, developer='/Library/Developer/CommandLineTools', compiler='/test/other/clang')
    assert result.returncode != 0
    assert 'different compiler' in result.stderr


def test_ordinary_xcode_identity_is_preserved(tmp_path):
    result = run_identity(tmp_path)
    assert result.returncode == 0
    assert result.stdout.splitlines() == ['XCODE', 'Xcode-test', '26.2']
