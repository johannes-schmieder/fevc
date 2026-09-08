"""Source-bound existing-draw replay; never overrides the full-call PSD gate."""
from pathlib import Path
import argparse
import collections
import gzip
import hashlib
import importlib.util
import json
import math
import os
import platform
import re
import subprocess
import tarfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PROTOCOL = ROOT/'fevc/docs/observation_residual_moments_q1_target_audit_v1.json'
ORIGINAL = ROOT/'.local/diagnostics/residual-moment-confirmation-20260906'
TOOLCHAIN = Path('/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin')
spec = importlib.util.spec_from_file_location('target_audit', HERE/'audit.py')
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
MARKER = '    let mut result = finish_component_covariance(\n'
CORE = 'rust/crates/vckss-core/src/generic_jla.rs'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write('\n')


def finite(value):
    if isinstance(value, dict):
        for v in value.values():
            finite(v)
    elif isinstance(value, list):
        for v in value:
            finite(v)
    elif isinstance(value, float) and not math.isfinite(value):
        raise ValueError('nonfinite JSON value')


def original_inputs():
    plan = json.loads(PROTOCOL.read_text())
    paths = {
        'confirmation_manifest_sha256': ORIGINAL/'confirmation/manifest.json',
        'confirmation_result_sha256': ORIGINAL/'confirmation/result.json',
        'confirmation_bundle_sha256': ORIGINAL/'confirmation/source.tar.gz',
        'previous_geometry_export_sha256': ORIGINAL/'failure-audit/export.jsonl',
    }
    for key, path in paths.items():
        if sha(path) != plan[key]:
            raise ValueError(f'original input changed: {key}')
    manifest = json.loads(paths['confirmation_manifest_sha256'].read_text())
    for name, digest in manifest['source_files'].items():
        if sha(ROOT/name) != digest:
            raise ValueError(f'original source changed: {name}')
    result = json.loads(paths['confirmation_result_sha256'].read_text())
    failures = [r for r in result['failed_calls'] if (r['cell'], r['k']) == ('dominant_common_t8', 16)]
    if len(failures) != 43 or any(r['phase'] != 'component_inference_psd' for r in failures):
        raise ValueError('original failure inventory')
    failed = sorted(r['replication'] for r in failures)
    if len(set(failed)) != 43:
        raise ValueError('duplicate original failure')
    comparison = [i for i in range(2500) if i not in failed][:43]
    return plan, paths, manifest, failed, comparison


def instrument(text, capture):
    if text.count(MARKER) != 1:
        raise ValueError('capture insertion anchor is not unique')
    return text.replace(MARKER, capture+'\n'+MARKER, 1)


