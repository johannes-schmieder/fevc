"""Frozen 70-call software replay. All outcomes are historical saved draws.

No coverage cutoff is evaluated here. The independent historical validators
retain their scientific checks; only a private metadata view maps the actual
2,048-probe accounting to their frozen 512-probe interface.
"""
import argparse
import collections
import concurrent.futures
import copy
import gzip
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import tarfile

import build as B

ROOT = B.ROOT
HERE = B.HERE
V = B.module('completion_previous_validation', HERE.parent / 'unified_residual_moments/validation.py')
sys.modules['validation'] = V
H = B.module('completion_previous_executor', HERE.parent / 'unified_residual_moments/run.py')
REG = ROOT / 'fevc/docs/inference_completion_v1.json'
BASE = ROOT / '.local/diagnostics'
ARCHIVE = BASE / 'residual-probe-2048-20260908/main'
PARENT = BASE / 'residual-probe-validation-20260908/main'
SAVED = BASE / 'individual-inference-upgrade-20260906/public-development-1'
sha, write = B.sha, B.write


def validate(records, task, arm='unified'):
    view = copy.deepcopy(records)
    for raw, row in zip(records[2:], view[2:]):
        if raw.get('status') != 'success':
            continue
        if raw.get('gram_probes') != 2048:
            raise ValueError('incorrect requested Gram count')
        units = raw.get('units', 0)
        if task['family'] == 'observation' and units != records[1].get('n'):
            raise ValueError('observation dimension/unit mismatch')
        atoms = (1000 + 128 + 2 + 2048) * units + 2 * raw.get('critical_draws', 0)
        if raw.get('counter_atoms') != atoms or raw.get('counter_words') != 2 * atoms:
            raise ValueError('incorrect actual Counter accounting')
        row['gram_probes'] = 512
        row['counter_atoms'] -= 1536 * units
        row['counter_words'] -= 3072 * units
    design, _ = V.validate(view, task, arm)
    return design, records[2:]


def tasks(profile):
    registration = json.loads(REG.read_text())
    cells = json.loads((ROOT / 'fevc/docs/residual_probe_validation_v1.json').read_text())['saved_outcomes']
    old = json.loads((PARENT / 'manifest.json').read_text())['tasks']
    result = []
    for category in ('primary', 'diagnostic'):
        reps = registration[f'{category}_replications'] if profile == 'main' else [0]
        for family, cell, k in cells[f'{category}_cells']:
            template = next(t for t in old if (t['family'], t['cell'], t['k']) == (family, cell, k))
            for rep in reps:
                result.append(dict(template, start=rep, reps=1, category=category,
                                   id=f'{family}-{cell}-{k}-{rep:04d}'))
    return result


def read_archive(directory, task, arm=None):
    manifest = json.loads((directory / 'manifest.json').read_text())
    old = next(t for t in manifest['tasks'] if
               all(t[k] == task[k] for k in ('family', 'cell', 'k')) and
               t['start'] <= task['start'] < t['start'] + t['reps'])
    folder = directory / 'tasks' / old['id']
    if arm:
        folder /= arm
    receipt = json.loads((folder / 'receipt.json').read_text())
    path = folder / 'rows.jsonl.gz'
    if receipt['status'] != 'VALIDATED' or receipt['task'] != old or sha(path) != receipt['output_sha256']:
        raise ValueError('invalid archived output identity')
    records = [json.loads(line) for line in gzip.open(path, 'rt')]
    calls = [r for r in records[2:] if r['replication'] == task['start']]
    if len(calls) != 1:
        raise ValueError('missing or duplicate archived replication')
    return records[1], calls[0], [path, folder / 'receipt.json']


def equivalent(a, b):
    if a is None or b is None:
        return a is b
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(equivalent(x, y) for x, y in zip(a, b))
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return math.isfinite(a) and math.isfinite(b) and abs(a-b) <= 1e-8 * max(1, abs(b))
    return a == b


