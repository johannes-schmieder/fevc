"""Frozen existing-draw match audit. No production-source edits or new draws."""
from pathlib import Path
import argparse
from concurrent.futures import ThreadPoolExecutor
import gzip
import hashlib
import json
import os
import subprocess
import tarfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PROTOCOL = ROOT/'fevc/docs/match_target_interval_audit_v1.json'
Q0 = Path('/private/tmp/fevc-rcq0-bb580fe/scc-confirmation')
Q1 = Path('/private/tmp/fevc-repair-4a68ea2-confirmation/collected')
TOOLCHAIN = Path('/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin')
CORE = 'rust/crates/vckss-core/src/generic_jla.rs'
MARKER = '    let mut result = finish_component_covariance(\n'
TARGETS = ('worker', 'firm', 'covariance', 'total')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    with Path(path).open('x') as f:
        json.dump(value, f, indent=2, allow_nan=False)
        f.write('\n')


def select(rows, q):
    calls = {}
    for r in rows:
        key = (r['cell'], r['replication'])
        targets = calls.setdefault(key, {})
        if r['target'] in targets or r['target'] not in TARGETS or r['k'] != 20:
            raise ValueError('duplicate, invalid target or dimension in original')
        targets[r['target']] = r
    if len(calls) != 35000 or any(set(rs) != set(TARGETS) for rs in calls.values()):
        raise ValueError('original inventory is incomplete')
    failed = {key for key, rs in calls.items() if any(r.get('error_phase') == 'component_inference_psd' for r in rs.values())}
    if len(failed) != (189 if q == 'q0' else 5000):
        raise ValueError('original PSD inventory changed')
    for key in failed:
        if any(r.get('error_phase') != 'component_inference_psd' or r['status'] != 'backend_failure' for r in calls[key].values()):
            raise ValueError('inconsistent shared failure')
    cells = sorted({k[0] for k in failed}) if q == 'q0' else ['one_mode_unequal_independent']
    comparison = set()
    for cell in cells:
        successes = sorted(k for k, rs in calls.items() if k[0] == cell and all(r['status'] == 'success' for r in rs.values()))
        if len(successes) < 4:
            raise ValueError('comparison inventory')
        comparison.update(successes[:4])
    dense = failed | comparison if q == 'q0' else set(comparison)
    if q == 'q1':
        for cell in sorted({k[0] for k in failed}):
            ordered = sorted(k for k in failed if k[0] == cell)
            dense.update(ordered[:32] + ordered[-32:])
    selected = [dict(q=q, cell=c, replication=r, group='rejected' if (c, r) in failed else 'comparison',
                     dense=(c, r) in dense, baseline=calls[c, r]) for c, r in sorted(failed | comparison)]
    excluded = [dict(cell=c, replication=rep, phase=rs['worker'].get('error_phase'))
                for (c, rep), rs in sorted(calls.items())
                if rs['worker']['status'] == 'backend_failure' and (c, rep) not in failed]
    return selected, excluded


def freeze(out):
    plan = json.loads(PROTOCOL.read_text())
    out.mkdir(parents=True, exist_ok=False)
    inputs = {'source_bundle_sha256': Q0/'input/source.tar.gz',
              'q0_manifest_sha256': Q0/'input/campaign-manifest.json',
              'q0_aggregate_sha256': Q0/'output/aggregate/aggregate.jsonl',
              'q1_manifest_sha256': Q1/'campaign-manifest.json',
              'q1_aggregate_sha256': Q1/'output/aggregate/aggregate.jsonl'}
    for key, p in inputs.items():
        if sha(p) != plan[key]:
            raise ValueError(f'original input changed: {key}')
    selected, excluded = [], {}
    for q in ('q0', 'q1'):
        with inputs[q+'_aggregate_sha256'].open() as f:
            s, e = select([json.loads(line) for line in f], q)
        selected.extend(s)
        excluded[q] = e
    if len(selected) != plan['validation']['expected_calls'] or sum(c['dense'] for c in selected) != plan['validation']['expected_dense_calls']:
        raise ValueError('registered selection counts')
    # This check precedes source reuse: the old q0 and q1 production/build/RNG
    # trees are byte-identical. Differences are confined to additional examples.
    paths = ['rust/Cargo.toml', 'rust/Cargo.lock', 'rust/crates/vckss-core/src', 'rust/crates/vckss-core/Cargo.toml', 'rust/vendor',
             'rust/crates/vckss-core/examples/inference_repair_match.rs', 'rust/crates/vckss-core/examples/common/q1_reference.rs']
    subprocess.run(['git', 'diff', '--exit-code', plan['q1_source_commit'], plan['source_commit'], '--', *paths], cwd=ROOT, check=True)
    manifest = dict(schema='FEVC_MATCH_TARGET_REPLAY_MANIFEST_V1', protocol_sha256=sha(PROTOCOL),
                    original_inputs={str(p): sha(p) for p in inputs.values()},
                    selected=selected, excluded_non_psd=excluded,
                    audit_sources={str(p.relative_to(ROOT)): sha(p) for p in sorted(HERE.glob('*')) if p.is_file()},
                    oracle_source_sha256=sha(HERE.parent/'residual_moment_q1_target_audit/audit.py'),
                    compatibility_paths=paths)
    write_json(out/'manifest.json', manifest)
    print(json.dumps({'calls': len(selected), 'dense': sum(c['dense'] for c in selected), 'manifest_sha256': sha(out/'manifest.json')}), flush=True)


def instrument(original, capture):
    if original.count(MARKER) != 1:
        raise ValueError('capture anchor not unique')
    return original.replace(MARKER, capture+'\n'+MARKER, 1)


