"""Independent raw-inventory audit and old-fitter controls split reaggregation."""
import argparse
import collections
import gzip
import hashlib
import json
import math
from pathlib import Path
import statistics

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def audit(directory):
    manifest = read(directory / 'manifest.json')
    result = read(directory / 'result.json')
    errors, failed, counts = [], [], collections.Counter()
    if result['manifest_sha256'] != sha(directory / 'manifest.json'):
        errors.append('manifest binding')
    tasks = manifest['tasks']
    if len(tasks) != 70 or len({t['id'] for t in tasks}) != 70:
        errors.append('task count/uniqueness')
    if {p.name for p in (directory / 'tasks').iterdir()} != {t['id'] for t in tasks}:
        errors.append('directory inventory')
    for task in tasks:
        folder = directory / 'tasks' / task['id']
        try:
            receipt = read(folder / 'receipt.json')
            records = [json.loads(s) for s in gzip.open(folder / 'rows.jsonl.gz', 'rt')]
            if len(records) != 3 or [r['kind'] for r in records] != ['task', 'design', 'call']:
                raise ValueError('partial/duplicate/ordered records')
            if receipt['output_sha256'] != sha(folder / 'rows.jsonl.gz') or receipt['task'] != task:
                raise ValueError('receipt binding')
            if receipt['manifest_sha256'] != sha(directory / 'manifest.json') or receipt['status'] != 'VALIDATED':
                raise ValueError('task validation')
            call = records[2]
            if call['replication'] != task['start'] or task['reps'] != 1:
                raise ValueError('replication inventory')
            q1 = task['family'] == 'match_q1' or (task['family'] == 'observation' and task['cell'].startswith('dominant'))
            if call['status'] == 'success':
                if call['gram_probes'] != 2048 or call['fit'] != 2:
                    raise ValueError('method/count')
                atoms = call['units'] * 3178 + 2 * call['critical_draws']
                if call['counter_atoms'] != atoms or call['counter_words'] != 2 * atoms:
                    raise ValueError('counter count')
                codes = call['q1'][16::20] if q1 else call['targets'][1::2]
                if len(codes) != 4 or sum(c == 0 for c in codes) != call['computed']:
                    raise ValueError('computed target accounting')
                for index, code in enumerate(codes):
                    label = 'success' if code == 0 else f'target_{int(code)}'
                    counts[label] += 1
                    if code:
                        failed.append(dict(task=task['id'], target=index, status=label))
            else:
                counts['shared_failure'] += 4
                failed.append(dict(task=task['id'], targets=4, status='shared_failure', detail=call.get('detail')))
        except (ValueError, OSError, KeyError) as error:
            errors.append(f'{task["id"]}: {error}')
    if sum(counts.values()) != 280 or dict(counts) != result['status_counts'] or result['calls'] != 70:
        errors.append('aggregate target accounting')
    for name, digest in manifest['inputs'].items():
        if sha(name) != digest:
            errors.append('input hash: ' + name)
    if sha(directory / 'source.tar.gz') != manifest['source_bundle_sha256']:
        errors.append('source bundle hash')
    return dict(status='AUDIT_FAIL' if errors else 'AUDIT_PASS', errors=errors,
                calls=70, targets=sum(counts.values()), statuses=dict(counts), failures=failed,
                manifest_sha256=sha(directory / 'manifest.json'), result_sha256=sha(directory / 'result.json'),
                source_bundle_sha256=manifest['source_bundle_sha256'])


def controls_split():
    parent = ROOT / '.local/diagnostics/individual-inference-upgrade-20260906/public-development-1'
    calls, hashes, truth = {}, {}, None
    for start in range(0, 400, 100):
        folder = parent / 'tasks' / f'observation-diffuse_common_controls-16-{start:04d}'
        receipt = read(folder / 'receipt.json')
        path = folder / 'rows.jsonl.gz'
        if sha(path) != receipt['output_sha256']:
            raise ValueError('old controls input hash')
        rows = [json.loads(s) for s in gzip.open(path, 'rt')]
        this_truth = rows[1]['truth'][1]
        if truth is not None and truth != this_truth:
            raise ValueError('old controls truth disagreement')
        truth = this_truth
        for row in rows[2:]:
            if row['replication'] in calls:
                raise ValueError('old controls duplicate')
            calls[row['replication']] = row
        hashes[str(path)] = sha(path)
        hashes[str(folder / 'receipt.json')] = sha(folder / 'receipt.json')
    if set(calls) != set(range(400)):
        raise ValueError('old controls incomplete')
    summaries = []
    for start, stop in [(0, 200), (200, 400), (0, 400)]:
        rows = [calls[i] for i in range(start, stop)]
        if any(r['status'] != 'success' or r['targets'][3] != 0 for r in rows):
            raise ValueError('old controls failure: do not condition silently')
        points = [r['point'][1] for r in rows]
        variances = [r['targets'][2] for r in rows]
        sd = statistics.stdev(points)
        rms = math.sqrt(statistics.mean(variances))
        coverage = statistics.mean(abs(p-truth) <= 1.959963984540054*math.sqrt(v) for p,v in zip(points,variances))
        summaries.append(dict(start=start, stop=stop, attempts=stop-start, sd=sd, rms_se=rms,
                              sd_over_rms_se=sd/rms, coverage=coverage))
    return dict(fitter='historical observation residual-moment fitter, subtractive 512 Gram probes',
                warning='old-fitter outcomes; not a 400-draw validation of the direct 2048 candidate',
                truth=truth, input_hashes=hashes, summaries=summaries)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('run', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result = audit(args.run)
    result['old_controls_split'] = controls_split()
    result['auditor_sha256'] = sha(__file__)
    with args.output.open('x') as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
    print(json.dumps({k: result[k] for k in ('status','calls','targets','statuses','errors')}))
    raise SystemExit(result['status'] != 'AUDIT_PASS')
