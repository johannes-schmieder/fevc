"""Bounded paired numerical replay and read-only q1 diagnostic. No fresh DGPs."""
import argparse
import concurrent.futures
import gzip
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys

from numerics import analyze_target

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
REGISTRATION = ROOT/'fevc/docs/individual_inference_followup_v1.json'
Q1_CELLS = {('observation', 'dominant_common_controls', 16), ('match_q1', 'one_mode_equal_independent', 20)}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2, allow_nan=False)


def same(left, right):
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(same(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(same(a, b) for a, b in zip(left, right))
    if isinstance(left, float) and isinstance(right, (float, int)):
        return math.isclose(left, right, rel_tol=1e-12, abs_tol=1e-12)
    return left == right


def load_cells(parent):
    manifest = json.loads((parent/'manifest.json').read_text())
    result = json.loads((parent/'result.json').read_text())
    if result['manifest_sha256'] != sha(parent/'manifest.json') or manifest['bundle_sha256'] != sha(parent/'source.tar.gz'):
        raise ValueError('changed parent evidence')
    selected = {}
    for c in manifest['cells']:
        key = c['family'], c['cell'], c['k']
        q0 = c['family'] != 'observation' and c.get('reference', c.get('reference_distribution', '')).lower() == 'q0'
        if q0 or key in Q1_CELLS:
            selected[key] = {'q0': q0, 'calls': {}, 'files': {}, 'tasks': []}
    for task in manifest['tasks']:
        key = task['family'], task['cell'], task['k']
        if key not in selected:
            continue
        entry = selected[key]
        folder = parent/'tasks'/task['id']
        path = folder/'rows.jsonl.gz'
        receipt = json.loads((folder/'receipt.json').read_text())
        if receipt['task'] != task or receipt['output_sha256'] != sha(path) or receipt['binary_sha256'] != manifest['binaries'][task['family']]:
            raise ValueError('changed task identity')
        rows = [json.loads(line) for line in gzip.open(path, 'rt')]
        if len(rows) != task['reps']+2 or rows[0]['kind'] != 'task' or rows[1]['kind'] != 'design':
            raise ValueError('partial or malformed task')
        if 'truth' in entry and entry['truth'] != rows[1]['truth']:
            raise ValueError('changing target truth')
        entry['truth'] = rows[1]['truth']; entry['master'] = task['master']
        entry['tasks'].append(task)
        for p in (path, folder/'receipt.json'):
            entry['files'][str(p)] = sha(p)
        for call in rows[2:]:
            rep = call['replication']
            if call['kind'] != 'call' or rep in entry['calls'] or not task['start'] <= rep < task['start']+task['reps']:
                raise ValueError('duplicate or unexpected replication')
            entry['calls'][rep] = call
    for entry in selected.values():
        if set(entry['calls']) != set(range(400)):
            raise ValueError('missing saved replications')
    if set(Q1_CELLS)-selected.keys() or sum(e['q0'] for e in selected.values()) != 15:
        raise ValueError('wrong selected cell inventory')
    return manifest, selected


def build_candidate(output):
    spec = importlib.util.spec_from_file_location('individual_builder', HERE.parent/'individual_inference/build.py')
    builder = importlib.util.module_from_spec(spec); spec.loader.exec_module(builder)
    adapter = output/'adapter'; adapter.mkdir()
    for name in ('observation.rs', 'match.rs', 'public_api.rs'):
        shutil.copy2(builder.HERE/name, adapter/name)
    path = adapter/'public_api.rs'
    code = path.read_text(); old = 'if observation{512}else{128}'
    if code.count(old) != 1:
        raise ValueError('private diagnostic insertion identity')
    path.write_text(code.replace(old, 'if observation || !q1 {512}else{128}'))
    builder.HERE = adapter
    return builder.build(output/'build')


def replay(pair, old_build, candidate, output, cells):
    key = tuple(pair['key']); rep = pair['replication']; entry = cells[key]
    folder = output/'pairs'/pair['id']; folder.mkdir()
    actual = {}
    order = ('baseline', 'candidate') if pair['ordinal'] % 2 == 0 else ('candidate', 'baseline')
    for arm in order:
        binary = (old_build if arm == 'baseline' else candidate)/key[0]
        command = [str(binary), 'development', key[1], str(key[2]), str(rep), '1', str(entry['master']), 'individual-v1']
        env = dict(os.environ)
        if arm == 'candidate' and rep == 0:
            env['FEVC_INDIVIDUAL_FIXTURE'] = str(folder/'fixture.csv')
        with (folder/f'{arm}.jsonl').open('x') as log, (folder/f'{arm}.stderr').open('x') as err:
            run = subprocess.run(command, env=env, stdout=log, stderr=err, timeout=120)
        if run.returncode:
            raise ValueError(f'{pair["id"]}: {arm} process {run.returncode}')
        records = [json.loads(line) for line in (folder/f'{arm}.jsonl').read_text().splitlines()]
        if len(records) != 3 or records[2]['replication'] != rep or records[1]['truth'] != entry['truth']:
            raise ValueError('malformed replay inventory')
        actual[arm] = records[2]
    old, new = actual['baseline'], actual['candidate']
    strip_time = lambda row: {k: v for k, v in row.items() if k != 'seconds'}
    if not same(strip_time(old), strip_time(entry['calls'][rep])):
        raise ValueError(f'{pair["id"]}: old binary does not reproduce saved draw')
    summary = {**pair, 'baseline_seconds': old['seconds'], 'candidate_seconds': new['seconds'], 'resolved': 0}
    if not entry['q0'] or old['status'] != 'success':
        if not same(strip_time(old), strip_time(new)):
            raise ValueError('unchanged q1/shared failure replay mismatch')
    else:
        allowed = {'spectrum', 'targets', 'computed', 'solver_columns', 'max_residual', 'peak'}
        if not same({k: v for k, v in strip_time(old).items() if k not in allowed},
                    {k: v for k, v in strip_time(new).items() if k not in allowed}):
            raise ValueError('candidate changed a point/fit/covariance/Counter identity')
        for t in range(4):
            if not same(old['targets'][2*t], new['targets'][2*t]):
                raise ValueError('changed scalar variance')
            old_status, new_status = old['targets'][2*t+1], new['targets'][2*t+1]
            if new_status != (0 if old_status == 6 and new['targets'][2*t] > 0 else 1 if old_status == 6 else old_status):
                raise ValueError('unresolved or new target failure')
            summary['resolved'] += int(old_status == 6)
            if new['spectrum'][15*t+13] != 512 or max(new['spectrum'][15*t+10:15*t+12]) > .002:
                raise ValueError('spectral certificate failure')
        if new['solver_columns']-old['solver_columns'] != 4*4*(512-128):
            raise ValueError('spectral solve count does not reconcile')
        if new['max_residual'] > new['residual_gate'] or new['computed'] != sum(new['targets'][2*t+1] == 0 for t in range(4)):
            raise ValueError('numerical/result receipt failure')
        summary.update(max_spectral_residual=max(new['spectrum'][15*t+j] for t in range(4) for j in (10, 11)),
                       extra_solver_columns=new['solver_columns']-old['solver_columns'], extra_peak_bytes=new['peak']-old['peak'])
    write(folder/'receipt.json', {'status': 'PASS', **summary, 'files': {p.name: sha(p) for p in folder.iterdir() if p.is_file()}})
    return summary


def run(parent, output):
    output.mkdir(parents=True, exist_ok=False)
    manifest, cells = load_cells(parent)
    old_build = Path(manifest['build'])
    for family, digest in manifest['binaries'].items():
        if sha(old_build/family) != digest:
            raise ValueError('old binary changed')
    candidate = build_candidate(output)
    pairs = []
    for key, entry in sorted(cells.items()):
        for ordinal, rep in enumerate((0, 133, 266, 399) if entry['q0'] else (0, 399)):
            pairs.append({'id': '-'.join(map(str, (*key, rep))), 'key': key, 'replication': rep, 'ordinal': ordinal})
    files = {str(REGISTRATION): sha(REGISTRATION), str(parent/'manifest.json'): sha(parent/'manifest.json'),
             str(parent/'result.json'): sha(parent/'result.json'), str(candidate/'receipt.json'): sha(candidate/'receipt.json')}
    files.update({str(p): sha(p) for p in HERE.glob('*.py')})
    for entry in cells.values():
        files.update(entry['files'])
    write(output/'manifest.json', {'schema': 'individual-followup-v1', 'registration': json.loads(REGISTRATION.read_text()),
          'inputs': files, 'old_binaries': manifest['binaries'], 'candidate_binaries': json.loads((candidate/'receipt.json').read_text())['binaries'],
          'pairs': pairs, 'q1_saved_calls': 800, 'q1_target_diagnostics': 3200})
    (output/'pairs').mkdir(); summaries = []; failures = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        futures = {pool.submit(replay, pair, old_build, candidate, output, cells): pair for pair in pairs}
        for future in concurrent.futures.as_completed(futures):
            try:
                summaries.append(future.result())
            except Exception as error:
                failures.append({'pair': futures[future], 'error': str(error)})
    write(output/'replay-result.json', {'status': 'PASS' if not failures else 'FAIL', 'pairs': sorted(summaries, key=lambda r: r['id']), 'failures': failures,
          'resolved_selected_targets': sum(r['resolved'] for r in summaries),
          'median_q0_paired_runtime_ratio': statistics.median(r['candidate_seconds']/r['baseline_seconds'] for r in summaries if cells[tuple(r['key'])]['q0'])})
    q1_summaries = []
    for key in sorted(Q1_CELLS):
        entry = cells[key]; calls = [entry['calls'][i] for i in range(400)]
        for target in range(4):
            summary, raw = analyze_target(calls, entry['truth'][target], target)
            q1_summaries.append({'key': key, **summary})
            write(output/('-'.join(map(str, key))+f'-{target}-diagnostic-intervals.json'), raw)
    if any(sha(p) != digest for p, digest in files.items()):
        raise ValueError('input changed during diagnosis')
    write(output/'q1-result.json', {'status': 'NUMERICAL_DIAGNOSTICS_PASS_NOT_QUALIFICATION', 'targets': q1_summaries})
    write(output/'result.json', {'status': 'BOUNDED_CHECKS_PASS' if not failures else 'BOUNDED_CHECKS_FAIL',
          'manifest_sha256': sha(output/'manifest.json'), 'replay_result_sha256': sha(output/'replay-result.json'),
          'q1_result_sha256': sha(output/'q1-result.json'), 'original_scientific_status': manifest['profile']+':FAIL',
          'fresh_draws': 0, 'new_confirmation': False})
    print(output, 'FAIL' if failures else 'PASS', flush=True)
    return 1 if failures else 0


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('parent', type=Path); parser.add_argument('output', type=Path)
    args = parser.parse_args()
    sys.exit(run(args.parent.resolve(), args.output.resolve()))