def compare(new, old, exact_route):
    errors = []
    for field in ('replication', 'seed'):
        if new[field] != old[field]:
            errors.append(field)
    if exact_route and new['status'] != old['status']:
        errors.append('shared status')
    if new['status'] == old['status'] == 'success':
        for field in ('point', 'point_mcse'):
            if not V.same(new[field], old[field]):
                errors.append(field)
        if new['units'] != old['units']:
            errors.append('units')
        if exact_route:
            discrete = ('joint', 'fit', 'computed', 'gram_probes', 'ordering',
                        'nonpositive', 'floored', 'critical_draws', 'counter_atoms', 'counter_words')
            for field in discrete:
                if new[field] != old[field]:
                    errors.append(field)
            for field in ('targets', 'q1', 'spectrum', 'primitive', 'covariance', 'gram_rcond',
                          'gram_inverse_relres', 'fit_relres', 'positivity_floor'):
                if not equivalent(new[field], old[field]):
                    errors.append(field)
            if new['targets'][1::2] != old['targets'][1::2] or new['q1'][16::20] != old['q1'][16::20]:
                errors.append('target status')
    elif (exact_route and new['status'] != 'success' and old['status'] != 'success'
          and new.get('detail') != old.get('detail')):
        errors.append('failure type')
    return errors


def freeze(build, output, profile, prerequisite):
    receipt = json.loads((build / 'receipt.json').read_text())
    if receipt['status'] != 'BUILD_PASS' or receipt['sources'] != B.source_files():
        raise ValueError('build/current source mismatch')
    inventory = tasks(profile)
    if len(inventory) != (70 if profile == 'main' else 14) or len({t['id'] for t in inventory}) != len(inventory):
        raise ValueError('task inventory')
    inputs = {str(p): sha(p) for p in (REG, ROOT / 'fevc/docs/residual_probe_validation_v1.json',
        ARCHIVE / 'manifest.json', ARCHIVE / 'result.json', ARCHIVE.parent / 'independent_audit.json',
        PARENT / 'manifest.json', build / 'receipt.json')}
    for task in inventory:
        for directory, arm in [(PARENT, 'current')] + ([(ARCHIVE, None)] if task['category'] == 'primary' else []):
            _, _, files = read_archive(directory, task, arm)
            inputs.update({str(p): sha(p) for p in files})
    if profile == 'main':
        if prerequisite is None:
            raise ValueError('main requires tiny pipeline')
        tiny = json.loads((prerequisite / 'result.json').read_text())
        tm = json.loads((prerequisite / 'manifest.json').read_text())
        if tiny['status'] != 'ENGINEERING_REPLAY_PASS' or tm['build_receipt_sha256'] != sha(build / 'receipt.json'):
            raise ValueError('tiny pipeline identity')
        inputs[str(prerequisite / 'result.json')] = sha(prerequisite / 'result.json')
    output.mkdir(parents=True, exist_ok=False)
    with tarfile.open(output / 'source.tar.gz', 'x:gz') as archive:
        for name in sorted(receipt['sources']):
            archive.add(ROOT / name, arcname=name, recursive=False)
        for path in sorted(build.rglob('*')):
            if path.is_file():
                archive.add(path, arcname='adapter-build/' + str(path.relative_to(build)), recursive=False)
    binaries = {f: dict(path=str(build / 'bin' / f), sha256=digest) for f, digest in receipt['binaries'].items()}
    if any(sha(Path(info['path'])) != info['sha256'] for info in binaries.values()):
        raise ValueError('changed executable')
    manifest = dict(schema='FEVC-INFERENCE-COMPLETION-REPLAY-V1', profile=profile, tasks=inventory,
        expected_calls=len(inventory), expected_targets=4*len(inventory), inputs=inputs,
        build_receipt_sha256=sha(build / 'receipt.json'), source_files=receipt['sources'],
        source_bundle_sha256=sha(output / 'source.tar.gz'), binaries=binaries,
        source_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        source_binding='immutable dirty-source bundle; not a release', new_outcomes=0,
        workers=4, threads_per_call=1, registration=json.loads(REG.read_text()))
    write(output / 'manifest.json', manifest)
    return manifest