def build(out):
    _, paths, manifest, failed, comparison = original_inputs()
    out.mkdir(parents=True, exist_ok=False)
    source = out/'source'
    source.mkdir()
    with tarfile.open(paths['confirmation_bundle_sha256'], 'r:gz') as archive:
        archive.extractall(source, filter='data')
    for name, digest in manifest['source_files'].items():
        if sha(source/name) != digest:
            raise ValueError(f'bundle source mismatch: {name}')
    core = source/CORE
    original = core.read_text()
    # Mechanical diagnostic build generation, only in the disposable source copy.
    core.write_text(instrument(original, (HERE/'capture.rs').read_text()))
    changed = [name for name, digest in manifest['source_files'].items() if sha(source/name) != digest]
    if changed != [CORE]:
        raise ValueError(f'unexpected instrumented paths: {changed}')
    generated = (source/'generated/confirmation.rs').read_text()
    if generated.count('fn main()') != 1:
        raise ValueError('original entrypoint identity')
    generated = generated.replace('fn main()', 'fn original_confirmation_main()', 1)
    generated, count = re.subn(r'#\[path = "[^"]*common/q1_reference.rs"\]',
        f'#[path = "{source}/rust/crates/vckss-core/examples/common/q1_reference.rs"]', generated)
    if count != 1:
        raise ValueError('independent reference include identity')
    export = out/'export.rs'
    export.write_text(generated+'\n'+(HERE/'export.rs').read_text())
    env = dict(os.environ, PATH=f'{TOOLCHAIN}:{os.environ["PATH"]}', CARGO_TARGET_DIR=str(out/'target'))
    with (out/'build.log').open('x') as log:
        subprocess.run([str(TOOLCHAIN/'cargo'), 'build', '--release', '--locked', '--offline', '-p', 'vckss-core'],
                       cwd=source/'rust', env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        libraries = list((out/'target/release/deps').glob('libvckss_core-*.rlib'))
        if len(libraries) != 1:
            raise ValueError('diagnostic library inventory')
        subprocess.run([str(TOOLCHAIN/'rustc'), '--edition=2021', '-O', '-Awarnings', str(export),
                        '--extern', f'vckss_core={libraries[0]}', '-L', f'dependency={libraries[0].parent}',
                        '-o', str(out/'export')], stdout=log, stderr=subprocess.STDOUT, check=True)
    receipt = dict(status='DIAGNOSTIC_CAPTURE_BUILD_ONLY', original_inputs={str(p): sha(p) for p in paths.values()},
                   generated_sha256=sha(export), binary_sha256=sha(out/'export'),
                   capture_sha256=sha(HERE/'capture.rs'), adapter_sha256=sha(HERE/'export.rs'),
                   changed_original_paths=changed, instrumented_core_sha256=sha(core),
                   rejected_replications=failed, comparison_replications=comparison,
                   source_files=manifest['source_files'],
                   rustc=subprocess.check_output([str(TOOLCHAIN/'rustc'), '-Vv'], text=True).strip())
    write_json(out/'build-receipt.json', receipt)
    print(json.dumps({k: v for k, v in receipt.items() if k != 'source_files'}, indent=2))


def baseline(selected):
    result = {}
    files = {}
    original = json.loads((ORIGINAL/'confirmation/result.json').read_text())
    for path in sorted((ORIGINAL/'confirmation/tasks').glob('dominant_common_t8-16-*/rows.jsonl.gz')):
        receipt_path = path.with_name('receipt.json')
        receipt = json.loads(receipt_path.read_text())
        if sha(receipt_path) != original['task_receipt_sha256'][path.parent.name] or sha(path) != receipt['output_sha256']:
            raise ValueError('frozen baseline receipt/output changed')
        if receipt['manifest_sha256'] != original['manifest_sha256'] or receipt['status'] != 'execution_and_inventory_pass':
            raise ValueError('baseline receipt provenance')
        files[str(path)] = sha(path)
        files[str(receipt_path)] = sha(receipt_path)
        with gzip.open(path, 'rt') as stream:
            for line in stream:
                row = json.loads(line)
                if row.get('replication') not in selected:
                    continue
                if row['kind'] == 'call':
                    key = (row['replication'], 'call')
                elif row['kind'] == 'target' and row['arm'] == 'native':
                    key = (row['replication'], row['target'])
                else:
                    continue
                if key in result:
                    raise ValueError('duplicate original baseline')
                result[key] = row
    expected = {(r, t) for r in selected for t in ('call', *oracle.TARGETS)}
    if set(result) != expected:
        raise ValueError('original baseline inventory')
    return result, files


def validate(rows, selected, failed, base, n):
    finite(rows)
    if len(rows) != 1+7*len(selected) or rows[0]['kind'] != 'exact_modes':
        raise ValueError('capture row inventory')
    draws = []
    for i, rep in enumerate(selected):
        block = rows[1+7*i:1+7*(i+1)]
        start, captured, *rest = block
        targets, end = rest[:4], rest[4]
        if start['kind'] != 'start' or end['kind'] != 'end' or start['replication'] != rep or end['replication'] != rep:
            raise ValueError('draw key/order inventory')
        if start['seed'] != base[rep, 'worker']['seed']:
            raise ValueError('original semantic seed')
        if sorted(start['order']) != list(range(n)) or len(start['y']) != n:
            raise ValueError('physical row dimensions/order')
        if captured['kind'] != 'capture_variance' or len(captured['variance']) != n or min(captured['variance']) <= 0:
            raise ValueError('captured variance')
        expected_status = 'failed' if rep in failed else 'success'
        if end['status'] != expected_status:
            raise ValueError('original full-call availability changed')
        if rep in failed and (end['phase'] != 'component_inference_psd' or end['code'] != 'JLA_CONSTRAINT_FAILED'):
            raise ValueError('original full-call failure changed')
        for t, row in enumerate(targets):
            if row['kind'] != 'capture_target' or row['target'] != t or row['status'] not in (0, 1, 2, 3, 6):
                raise ValueError('target inventory/status')
            if any(len(row[key]) != n for key in ('mode', 'ratio', 'influence')) or len(row['q']) != len(oracle.Q_FIELDS):
                raise ValueError('target dimensions')
            nullable = {'determinant', 'curvature', 'critical', 'lower', 'upper'}
            if any(v is None and (row['status'] == 0 or key not in nullable) for key, v in zip(oracle.Q_FIELDS, row['q'])):
                raise ValueError('missing available target field')
            if row['critical_draws'] != (100000 if row['status'] == 0 else 0):
                raise ValueError('critical draw accounting')
            if rep not in failed:
                old = base[rep, oracle.TARGETS[t]]
                q = dict(zip(oracle.Q_FIELDS, row['q']))
                if old['status'] != 'success' or row['status'] != 0:
                    raise ValueError('successful comparison target changed')
                current = [q['point'], q['upper']-q['lower'], q['critical'], end['covariance'][t*4+t]]
                original = [old['point_error']+rows[0]['truth'][t], old['width'], old['critical'], old['variance']]
                if oracle.relative_error(current, original) > 1e-8:
                    raise ValueError('successful frozen baseline changed')
                if oracle.relative_error([q['point'], q['upper']-q['lower'], q['critical']],
                                         [end['points'][t], end['widths'][t], end['critical'][t]]) > 1e-8:
                    raise ValueError('capture changed final native result')
        draws.append((start, captured, targets, end))
    return rows[0], draws


def summarize(records):
    summary = {}
    for group in ('rejected', 'comparison'):
        summary[group] = {}
        for target in oracle.TARGETS:
            chosen = [r for r in records if r['group'] == group and r['target'] == target]
            summary[group][target] = {}
            for arm in records[0]['arms']:
                values = [r['arms'][arm] for r in chosen]
                success = [r for r in values if r['status'] == 0]
                summary[group][target][arm] = dict(attempts=len(values), computed=len(success),
                    statuses=dict(collections.Counter(str(r['status']) for r in values)),
                    selected_draws_covered=sum(r['covered'] for r in success),
                    minimum_determinant=min((r['determinant'] for r in success), default=None),
                    width_range=[min((r['width'] for r in success), default=None), max((r['width'] for r in success), default=None)])
    return summary


def check_build(out, receipt):
    if sha(out/'export') != receipt['binary_sha256'] or sha(out/'export.rs') != receipt['generated_sha256']:
        raise ValueError('diagnostic executable/source changed')
    if sha(HERE/'capture.rs') != receipt['capture_sha256'] or sha(HERE/'export.rs') != receipt['adapter_sha256']:
        raise ValueError('capture source changed')
    for name, digest in receipt['source_files'].items():
        if sha(ROOT/name) != digest:
            raise ValueError(f'production source changed: {name}')
        expected = receipt['instrumented_core_sha256'] if name == CORE else digest
        if sha(out/'source'/name) != expected:
            raise ValueError(f'disposable source changed: {name}')


def run(out, profile):
    plan, inputs, _, failed, comparison = original_inputs()
    receipt = json.loads((out/'build-receipt.json').read_text())
    check_build(out, receipt)
    selected = sorted(failed+comparison) if profile == 'full' else sorted([failed[0], comparison[0]])
    if profile == 'full' and json.loads((out/'pilot/result.json').read_text())['status'] != 'DIAGNOSTIC_COMPLETE_NOT_CONFIRMATION':
        raise ValueError('tiny end-to-end prerequisite missing')
    base, baseline_files = baseline(selected)
    folder = out/profile
    folder.mkdir(exist_ok=False)
    source_files = {str(p.relative_to(ROOT)): sha(p) for p in [PROTOCOL, *sorted(HERE.glob('*.py')), *sorted(HERE.glob('*.rs'))]}
    manifest = dict(status='FROZEN_EXISTING_DRAW_DIAGNOSTIC', profile=profile, protocol=plan,
                    selected_replications=selected, rejected_replications=failed,
                    source_files=source_files, binary_sha256=sha(out/'export'),
                    build_receipt_sha256=sha(out/'build-receipt.json'),
                    original_inputs={str(p): sha(p) for p in inputs.values()}, baseline_files=baseline_files,
                    expected_calls=len(selected), expected_targets=4*len(selected),
                    python=platform.python_version(), platform=platform.platform(), numpy=oracle.np.__version__)
    write_json(folder/'manifest.json', manifest)
    now = time.monotonic()
    with (folder/'export.jsonl').open('x') as stream, (folder/'stderr.log').open('x') as err:
        subprocess.run([str(out/'export'), ','.join(map(str, selected))], stdout=stream, stderr=err, check=True)
    if profile == 'pilot':
        with (folder/'deliberate-failure.log').open('x') as stream:
            bad = subprocess.run([str(out/'export'), '0,0'], stdout=stream, stderr=subprocess.STDOUT)
        if bad.returncode == 0:
            raise ValueError('duplicate CLI keys accepted')
    rows = [json.loads(line) for line in (folder/'export.jsonl').read_text().splitlines()]
    with inputs['previous_geometry_export_sha256'].open() as stream:
        geometry = json.loads(next(stream))
    modes, draws = validate(rows, selected, failed, base, geometry['n'])
    records, errors = oracle.inspect(geometry, modes, draws, failed)
    if len(records) != 4*len(selected):
        raise ValueError('independent target inventory')
    check_build(out, receipt)
    for name, digest in source_files.items():
        if sha(ROOT/name) != digest:
            raise ValueError('audit source changed during replay')
    result = dict(status='DIAGNOSTIC_COMPLETE_NOT_CONFIRMATION', profile=profile,
                  manifest_sha256=sha(folder/'manifest.json'), export_sha256=sha(folder/'export.jsonl'),
                  calls=len(selected), target_attempts=len(records), checks=errors,
                  original_full_call_rejections=sum(r in failed for r in selected),
                  summary=summarize(records), rows=records, elapsed_seconds=time.monotonic()-now,
                  limitation='Selected existing draws, not a coverage qualification or a waiver of the original FAIL. Public joint covariance/PSD rule unchanged.')
    write_json(folder/'result.json', result)
    print(json.dumps({k: v for k, v in result.items() if k not in ('rows', 'summary')}, indent=2), flush=True)
    print(json.dumps(result['summary'], indent=2), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=('build', 'pilot', 'full'))
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    if args.action == 'build':
        build(args.out.resolve())
    else:
        run(args.out.resolve(), args.action)
