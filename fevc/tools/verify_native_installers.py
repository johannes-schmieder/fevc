"""Verify exact native payload bytes through real Stata installation commands."""
import argparse
import functools
import hashlib
import http.server
import json
import os
import platform
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import threading


REPOSITORY_PLUGINS = {
    'fevc_rust_macos_arm64.plugin', 'fevc_rust_macos_x86_64.plugin',
    'fevc_rust_macos.plugin', 'fevc_rust_linux_x64.plugin',
    'fevc_rust_windows_x64.plugin',
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def sanitize(text):
    lines = text.splitlines()
    first = next((i for i, line in enumerate(lines) if line.startswith('. ')), len(lines))
    return '\n'.join(line for line in lines[first:]
                     if 'Licensed to:' not in line and 'Serial number:' not in line) + '\n'


def installed_file(plus, name):
    # Stata normalizes installed names (including LICENSE) to lowercase.
    # Match that behavior on case-sensitive filesystems, rejecting duplicates.
    found = [p for p in plus.rglob('*')
             if p.is_file() and p.name.casefold() == name.casefold()]
    if len(found) != 1:
        raise ValueError(f'expected one installed {name}, found {len(found)}')
    return found[0]


def verify(catalog, test_root, output, stata, url, methods):
    entries = [line[2:] for line in (catalog / 'fevc.pkg').read_text().splitlines()
               if line.startswith(('f ', 'F '))]
    assert len(entries) == len(set(entries))
    expected = {}
    for entry in entries:
        assert entry.startswith('fevc/') and '..' not in Path(entry).parts
        expected[Path(entry).name] = (catalog / entry).read_bytes()
    assert len(expected) == len(entries)
    assert {n for n in expected if n.endswith('.plugin')} == REPOSITORY_PLUGINS
    output.mkdir(parents=True, exist_ok=False)
    results = []
    with tempfile.TemporaryDirectory(prefix='fevc-installer-') as tmp:
        temporary = Path(tmp).resolve()
        for method in methods:
            plus = temporary / method / 'plus'
            personal = temporary / method / 'personal'
            site = temporary / method / 'site'
            for p in (plus, personal, site):
                p.mkdir(parents=True)
            for mode in ('fresh', 'replace'):
                label = method + '-' + mode
                cwd = temporary / label
                cwd.mkdir()
                if mode == 'replace':
                    # Verify the command actually replaces a changed installed file.
                    helpfile = next(plus.rglob('fevc.sthlp'))
                    helpfile.write_bytes(helpfile.read_bytes() + b'\n* installer replacement sentinel\n')
                installer = (f'net install fevc, replace from("{url}")' if method == 'net'
                             else 'github install johannes-schmieder/fevc')
                bootstrap = ('net install github, replace from("https://haghish.github.io/github/")'
                             if method == 'github' and mode == 'fresh' else '')
                driver = output / (label + '.do')
                driver.write_text(f'''version 18.0
clear all
set more off
set varabbrev off
set processors {min(4, int(os.environ.get('NSLOTS', '4')))}
sysdir set PLUS "{plus}/"
sysdir set PERSONAL "{personal}/"
sysdir set SITE "{site}/"
{bootstrap}
{installer}
findfile fevc.ado
assert strpos(`"`r(fn)'"', "{plus}") == 1
findfile fevc.sthlp
assert strpos(`"`r(fn)'"', "{plus}") == 1
fevc_rust probe
assert r(progress_api) == 2
clear
set obs 3
generate long sentinel = _n
quietly _datasignature
local before `"`r(datasignature)'"'
fevc_run exact_controls using fevc.sthlp
assert "`e(backend_selected)'" == "rust"
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
quietly _datasignature
assert `"`r(datasignature)'"' == `"`before'"'
estat decomposition, full
help fevc
do "{test_root}/fevc/tests/stata/test_rust_match_component_inference.do" "{plus}/f"
do "{test_root}/fevc/tests/stata/test_pooled_deletion.do" "{plus}/f"
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
display "FEVC_INSTALL_RUNTIME Stata=`c(stata_version)' machine=`c(machine_type)'"
display "FEVC PUBLIC INSTALL PASS"
exit 0
''')
                completed = subprocess.run([stata, '-q', 'do', str(driver)], cwd=cwd,
                    stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                    text=True, timeout=1800)
                transcript = sanitize(completed.stdout)
                (output / (label + '.sanitized.log')).write_text(transcript)
                if (completed.returncode or 'FEVC PUBLIC INSTALL PASS' not in completed.stdout
                        or 'PASS test_rust_match_component_inference.do' not in completed.stdout
                        or 'PASS test_pooled_deletion.do' not in completed.stdout):
                    raise RuntimeError('Stata installation checks failed: ' + label)
                assert not list(plus.rglob('_fevc*.ado')), 'obsolete package helpers installed'
                inventory = []
                for name, payload in expected.items():
                    found = installed_file(plus, name)
                    assert found.read_bytes() == payload, (label, name, 'hash mismatch')
                    inventory.append({'name': name, 'installed_name': found.name,
                                      'sha256': sha(payload)})
                results.append({'method': method, 'mode': mode, 'status': 'PASS',
                    'installed_files': inventory, 'stata_process_rc': completed.returncode,
                    'transcript_sha256': sha(transcript.encode()),
                    'checks': ['native progress API 2', 'README example', 'caller data restoration',
                               'decomposition', 'help', 'installed match q0/q1',
                               'installed deletion-unit mover regression', 'idle native registry',
                               'all installed file hashes']})
                (output / 'verification.json').write_text(json.dumps({
                    'status': 'IN_PROGRESS', 'results': results}, indent=2) + '\n')
                print('INSTALL_PASS', label, len(inventory), 'files', flush=True)
    receipt = {'status': 'PASS', 'platform': platform.system(), 'architecture': platform.machine(),
               'catalog_url': url, 'methods': methods, 'results': results,
               'catalog_sha256': sha((catalog / 'fevc.pkg').read_bytes()),
               'verifier_sha256': sha(Path(__file__).read_bytes()),
               'scope': ('Installed payload hashes cover all five plugins; runtime checks '
                         'apply only to the recorded execution platform. Windows Stata '
                         'testing is performed separately by the owner.')}
    (output / 'verification.json').write_text(json.dumps(receipt, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--catalog', required=True, type=Path)
    parser.add_argument('--test-root', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--stata')
    parser.add_argument('--public', action='store_true')
    args = parser.parse_args()
    stata = args.stata or shutil.which('stata-mp')
    if not stata and platform.system() == 'Darwin':
        stata = '/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp'
    assert stata and Path(stata).is_file(), 'Stata executable required'
    server = None
    worker = None
    if args.public:
        url = 'https://raw.githubusercontent.com/johannes-schmieder/fevc/main/'
        methods = ['net', 'github']
    else:
        class Handler(http.server.SimpleHTTPRequestHandler):
            def log_message(self, *args):
                pass
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0),
            functools.partial(Handler, directory=str(args.catalog.resolve())))
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        url = f'http://127.0.0.1:{server.server_address[1]}/'
        methods = ['net']
    try:
        verify(args.catalog.resolve(), args.test_root.resolve(), args.output.resolve(),
               stata, url, methods)
    finally:
        if server:
            server.shutdown()
            server.server_close()
            worker.join()


if __name__ == '__main__':
    main()