def aggregate(output):
    manifest = json.loads((output / 'manifest.json').read_text())
    errors, rows, statuses = [], [], collections.Counter()
    expected = {t['id'] for t in manifest['tasks']}
    actual = {p.name for p in (output / 'tasks').iterdir()} if (output / 'tasks').exists() else set()
    if actual != expected:
        errors.append('task directory inventory')
    for task in manifest['tasks']:
        try:
            folder = output / 'tasks' / task['id']
            receipt = json.loads((folder / 'receipt.json').read_text())
            path = folder / 'rows.jsonl.gz'
            if (receipt['status'] != 'VALIDATED' or receipt['task'] != task or
                receipt['manifest_sha256'] != sha(output / 'manifest.json') or
                receipt['binary_sha256'] != manifest['binaries'][task['family']]['sha256'] or
                receipt['output_sha256'] != sha(path)):
                raise ValueError('task receipt identity')
            records = [json.loads(line) for line in gzip.open(path, 'rt')]
            design, calls = validate(records, task)
            if len(calls) != 1 or receipt['calls'] != 1 or receipt['targets'] != 4 or receipt['design'] != design:
                raise ValueError('task receipt counts/design')
            call = calls[0]
            if receipt['shared_failures'] != int(call['status'] != 'success'):
                raise ValueError('shared failure accounting')
            old_design, old, _ = read_archive(PARENT, task, 'current')
            if not V.same(design, old_design):
                raise ValueError('saved design identity')
            discrepancies = compare(call, old, False)
            if task['category'] == 'primary':
                old_design, candidate, _ = read_archive(ARCHIVE, task)
                if not V.same(design, old_design):
                    raise ValueError('candidate design identity')
                discrepancies += compare(call, candidate, True)
            if discrepancies:
                raise ValueError('replay mismatch: ' + ', '.join(discrepancies))
            cell = V.OLD.spec_for(task)
            for target in range(4):
                result = V.target_row(call, design, cell, target)
                statuses[result['status']] += 1
                rows.append(dict(task=task['id'], target=V.OLD.TARGETS[target], result=result,
                                 shared_status=call['status'], old_shared_status=old['status'],
                                 gram_rcond=call.get('gram_rcond'), floored=call.get('floored')))
        except (KeyError, ValueError, OSError, StopIteration, TypeError) as error:
            errors.append(f'{task["id"]}: {error}')
    for name, digest in manifest['inputs'].items():
        if sha(Path(name)) != digest:
            errors.append('changed input: ' + name)
    for info in manifest['binaries'].values():
        if sha(Path(info['path'])) != info['sha256']:
            errors.append('changed binary: ' + info['path'])
    if sha(output / 'source.tar.gz') != manifest['source_bundle_sha256']:
        errors.append('changed source bundle')
    if len(rows) != manifest['expected_targets']:
        errors.append(f'target inventory {len(rows)}/{manifest["expected_targets"]}')
    result = dict(status='ENGINEERING_REPLAY_FAIL' if errors else 'ENGINEERING_REPLAY_PASS',
        calls=len(rows)//4, targets=len(rows), status_counts=dict(statuses), errors=errors, rows=rows,
        manifest_sha256=sha(output / 'manifest.json'), coverage_claim=False)
    write(output / 'result.json', result)
    return result


def execute(task, manifest, output):
    receipt = H.execute(Path(manifest['binaries'][task['family']]['path']),
                        output / 'tasks' / task['id'], task, 'unified',
                        sha(output / 'manifest.json'), fixture=True)
    print(task['id'], receipt['status'], flush=True)
    return receipt


def run(build, output, profile, prerequisite):
    os.environ['RAYON_NUM_THREADS'] = '1'
    manifest = freeze(build.resolve(), output.resolve(), profile, prerequisite)
    H.validate = validate
    if profile == 'tiny':
        for family, info in manifest['binaries'].items():
            with (output / f'{family}-malformed.log').open('x') as stream:
                done = subprocess.run([info['path'], 'malformed'], stdout=stream, stderr=subprocess.STDOUT, timeout=30)
            if done.returncode == 0:
                raise ValueError('malformed CLI accepted')
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        list(pool.map(lambda task: execute(task, manifest, output), manifest['tasks']))
    result = aggregate(output)
    print(result['status'], result['calls'], result['targets'], result['errors'], flush=True)
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('profile', choices=['tiny', 'main'])
    parser.add_argument('--build', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--prerequisite', type=Path)
    args = parser.parse_args()
    result = run(args.build, args.output, args.profile, args.prerequisite)
    raise SystemExit(result['status'] != 'ENGINEERING_REPLAY_PASS')
