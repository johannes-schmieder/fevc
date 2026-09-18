#!/usr/bin/env python3
"""Snapshot public implementation sources without changing the installed package."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

REPO = Path(__file__).resolve().parents[3]
SUFFIXES = {'.rs', '.toml', '.lock', '.c', '.h', '.py', '.do', '.ado', '.mata',
            '.sh', '.sge', '.json', '.txt', '.md', '.yml', '.yaml', '.sthlp',
            '.pkg', '.ini', '.bib', '.m', '.toc', '.def', '.inc', '.sha256'}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    parser.add_argument('--without-spectral-prototype', action='store_true',
                        help='Select the verified slice-07 spectral runtime/test in the new snapshot only')
    args = parser.parse_args()
    native_manifest = REPO.parent / 'fevc-paper/replication/results/raw/light_refresh_20260915/native-source-manifest.sha256'
    required_native = {line.split(None, 1)[1] for line in native_manifest.read_text().splitlines()}
    names = subprocess.check_output(['git', 'ls-files', '-c', '-o', '--exclude-standard', '-z',
        '--', 'rust', 'fevc', 'ci', 'tests', 'AGENTS.md', 'LICENSE', 'pyproject.toml',
        'pytest.ini', 'rust-toolchain.toml'], cwd=REPO).decode().split('\0')
    source = {}
    for name in sorted(set(filter(None, names))):
        path = REPO / name
        if path.suffix not in SUFFIXES and path.name not in {'LICENSE', 'COPYING'} and name not in required_native:
            continue
        if not path.is_file():
            continue
        assert not path.is_symlink() and path.stat().st_size < 10_000_000, name
        source[name] = sha(path)
    assert required_native <= source.keys(), f'Missing native inputs: {required_native - source.keys()}'
    origins = {name: REPO / name for name in source}
    selections = {}
    if args.without_spectral_prototype:
        area = REPO / '.local/pipeline-optimization-20260915'
        reference = area / 'slice-07'
        prototype = area / 'slice-08'
        assert sha(reference / 'snapshot.json') == '6431181069b10dd0014c822f75e04f0b8d1ab679133c099dedd63389035a11b3'
        assert sha(prototype / 'snapshot.json') == '73a57b570a8dff2c0a25df2a7152cc80af8ab1bbdeaa47777471c50f0cf2b225'
        old = json.loads((reference / 'snapshot.json').read_text())['source']
        new = json.loads((prototype / 'snapshot.json').read_text())['source']
        for name in ('rust/crates/vckss-core/src/component_inference.rs',
                     'rust/crates/vckss-core/src/generic_jla.rs',
                     'rust/crates/vckss-core/src/generic_jla/spectrum_batches.rs',
                     'rust/crates/vckss-core/tests/generic_jla/direct_component.rs'):
            assert source[name] == new[name], f'Overlapping later change: {name}'
            origin = reference / 'source' / name
            assert sha(origin) == old[name], f'Reference drift: {name}'
            selections[name] = dict(reference_snapshot_sha256=sha(reference / 'snapshot.json'),
                                    excluded_working_sha256=source[name], selected_sha256=old[name])
            origins[name], source[name] = origin, old[name]
    args.output.mkdir(parents=True, exist_ok=False)
    for name, digest in source.items():
        dest = args.output / 'source' / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(origins[name], dest)
        assert sha(dest) == digest
    baseline = REPO / '.local/pipeline-optimization-20260915/baseline/snapshot.json'
    before = json.loads(baseline.read_text())['source']
    receipt = dict(source=source, baseline_snapshot_sha256=sha(baseline),
                   source_selections=selections,
                   changed_from_baseline=[name for name in source if source[name] != before.get(name)],
                   git_head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(),
                   status='isolated development source; no performance or native qualification implied')
    target = args.output / 'snapshot.json'
    target.write_text(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(snapshot=str(target), sha256=sha(target), files=len(source))))


if __name__ == '__main__':
    main()
