#!/usr/bin/env python3
"""Bounded core-only diagnostics; these timings are not command acceptance."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

REPO = Path(__file__).resolve().parents[3]
GRAPHS = ('degree4_bottleneck', 'degree5_well_mixed', 'degree5_segmented',
          'mixed_well_mixed', 'mixed_segmented')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    parser.add_argument('--inputs', type=Path, required=True)
    parser.add_argument('--threads', type=int, nargs='+', default=[1, 4, 7])
    parser.add_argument('--modes', nargs='+', default=['observation', 'match'])
    parser.add_argument('--source-receipt', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    executable = args.output / 'pipeline_profile'
    shutil.copy2(REPO / 'rust/target/release/examples/pipeline_profile', executable)
    source_receipt = None
    if args.source_receipt:
        snapshot = json.loads(args.source_receipt.read_text())
        for name, digest in snapshot['source'].items():
            assert sha(REPO / name) == digest, f'Source drift: {name}'
        source_receipt = sha(args.source_receipt)
    records = []
    for graph in GRAPHS:
        path = args.inputs / f'100000-{graph}.csv'
        meta = json.loads(path.with_suffix('.json').read_text())
        assert sha(path) == meta['sha256']
        for threads in args.threads:
            for mode in args.modes:
                key = f'{graph}-{mode}-t{threads}'
                result = subprocess.run([str(executable), str(path), mode, str(threads)],
                                        text=True, capture_output=True, timeout=180)
                (args.output / f'{key}.log').write_text(result.stdout + result.stderr)
                phases = {}
                for line in result.stderr.splitlines():
                    if line.startswith('FEVC_PIPELINE_PROFILE_V1\t'):
                        _, phase, calls, inclusive, exclusive = line.split('\t')
                        phases[phase] = dict(calls=int(calls), inclusive_seconds=int(inclusive)/1e9,
                                             exclusive_seconds=int(exclusive)/1e9)
                passed = result.returncode == 0 and 'DIAGNOSTIC_CORE_PASS' in result.stdout
                records.append(dict(key=key, input_sha256=sha(path), executable_sha256=sha(executable),
                                    source_receipt_sha256=source_receipt,
                                    returncode=result.returncode, passed=passed, phases=phases,
                                    result=result.stdout.strip()))
                (args.output / 'results.json').write_text(json.dumps(records, indent=2)+'\n')
                print(json.dumps(records[-1]), flush=True)
                if not passed:
                    raise RuntimeError(f'Diagnostic failed: {key}')


if __name__ == '__main__':
    main()
