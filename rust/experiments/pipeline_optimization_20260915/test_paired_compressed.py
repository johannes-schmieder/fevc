import hashlib
import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace

import pytest


def harness():
    path = Path(__file__).with_name('paired_compressed.py')
    spec = importlib.util.spec_from_file_location('paired_compressed', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def prepared(tmp_path, monkeypatch):
    module = harness()
    for name in ('baseline', 'candidate'):
        root = tmp_path / name
        (root / 'source/rust').mkdir(parents=True)
        (root / 'source/public.txt').write_text(name)
        (root / 'snapshot.json').write_text(json.dumps({'source': {
            'public.txt': module.sha(root / 'source/public.txt')}}))
    inputs = tmp_path / 'inputs'
    inputs.mkdir()
    for graph in module.GRAPHS:
        path = inputs / f'100000-{graph}.csv'
        path.write_text('public-test-fixture')
        path.with_suffix('.json').write_text(json.dumps({'sha256': module.sha(path)}))

    def build(command, **_kwargs):
        assert '--release' in command and '--features' not in command
        binary = Path(command[command.index('--target-dir') + 1]) / 'release/examples/pipeline_profile'
        binary.parent.mkdir(parents=True)
        binary.write_text('test-executable-identity')
        return SimpleNamespace(returncode=0)

    monkeypatch.setattr(module.subprocess, 'run', build)
    args = SimpleNamespace(output=tmp_path / 'run', baseline=tmp_path / 'baseline',
                           candidate=tmp_path / 'candidate', inputs=inputs)
    module.prepare(args)
    manifest = json.loads((args.output / 'manifest.json').read_text())
    assert len(manifest['calls']) == 32
    assert sum(c['warmup'] for c in manifest['calls']) == 8
    for offset in range(0, 32, 2):
        pair = manifest['calls'][offset:offset+2]
        assert {c['variant'] for c in pair} == {'baseline', 'candidate'}
        assert len({(c['input_sha256'], c['threads'], c['repetition']) for c in pair}) == 1
    return module, args


def result(command, bad=False):
    candidate = 'build-candidate' in command[0]
    value = 2 if candidate and bad else 1
    seconds = 1 if candidate else 2
    return SimpleNamespace(returncode=0, stderr='', stdout=(
        f'DIAGNOSTIC_CORE_PASS rows=100000 threads={command[-1]} prepare=0.1 estimate={seconds} '
        f'worker={value} firm=1 covariance=0 total=2\n'))


def test_capture_free_inventory_rotation_validation_and_no_overwrite(tmp_path, monkeypatch):
    module, args = prepared(tmp_path, monkeypatch)
    monkeypatch.setattr(module.subprocess, 'run', lambda command, **_kwargs: result(command))
    module.run(args)
    report = json.loads((args.output / 'results.json').read_text())
    assert report['status'] == 'PASS_DIAGNOSTIC_ONLY'
    assert all(c['status'] == 'PASS' for c in report['calls'])
    assert all(c['median_speedup'] == 2 for c in report['pairs'])
    with pytest.raises(AssertionError, match='Never overwrite'):
        module.run(args)


def test_scientific_failure_preserves_failed_and_unattempted_calls(tmp_path, monkeypatch):
    module, args = prepared(tmp_path, monkeypatch)
    monkeypatch.setattr(module.subprocess, 'run', lambda command, **_kwargs: result(command, bad=True))
    with pytest.raises(AssertionError, match='worker'):
        module.run(args)
    report = json.loads((args.output / 'results.json').read_text())
    assert report['status'] == 'STOPPED'
    assert [c['status'] for c in report['calls']] == ['PASS', 'FAIL'] + ['UNATTEMPTED'] * 30


def test_binary_drift_rejects_before_calls(tmp_path, monkeypatch):
    module, args = prepared(tmp_path, monkeypatch)
    (args.output / 'build-candidate/release/examples/pipeline_profile').write_text('changed')
    with pytest.raises(AssertionError):
        module.run(args)
    assert not (args.output / 'results.json').exists()