def build(out):
    manifest = json.loads((out/'manifest.json').read_text())
    for p, digest in manifest['audit_sources'].items():
        if sha(ROOT/p) != digest:
            raise ValueError('audit source changed after freeze')
    with tarfile.open(Q0/'input/source.tar.gz') as archive:
        archive.extractall(out, filter='data')
    source = out/'source'
    # Bind every Rust source and build input, not just the modified module.
    originals = {str(p.relative_to(source)): sha(p) for p in sorted((source/'rust').rglob('*')) if p.is_file()}
    core = source/CORE
    core.write_text(instrument(core.read_text(), (HERE/'capture.rs').read_text()))
    env = dict(os.environ, PATH=f'{TOOLCHAIN}:{os.environ["PATH"]}', CARGO_TARGET_DIR=str(out/'target'))
    with (out/'build.log').open('x') as log:
        subprocess.run([str(TOOLCHAIN/'cargo'), 'build', '--release', '--locked', '--offline', '-p', 'vckss-core'],
                       cwd=source/'rust', env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        libraries = list((out/'target/release/deps').glob('libvckss_core-*.rlib'))
        if len(libraries) != 1:
            raise ValueError('library inventory')
        for q, name in [('q0', 'rc_match_q0'), ('q1', 'inference_repair_match')]:
            code = (source/f'rust/crates/vckss-core/examples/{name}.rs').read_text()
            anchor = '    pub fn entry() {'
            if code.count(anchor) != 1 or code.count('    campaign::entry();') != 1:
                raise ValueError('export insertion anchors')
            code = code.replace(anchor, (HERE/'export.rs').read_text()+'\n'+anchor, 1).replace('    campaign::entry();', '    campaign::audit_entry();')
            code = code.replace('#[path = "common/q1_reference.rs"]', f'#[path = "{source}/rust/crates/vckss-core/examples/common/q1_reference.rs"]')
            export = out/f'{q}.rs'
            export.write_text(code)
            subprocess.run([str(TOOLCHAIN/'rustc'), '--edition=2021', '-O', '-Awarnings', str(export), '--extern',
                            f'vckss_core={libraries[0]}', '-L', f'dependency={libraries[0].parent}', '-o', str(out/q)],
                           stdout=log, stderr=subprocess.STDOUT, check=True)
    changed = [p for p, digest in originals.items() if sha(source/p) != digest]
    if changed != [CORE]:
        raise ValueError('unexpected production source modification')
    write_json(out/'build-receipt.json', dict(manifest_sha256=sha(out/'manifest.json'), original_source_files=originals,
               changed_paths=changed, instrumented_core_sha256=sha(core), binaries={q:sha(out/q) for q in ('q0','q1')},
               generated_sources={q:sha(out/f'{q}.rs') for q in ('q0','q1')},
               rustc=subprocess.check_output([str(TOOLCHAIN/'rustc'), '-Vv'], text=True).strip()))
    print('Diagnostic build complete', flush=True)


def execute(out, profile):
    manifest = json.loads((out/'manifest.json').read_text())
    build_receipt = json.loads((out/'build-receipt.json').read_text())
    if sha(out/'manifest.json') != build_receipt['manifest_sha256'] or any(sha(out/q) != h for q,h in build_receipt['binaries'].items()):
        raise ValueError('frozen binary/manifest changed')
    chosen = manifest['selected']
    if profile == 'tiny':
        chosen = [next(c for c in chosen if c['q'] == q and c['group'] == g)
                  for q in ('q0', 'q1') for g in ('rejected', 'comparison')]
    dest = out/profile
    dest.mkdir(exist_ok=False)
    def one(c):
        key = f'{c["q"]}-{c["cell"]}-{c["replication"]:04d}'
        start = time.monotonic()
        env = dict(os.environ, FEVC_MATCH_AUDIT_DENSE='1' if c['dense'] else '0', RAYON_NUM_THREADS='1')
        proc = subprocess.run([str(out/c['q']), c['cell'], str(c['replication']), env['FEVC_MATCH_AUDIT_DENSE']],
                              capture_output=True, env=env)
        with gzip.open(dest/f'{key}.jsonl.gz', 'xb') as f:
            f.write(proc.stdout)
        with (dest/f'{key}.stderr').open('xb') as f:
            f.write(proc.stderr)
        receipt = dict(key=key, exit_code=proc.returncode, seconds=time.monotonic()-start,
                       output_sha256=sha(dest/f'{key}.jsonl.gz'), stderr_sha256=sha(dest/f'{key}.stderr'),
                       manifest_sha256=sha(out/'manifest.json'), binary_sha256=build_receipt['binaries'][c['q']])
        write_json(dest/f'{key}.receipt.json', receipt)
        return receipt
    start = time.monotonic()
    receipts = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        for receipt in pool.map(one, chosen):
            receipts.append(receipt)
            if len(receipts) % 250 == 0:
                print(f'{len(receipts)}/{len(chosen)} calls captured ({time.monotonic()-start:.1f}s)', flush=True)
    write_json(dest/'execution.json', dict(profile=profile, expected_calls=len(chosen), seconds=time.monotonic()-start,
               failed_processes=[r for r in receipts if r['exit_code']], receipt_hashes={r['key']:sha(dest/f'{r["key"]}.receipt.json') for r in receipts}))
    if any(r['exit_code'] for r in receipts):
        raise ValueError('at least one replay process failed; all attempts retained')
    print(f'{profile} captured {len(receipts)} calls in {time.monotonic()-start:.1f}s', flush=True)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('action', choices=['freeze', 'build', 'tiny', 'replay'])
    p.add_argument('output', type=Path)
    args = p.parse_args()
    out = args.output.resolve()
    if args.action == 'freeze': freeze(out)
    elif args.action == 'build': build(out)
    else: execute(out, args.action)


if __name__ == '__main__':
    main()
