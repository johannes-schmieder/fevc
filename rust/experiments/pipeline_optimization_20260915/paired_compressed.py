#!/usr/bin/env python3
"""Small, rotated, capture-free core diagnostic; NOT a native acceptance screen."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import time

TOOLCHAIN = Path('/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin')
GRAPHS = ('degree4_bottleneck', 'mixed_segmented')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_snapshot(path):
    receipt = path / 'snapshot.json'
    for name, digest in json.loads(receipt.read_text())['source'].items():
        assert sha(path / 'source' / name) == digest, f'Source drift: {name}'
    return sha(receipt)


def prepare(args):
    args.output.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ, PATH=f'{TOOLCHAIN}:/opt/homebrew/bin:/usr/bin:/bin')
    variants = {}
    for name, snapshot in [('baseline', args.baseline), ('candidate', args.candidate)]:
        identity = verify_snapshot(snapshot)
        target = args.output / f'build-{name}'
        command = [str(TOOLCHAIN / 'cargo'), 'build', '--locked', '--offline', '--release',
                   '--target-dir', str(target), '-p', 'vckss-core', '--example', 'pipeline_profile']
        with (args.output / f'build-{name}.log').open('x') as stream:
            result = subprocess.run(command, cwd=snapshot / 'source/rust', env=environment,
                                    stdout=stream, stderr=subprocess.STDOUT, timeout=600)
        assert result.returncode == 0, name
        assert verify_snapshot(snapshot) == identity
        binary = target / 'release/examples/pipeline_profile'
        variants[name] = dict(snapshot=str(snapshot), snapshot_sha256=identity,
                              binary=str(binary), binary_sha256=sha(binary), build_command=command)
    calls = []
    for graph in GRAPHS:
        path = args.inputs / f'100000-{graph}.csv'
        identity = sha(path)
        assert json.loads(path.with_suffix('.json').read_text())['sha256'] == identity
        for threads in (1, 7):
            for repetition in range(4):
                order = ['baseline', 'candidate']
                if repetition % 2:
                    order.reverse()
                for variant in order:
                    calls.append(dict(graph=graph, input=str(path), input_sha256=identity,
                                      threads=threads, repetition=repetition, warmup=repetition == 0,
                                      variant=variant, status='UNATTEMPTED'))
    manifest = dict(schema='FEVC-COMPRESSED-CORE-DIAGNOSTIC-V1', variants=variants, calls=calls,
                    scope='100k compressed match; 24 measured calls plus 8 warm-ups; no promotion gate',
                    exclusions='native preparation reuse, Stata, Matlab, SCC, complete command, RSS',
                    probes=200, seed=104729, capture=False, script_sha256=sha(Path(__file__)))
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print('PREPARED: execute only after competing local build/test work finishes', flush=True)


def run(args):
    manifest_path = args.output / 'manifest.json'
    manifest = json.loads(manifest_path.read_text())
    assert sha(Path(__file__)) == manifest['script_sha256']
    results_path = args.output / 'results.json'
    assert not results_path.exists(), 'Never overwrite an attempted diagnostic'
    for variant in manifest['variants'].values():
        assert verify_snapshot(Path(variant['snapshot'])) == variant['snapshot_sha256']
        assert sha(Path(variant['binary'])) == variant['binary_sha256']
    records = manifest['calls']
    report = dict(manifest_sha256=sha(manifest_path), host=platform.node(), machine=platform.machine(),
                  calls=records, status='RUNNING')

    def save():
        results_path.write_text(json.dumps(report, indent=2) + '\n')

    save()
    for index, call in enumerate(records):
        call['status'] = 'ATTEMPTED'
        save()
        started = time.monotonic()
        try:
            assert sha(Path(call['input'])) == call['input_sha256']
            binary = manifest['variants'][call['variant']]['binary']
            result = subprocess.run([binary, call['input'], 'match', str(call['threads'])],
                                    text=True, capture_output=True, timeout=300)
            (args.output / f'call-{index:02d}.log').write_text(result.stdout + result.stderr)
            call['process_seconds'] = time.monotonic() - started
            call['returncode'] = result.returncode
            assert result.returncode == 0 and result.stdout.startswith('DIAGNOSTIC_CORE_PASS ')
            assert 'FEVC_PIPELINE_PROFILE_V1' not in result.stderr, 'Capture-enabled binary'
            values = dict(field.split('=') for field in result.stdout.split()[1:])
            assert int(values['rows']) == 100_000 and int(values['threads']) == call['threads']
            call['values'] = {name: float(value) for name, value in values.items()}
            assert all(math.isfinite(value) for value in call['values'].values())
            assert call['values']['estimate'] > 0
            for previous in records[:index]:
                if (previous['graph'], previous['threads'], previous['repetition']) == (call['graph'], call['threads'], call['repetition']):
                    for key in ('worker', 'firm', 'covariance', 'total'):
                        left, right = previous['values'][key], call['values'][key]
                        assert abs(left-right) <= 1e-8 * max(1, abs(left), abs(right)), key
            call['status'] = 'PASS'
        except Exception as error:
            call.update(status='FAIL', error=repr(error))
            report['status'] = 'STOPPED'
            save()
            raise
        save()
        print(f"PASS {index+1}/{len(records)} {call['graph']} T={call['threads']} {call['variant']}", flush=True)
    pairs = []
    for graph in GRAPHS:
        for threads in (1, 7):
            ratios = []
            for repetition in (1, 2, 3):
                cell = {c['variant']: c['values']['estimate'] for c in records
                        if (c['graph'], c['threads'], c['repetition']) == (graph, threads, repetition)}
                ratios.append(cell['baseline'] / cell['candidate'])
            pairs.append(dict(graph=graph, threads=threads, speedups=ratios,
                              median_speedup=statistics.median(ratios)))
    report.update(status='PASS_DIAGNOSTIC_ONLY', pairs=pairs)
    save()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'run'])
    parser.add_argument('output', type=Path)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--candidate', type=Path)
    parser.add_argument('--inputs', type=Path)
    arguments = parser.parse_args()
    arguments.output = arguments.output.resolve()
    if arguments.action == 'prepare':
        for key in ('baseline', 'candidate', 'inputs'):
            value = getattr(arguments, key)
            if value is None:
                parser.error(f'--{key} is required for prepare')
            setattr(arguments, key, value.resolve(strict=True))
        prepare(arguments)
    else:
        run(arguments)
