"""Compare four public Stata calls with already-captured V4 native replay draws."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check(replay, plugin, output):
    output.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((replay / 'manifest.json').read_text())
    tasks = ['observation-diffuse_common-16-0000', 'observation-dominant_common-16-0000',
             'match_q0-diffuse_equal_independent-20-0000', 'match_q1-one_mode_equal_independent-20-0000']
    receipts = []
    for name in tasks:
        folder = output / name
        folder.mkdir()
        source = replay / 'tasks' / name
        receipt = json.loads((source / 'receipt.json').read_text())
        if receipt['output_sha256'] != sha(source / 'rows.jsonl.gz') or receipt['status'] != 'VALIDATED':
            raise ValueError('native source binding')
        task = next(t for t in manifest['tasks'] if t['id'] == name)
        rows = [json.loads(s) for s in gzip.open(source / 'rows.jsonl.gz', 'rt')]
        call = rows[2]
        if call['status'] != 'success':
            raise ValueError('parity fixture unavailable')
        q1 = task['family'] == 'match_q1' or task['cell'].startswith('dominant')
        observation = task['family'] == 'observation'
        deletion = 'deletion(observation)' if observation else 'deletion(match) deletionid(deletion) nuisance(fixedoffset)'
        weight = '' if observation else '[fw=frequency]'
        reference = 'q1' if q1 else 'highrank'
        lines = ['version 18.0', 'clear all', 'set more off', 'set type double', 'set processors 1',
            f'adopath ++ "{ROOT}/fevc"', f'adopath ++ "{plugin}"', f'quietly run "{ROOT}/fevc/fevc.ado"',
            f'import delimited using "{source}/fixture.csv", clear asdouble',
            f'quietly fevc outcome {weight}, worker(worker) firm(firm) {deletion} stayers(movers) backend(rust) engine(generic) rng(counter_v1) algorithm(jla) preconditioner(diagonal) batch(16) targetweight(target) inferencemodel(structured_common) inference({reference})',
            'assert e(probes)==200', 'assert e(inference_gram_probes)==2048',
            'assert "`e(inference_gram_method)\'"=="direct_residual_covariance"',
            f'assert e(inference_joint_status)=={call["joint"]}',
            f'assert e(inference_computed_targets)=={call["computed"]}',
            f'assert e(inference_counter_atoms)=={call["counter_atoms"]}',
            f'assert e(inference_counter_words)=={call["counter_words"]}']
        def close(expression, value):
            lines.append(f'assert missing({expression})' if value is None else
                         f'assert abs({expression}-({value:.17e}))<=1e-7*max(1,abs({value:.17e}))')
        for i in range(4):
            close(f'e(b)[1,{i+1}]', call['point'][i])
            lines.append(f'assert e(q0_status)[{i+1},1]=={call["targets"][2*i+1]}')
            if q1:
                for j in range(20):
                    close(f'e(component_q1_diagnostics)[{i+1},{j+1}]', call['q1'][20*i+j])
            elif call['targets'][2*i+1] == 0:
                close(f'e(component_inference)[{i+1},2]^2', call['targets'][2*i])
        if q1:
            lines += ['local matrices : e(matrices)', 'assert !strpos(" `matrices\' "," V ")']
        lines += ['quietly fevc_rust snapshot', 'assert r(state)==0', 'display "COMPLETION NATIVE PARITY PASS"', 'exit, clear']
        (folder / 'check.do').write_text('\n'.join(lines)+'\n')
        done = subprocess.run(['/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp', '-b', 'do', 'check.do'], cwd=folder, timeout=180)
        log = folder / 'check.log'
        if done.returncode or not log.exists() or '\nCOMPLETION NATIVE PARITY PASS\n' not in log.read_text():
            raise ValueError(f'Stata parity failed: {folder}')
        receipts.append(dict(task=name, status='PASS', native_sha256=sha(source/'rows.jsonl.gz'),
            fixture_sha256=sha(source/'fixture.csv'), driver_sha256=sha(folder/'check.do'), log_sha256=sha(log)))
    result = dict(status='PASS', tolerance='1e-7 * max(1,abs(native))', cases=receipts,
                  replay_manifest_sha256=sha(replay/'manifest.json'), script_sha256=sha(Path(__file__)),
                  plugin_sha256=sha(plugin/'fevc_rust_macos_arm64.plugin'))
    with (output / 'receipt.json').open('x') as stream:
        json.dump(result, stream, indent=2)
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('replay','plugin','output'):
        parser.add_argument(name, type=Path)
    args = parser.parse_args()
    print(json.dumps(check(args.replay.resolve(), args.plugin.resolve(), args.output.resolve()),indent=2))
