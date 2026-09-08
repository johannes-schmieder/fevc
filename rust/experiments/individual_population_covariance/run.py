"""Read-only capture of fixed numerical kernels; reuse only existing DGP seeds."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PREVIOUS = HERE.parent/'individual_inference_followup'
sys.path.insert(0, str(PREVIOUS))
spec = importlib.util.spec_from_file_location('paired_followup', PREVIOUS/'run.py')
prior = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prior)
sha, write, same = prior.sha, prior.write, prior.same
REGISTRATION = ROOT/'fevc/docs/individual_population_covariance_v1.json'
TOOLCHAIN = Path('/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin')
CORE = 'rust/crates/vckss-core/src/generic_jla.rs'


def replace_once(code, old, new):
    if code.count(old) != 1:
        raise ValueError(f'nonunique instrumentation anchor: {old[:100]}')
    return code.replace(old, new, 1)


def build(parent, output, manifest):
    source = output/'source'
    source.mkdir()
    with tarfile.open(parent/'source.tar.gz') as archive:
        archive.extractall(source, filter='data')
    for name, digest in manifest['source_files'].items():
        if sha(source/name) != digest:
            raise ValueError(f'frozen bundle source changed: {name}')
    # The parent snapshot selected .rs/.toml and omitted this build-time .rs.in.
    # Bind the unchanged tracked template explicitly; never alter CMG itself.
    template = 'rust/crates/vckss-core/src/cmg_impl.rs.in'
    committed = subprocess.check_output(['git','show',f'HEAD:{template}'],cwd=ROOT)
    if hashlib.sha256(committed).hexdigest() != sha(ROOT/template):
        raise ValueError('supplemental build template differs from HEAD')
    (source/template).write_bytes(committed)
    supplemental = {template: sha(source/template)}
    core = source/CORE
    anchor = '    let mut result = finish_component_covariance_with_reporting(\n'
    core.write_text(replace_once(core.read_text(), anchor, (HERE/'capture.rs').read_text()+'\n'+anchor))
    changed = [name for name, digest in manifest['source_files'].items() if sha(source/name) != digest]
    if changed != [CORE]:
        raise ValueError(f'unexpected instrumentation: {changed}')
    generated = {}
    for family in ('observation', 'match_q1'):
        code = (source/f'build/{family}.rs').read_text()
        adapter_name = 'observation.rs' if family == 'observation' else 'match.rs'
        adapter = (source/'rust/experiments/individual_inference'/adapter_name).read_text()
        edited = adapter
        if family == 'observation':
            anchor = '    for rep in start..start+reps {'
            edited = replace_once(edited, anchor,
                '    println!("{{\\\"kind\\\":\\\"population\\\",\\\"mean\\\":{:?},\\\"variance\\\":{:?}}}", mean, variance);\n'+anchor)
            inp = '&input'
        else:
            inp = '&generated.input'
            anchor = '        let truth = component_truth(&worker, &firm, &target_weight, &worker_effect, &firm_effect);'
            capture = '''        assert!(!cell.controls, "registered no-control match design");
        let population_mean: Vec<f64> = worker.iter().zip(&firm).map(|(&w,&f)| worker_effect[(w-10000) as usize]+firm_effect[(f-20000) as usize]).collect();
        println!("{{\\"kind\\":\\"population\\",\\"mean\\":{:?},\\"variance\\":{:?}}}", population_mean, aggregate_variance);
'''
            code = replace_once(code, anchor, capture+anchor)
        anchor = '        let now=std::time::Instant::now();'
        edited = replace_once(edited, anchor, f'        crate::population_input({inp},rep,seed);\n        if rep == 0 || rep == 399 {{\n'+anchor)
        emit = next(line for line in edited.splitlines() if '::emit(result,rep,seed' in line)
        edited = replace_once(edited, emit, emit+'\n        }')
        code = replace_once(code, adapter, edited)
        code, count = re.subn(r'#\[path = "[^"]*common/q1_reference.rs"\]',
            f'#[path = "{source}/rust/crates/vckss-core/examples/common/q1_reference.rs"]', code)
        if count != 1:
            raise ValueError('independent reference source anchor')
        code, count = re.subn(r'#\[path = "[^"]*/public_api.rs"\]',
            f'#[path = "{source}/rust/experiments/individual_inference/public_api.rs"]', code)
        if count != 1:
            raise ValueError('public ABI adapter source anchor')
        path = output/f'{family}.rs'
        path.write_text(code+'\n'+(HERE/'export.rs').read_text())
        generated[family] = sha(path)
    env = dict(os.environ, PATH=f'{TOOLCHAIN}:{os.environ["PATH"]}', CARGO_TARGET_DIR=str(output/'target'))
    with (output/'build.log').open('x') as log:
        subprocess.run([str(TOOLCHAIN/'cargo'), 'build', '--release', '--locked', '--offline', '-p', 'vckss-plugin'],
                       cwd=source/'rust', env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        deps = output/'target/release/deps'
        libraries = {name: [p for p in deps.glob('*.rlib')
                            if p.name == f'lib{name}.rlib' or p.name.startswith(f'lib{name}-')]
                     for name in ('vckss_core','vckss_plugin')}
        if any(len(v) != 1 for v in libraries.values()):
            raise ValueError('compiled library inventory')
        for family in generated:
            command = [str(TOOLCHAIN/'rustc'), '--edition=2021', '-O', '-Awarnings', str(output/f'{family}.rs'),
                       '-L', f'dependency={deps}', '-o', str(output/family)]
            for name, paths in libraries.items():
                command += ['--extern', f'{name}={paths[0]}']
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    write(output/'build-receipt.json', dict(changed_original_paths=changed, instrumented_core_sha256=sha(core),
          generated_sources=generated, binaries={f: sha(output/f) for f in generated},
          supplemental_build_sources=supplemental,
          supplemental_source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
          rustc=subprocess.check_output([str(TOOLCHAIN/'rustc'), '-Vv'], text=True).strip()))


def run(parent, output):
    manifest, cells = prior.load_cells(parent)
    plan = json.loads(REGISTRATION.read_text())
    if manifest['bundle_sha256'] != plan['parent_bundle_sha256'] or set(map(tuple,plan['cells'])) != prior.Q1_CELLS:
        raise ValueError('registered parent or cells mismatch')
    output.mkdir(parents=True, exist_ok=False)
    build(parent, output, manifest)
    inputs = {str(p): sha(p) for p in (REGISTRATION, parent/'manifest.json', parent/'result.json', parent/'source.tar.gz',
                                      PREVIOUS/'run.py', PREVIOUS/'numerics.py')}
    inputs.update({str(p): sha(p) for p in HERE.iterdir() if p.is_file()})
    inputs.update({str(ROOT/p):h for p,h in json.loads((output/'build-receipt.json').read_text())['supplemental_build_sources'].items()})
    for key in prior.Q1_CELLS:
        inputs.update(cells[key]['files'])
    write(output/'manifest.json', dict(schema=plan['schema'], registration=plan, inputs=inputs,
          build_receipt_sha256=sha(output/'build-receipt.json'), expected_inputs=800, expected_native_calls=4,
          parent=str(parent), new_outcomes=0))
    receipts = []
    for family, cell, k in sorted(prior.Q1_CELLS):
        entry = cells[family,cell,k]
        command = [str(output/family), 'development', cell, str(k), '0', '400', str(entry['master']), 'individual-v1']
        path, stderr = output/f'{family}.jsonl', output/f'{family}.stderr'
        with path.open('x') as log, stderr.open('x') as err:
            result = subprocess.run(command, stdout=log, stderr=err, timeout=120)
        receipt = dict(family=family, command=command, exit_code=result.returncode,
                       output_sha256=sha(path), stderr_sha256=sha(stderr))
        receipts.append(receipt)
        if result.returncode:
            write(output/'execution.json', dict(status='FAIL', receipts=receipts))
            raise ValueError(f'capture failed: {family}')
    write(output/'execution.json', dict(status='PASS', receipts=receipts))
    print(output, flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('parent', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    run(args.parent.resolve(), args.output.resolve())
