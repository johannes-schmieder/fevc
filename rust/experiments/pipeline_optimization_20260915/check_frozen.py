#!/usr/bin/env python3
"""Record source-bound checks and diagnostic builds without changing sources."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

TOOLCHAIN = Path('/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('snapshot', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    receipt = args.snapshot / 'snapshot.json'
    source = args.snapshot / 'source'
    hashes = json.loads(receipt.read_text())['source']

    def verify():
        for name, digest in hashes.items():
            assert sha(source / name) == digest, f'Source changed: {name}'

    verify()
    args.output.mkdir(parents=True, exist_ok=False)
    cargo = str(TOOLCHAIN / 'cargo')
    environment = dict(os.environ, PATH=f'{TOOLCHAIN}:/opt/homebrew/bin:/usr/bin:/bin')
    stages = [
        ('format', [cargo, 'fmt', '--all', '--', '--check']),
        ('clippy', [cargo, 'clippy', '--locked', '--workspace', '--all-targets', '--', '-D', 'warnings']),
        ('clippy-profile', [cargo, 'clippy', '--locked', '--workspace', '--all-targets',
                            '--features', 'vckss-core/pipeline-profile', '--', '-D', 'warnings']),
        ('workspace', [cargo, 'test', '--locked', '--workspace', '--all-targets', '--', '--test-threads=2']),
        ('profile-build', [cargo, 'build', '--locked', '--release', '-p', 'vckss-core',
                           '--features', 'pipeline-profile', '--example', 'pipeline_profile']),
    ]
    records = []
    for name, command in stages:
        verify()
        started = time.monotonic()
        with (args.output / f'{name}.log').open('x') as stream:
            result = subprocess.run(command, cwd=source / 'rust', env=environment,
                                    stdout=stream, stderr=subprocess.STDOUT, timeout=3600)
        verify()
        records.append(dict(stage=name, command=command, returncode=result.returncode,
                            seconds=time.monotonic()-started, log_sha256=sha(args.output / f'{name}.log')))
        (args.output / 'checks.json').write_text(json.dumps(dict(
            snapshot_sha256=sha(receipt), stages=records), indent=2)+'\n')
        print(json.dumps(records[-1]), flush=True)
        if result.returncode:
            raise RuntimeError(f'Stopped on {name}: see preserved log')


if __name__ == '__main__':
    main()
