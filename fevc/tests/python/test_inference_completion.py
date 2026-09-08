"""Adversarial validation for the fixed engineering replay, using a synthetic draw."""
import copy
import importlib.util
import json
from pathlib import Path
import sys

import pytest

ROOT = Path(__file__).resolve().parents[3]
HERE = ROOT / 'rust/experiments/inference_completion'


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


saved = {name: sys.modules.get(name) for name in ('build', 'validation')}
try:
    load('build', HERE / 'build.py')
    RUN = load('completion_test_runner', HERE / 'run.py')
finally:
    for name, module in saved.items():
        if module is None:
            sys.modules.pop(name, None)
        else:
            sys.modules[name] = module


@pytest.fixture
def records():
    return json.loads((ROOT / 'fevc/tests/fixtures/direct_gram_replay.json').read_text())


def task(records):
    return dict(records[0], id='observation-diffuse_common-16-0000', category='primary')


def test_valid_synthetic_saved_draw_and_exact_status_accounting(records):
    original = copy.deepcopy(records)
    design, calls = RUN.validate(records, task(records))
    assert records == original
    assert len(calls) == 1 and design['n'] == 922
    assert not RUN.compare(calls[0], copy.deepcopy(calls[0]), True)


@pytest.mark.parametrize('fault', ['missing', 'duplicate', 'seed', 'count', 'words',
    'units', 'profile', 'dimension', 'nonfinite', 'target', 'partial', 'gram_rank', 'residual'])
def test_invalid_raw_output_cannot_pass(records, fault):
    expected = task(records)
    if fault == 'missing':
        records.pop()
    elif fault == 'duplicate':
        records.append(copy.deepcopy(records[-1]))
    elif fault == 'seed':
        records[-1]['seed'] += 1
    elif fault == 'count':
        records[-1]['gram_probes'] = 512
    elif fault == 'words':
        records[-1]['counter_words'] -= 1
    elif fault == 'units':
        records[-1]['units'] -= 1
    elif fault == 'profile':
        records[0]['profile'] = 'tiny'
    elif fault == 'dimension':
        records[1]['n'] += 1
    elif fault == 'nonfinite':
        records[-1]['point'][0] = float('nan')
    elif fault == 'target':
        records[-1]['targets'][1] = 99
    elif fault == 'partial':
        records[-1].pop('targets')
    elif fault == 'gram_rank':
        records[-1]['gram_rcond'] = [0.0]
    elif fault == 'residual':
        records[-1]['max_residual'] = 1.0
    with pytest.raises((ValueError, KeyError, AssertionError, TypeError)):
        RUN.validate(records, expected)


def test_shared_failures_are_attempts_and_same_route_failure_is_blocking(records):
    failure = dict(kind='call', replication=records[-1]['replication'], seed=records[-1]['seed'],
                   seconds=0.1, status='failure', detail='SINGULAR_INFORMATION')
    assert RUN.compare(failure, records[-1], True) == ['shared status']
    assert not RUN.compare(failure, records[-1], False)


def test_point_and_discrete_changes_are_blocking(records):
    old = records[-1]
    for field, change in [('point', 1e-5), ('targets', 1.0), ('gram_probes', 1), ('computed', 1)]:
        new = copy.deepcopy(old)
        if isinstance(new[field], list):
            new[field][0] += change
        else:
            new[field] += change
        assert RUN.compare(new, old, True)
    new = copy.deepcopy(old)
    new['seconds'] *= 2
    new['peak'] += 128
    assert not RUN.compare(new, old, True)


def test_frozen_recipe_and_additive_abi():
    registration = json.loads(RUN.REG.read_text())
    assert registration['native_calls'] == 70 and registration['target_attempts'] == 280
    assert registration['primary_replications'] == [0, 49, 99, 149, 199]
    assert registration['diagnostic_replications'] == [0, 9, 19, 29, 39]
    adapter = RUN.B.adapt((RUN.B.PREVIOUS.LEGACY / 'public_api.rs').read_text())
    assert 'interrupt_v4(generation,&attachment,2048)' in adapter
    assert 'inference_interface_version(), 4' in adapter
    for filename in ('fevc/fevc.ado', 'fevc/_fevc_component_model_route.ado'):
        assert 'inferencegramprobes' in (ROOT / filename).read_text().lower()
