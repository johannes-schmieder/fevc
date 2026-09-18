#!/usr/bin/env python3
"""Twelve bounded core-only attachment diagnostics from a frozen source."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('output', type=Path)
    p.add_argument('--input', required=True, type=Path)
    p.add_argument('--source-receipt', required=True, type=Path)
    args = p.parse_args()
    root = Path(__file__).resolve().parents[3]
    receipt = json.loads(args.source_receipt.read_text())
    assert args.input.with_suffix('.json').is_file()
    metadata = json.loads(args.input.with_suffix('.json').read_text())
    assert metadata['rows'] == 8000 and metadata['sha256'] == sha(args.input)
    for name, digest in receipt['source'].items():
        assert sha(root / name) == digest, name
    args.output.mkdir(parents=True, exist_ok=False)
    modes = ['observation-projection', 'match-projection',
             'observation-component', 'match-component']
    calls = [dict(mode=m, threads=t, status='unattempted')
             for t in (1, 4, 7) for m in modes]
    record = dict(kind='core-only diagnostic; no performance acceptance',
                  source_sha256=sha(args.source_receipt), input_sha256=sha(args.input),
                  calls=calls)
    target = args.output / 'results.json'

    def save():
        target.write_text(json.dumps(record, indent=2, sort_keys=True) + '\n')

    save()
    env = dict(os.environ, PATH='/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin:/opt/homebrew/bin:/usr/bin:/bin')
    with (args.output / 'build.log').open('w') as log:
        build = subprocess.run(['cargo', 'build', '--release', '-p', 'vckss-core',
            '--features', 'pipeline-profile', '--example', 'pipeline_paths'],
            cwd=root/'rust', env=env, stdout=log, stderr=subprocess.STDOUT, timeout=300)
    record['build_returncode'] = build.returncode
    save()
    if build.returncode:
        return build.returncode
    binary = args.output / 'pipeline_paths'
    shutil.copy2(root / 'rust/target/release/examples/pipeline_paths', binary)
    record['binary_sha256'] = sha(binary)
    for call in calls:
        log_path = args.output / f"{call['mode']}-t{call['threads']}.log"
        call['status'] = 'started'
        save()
        try:
            with log_path.open('w') as log:
                result = subprocess.run([str(binary), str(args.input), call['mode'], str(call['threads'])],
                    stdout=log, stderr=subprocess.STDOUT, timeout=300)
            call['returncode'] = result.returncode
        except subprocess.TimeoutExpired:
            call['returncode'] = 124
        text = log_path.read_text()
        phases = {}
        for line in text.splitlines():
            if line.startswith('FEVC_PIPELINE_PROFILE_V1\t'):
                _, phase, n, inclusive, exclusive = line.split('\t')
                phases[phase] = dict(calls=int(n), inclusive_seconds=int(inclusive)/1e9,
                                     exclusive_seconds=int(exclusive)/1e9)
        call['phases'] = phases
        call['receipt'] = [s for s in text.splitlines() if s.startswith('DIAGNOSTIC_')]
        call['log_sha256'] = sha(log_path)
        call['status'] = 'pass' if call['returncode'] == 0 and 'DIAGNOSTIC_PATH_PASS ' in text else 'failed'
        save()
        print(json.dumps(call), flush=True)
        if call['status'] != 'pass':
            return 1
    for name, digest in receipt['source'].items():
        assert sha(root / name) == digest, name
    record['source_verified_after_calls'] = True
    save()
    return 0


if __name__ == '__main__':
    sys.exit(main())
