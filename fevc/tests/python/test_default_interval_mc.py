"""Monte Carlo harness of the default interval (fevc/tools/default_interval), without Stata: design
generator, manifest, inventory, aggregation and verdict on synthetic task outputs."""

import hashlib
import importlib
import json
import os
import sys
from pathlib import Path

import numpy as np
import pandas as pd
import pytest

ROOT = Path(__file__).resolve().parents[3]
TOOL = ROOT / "fevc/tools/default_interval"


def _load():
    # The harness modules import each other by plain name from their own directory; drop any module
    # of the same name loaded from elsewhere first.
    for name in ("designs", "pkgmc", "aggregate", "confirm"):
        mod = sys.modules.get(name)
        if mod is not None and Path(getattr(mod, "__file__", "") or "").parent != TOOL:
            del sys.modules[name]
    sys.path.insert(0, str(TOOL))
    return [importlib.import_module(n) for n in ("designs", "pkgmc", "aggregate", "confirm")]


designs, pkgmc, aggregate, confirm = _load()

PER = ('truth', 'b', 'se', 'nlb', 'nub', 'qlb', 'qub', 'lb', 'ub', 'regime', 'share', 'kappa',
       'crit', 'eigshare', 'maxw', 'conv', 'q1status', 'lam', 'b1_', 'vb_lo', 'vb_fit', 'vr')


def task_frame(t, man, rng, share=0.2):
    """A well-formed task output: normal interval of half-width 0.2 and q = 1 interval of half-width
    0.3 around b."""
    n = t['reps']
    rows = {}
    rows.update(cell=[t['cell']] * n, deletion=[t['deletion']] * n, task=[t['task']] * n,
                rep=list(range(t['first'], t['first'] + n)))
    rows['seed'] = [t['seed_base']] * n
    rows['stream'] = [r + 1 for r in rows['rep']]
    rows.update(rc=[0] * n, status=['FEVC_REPEAT_MEASUREMENT_SE'] * n,
                model=[man['expect_model']] * n, reason=[np.nan] * n,
                n_retained=[man['designs'][t['cell']]['person_years']] * n, seconds=[1.0] * n,
                q1_components=[0] * n, steps=[10] * n, unresolved=[0] * n, joint_posted=[1] * n,
                threshold=[0.05] * n)
    for j in range(1, 5):
        b = 1.0 + 0.1 * rng.standard_normal(n)
        se = np.full(n, 0.2 / 1.959964)
        lam, leadvar = 1.0, 0.01
        square = 2 * lam ** 2 * leadvar ** 2 / se ** 2
        # so that the share is as asked
        score = np.sqrt(leadvar + (share - square) * se ** 2 / (4 * lam ** 2 * leadvar))
        regime = np.full(n, 1.0 if (share > 0.05 and j < 4) else 0.0)
        vals = dict(truth=np.full(n, 1.0), b=b, se=se, nlb=b - 0.2, nub=b + 0.2, qlb=b - 0.3,
                    qub=b + 0.3, regime=regime, share=np.full(n, share), kappa=np.full(n, 0.4),
                    crit=np.full(n, 2.0), eigshare=np.full(n, 0.3), maxw=np.full(n, 0.01),
                    conv=np.ones(n), q1status=np.zeros(n), lam=np.full(n, lam), b1_=score,
                    vb_lo=np.full(n, leadvar), vb_fit=np.full(n, 0.5 * leadvar),
                    vr=np.full(n, 0.01))
        vals['lb'] = np.where(regime == 1, vals['qlb'], vals['nlb'])
        vals['ub'] = np.where(regime == 1, vals['qub'], vals['nub'])
        for k in PER:
            rows[f'{k}{j}'] = vals[k]
    return pd.DataFrame(rows)


def write_manifest(d, man):
    json.dump(man, open(d + '/manifest.json', 'w'))
    open(d + '/manifest.sha256', 'w').write(pkgmc.sha(d + '/manifest.json') + '\n')


@pytest.fixture
def run(tmp_path):
    """A synthetic complete run: two cells, two deletion modes, two tasks of five replications."""
    d = str(tmp_path / 'run')
    os.makedirs(d + '/out')
    man = dict(schema='PKGMC-MANIFEST-V1', run='synthetic', run_seed=3,
               deletions=['match', 'observation'], reps_per_task=5, tasks_per_cell=2,
               expect_model='repeat_measurement', expect_threshold=0.05,
               cells=[dict(name='a', group='none'), dict(name='b', errors='skew', group='strong')],
               designs=dict(a=dict(person_years=100), b=dict(person_years=120)))
    man['tasks'] = len(pkgmc.tasks_of(man))
    write_manifest(d, man)
    rng = np.random.default_rng(1)
    for t in pkgmc.tasks_of(man):
        task_frame(t, man, rng).to_csv(f"{d}/out/{t['cell']}__{t['deletion']}__{t['task']}.csv",
                                       index=False)
    return d, man


def edit(d, name, change):
    f = f'{d}/out/{name}'
    x = pd.read_csv(f)
    change(x)
    x.to_csv(f, index=False)


def test_tasks_have_unique_keys_and_shared_seeds_across_deletion_modes(run):
    d, man = run
    tasks = pkgmc.tasks_of(man)
    assert len({(t['cell'], t['deletion'], t['task']) for t in tasks}) == len(tasks) == 8
    seeds = {}
    for t in tasks:
        for r in range(t['first'], t['first'] + t['reps']):
            seeds.setdefault((t['cell'], t['deletion']), []).append((t['seed_base'], r + 1))
    # (cell seed, stream)
    assert (seeds[('a', 'match')] == seeds[('a', 'observation')]
            == [(300000, s) for s in range(1, 11)])
    assert not set(seeds[('a', 'match')]) & set(seeds[('b', 'match')])


def test_a_complete_run_has_no_problems(run):
    d, man = run
    problems, frame = pkgmc.check(d, quiet=True)
    assert problems == [] and len(frame) == 40


@pytest.mark.parametrize('damage, message', [
    (lambda d: os.remove(d + '/out/a__match__1.csv'), 'missing a__match__1.csv'),
    (lambda d: open(d + '/out/a__match__1.csv.tmp', 'w').write('x'), 'partial output'),
    (lambda d: open(d + '/out/c__match__0.csv', 'w').write('x'), 'unexpected output'),
    (lambda d: open(d + '/out/a__match__0.csv', 'w').write(''), 'unreadable'),
    (lambda d: edit(d, 'a__match__0.csv', lambda x: x.drop(index=4, inplace=True)),
     'replications are not 0..4'),
    (lambda d: edit(d, 'a__match__1.csv', lambda x: x.__setitem__('rep', x['rep'] - 5)),
     'replications are not 5..9'),
    (lambda d: edit(d, 'a__match__0.csv', lambda x: x.__setitem__('deletion', 'observation')),
     'keys disagree'),
    (lambda d: edit(d, 'a__match__0.csv', lambda x: x.__setitem__('seed', x['seed'] + 1)),
     'seeds disagree'),
    (lambda d: edit(d, 'a__match__1.csv', lambda x: x.__setitem__('stream', x['stream'] - 5)),
     'seeds disagree'),
    (lambda d: edit(d, 'a__match__0.csv', lambda x: x.__setitem__('truth2', np.nan)),
     'missing truth'),
    (lambda d: edit(d, 'b__match__0.csv', lambda x: x.__setitem__('model', 'leaveout_residual')),
     'did not use the model'),
    (lambda d: edit(d, 'b__match__0.csv', lambda x: x.__setitem__('n_retained', 119)),
     'not the whole design'),
    (lambda d: edit(d, 'b__match__0.csv', lambda x: x.__setitem__('threshold', 0.1)),
     'did not use the rule threshold'),
    (lambda d: open(d + '/manifest.json', 'a').write(' '), 'the manifest changed'),
])
def test_inventory_reports_damage_and_aggregation_refuses_it(run, damage, message):
    d, man = run
    damage(d)
    problems, frame = pkgmc.check(d, quiet=True)
    assert any(message in p for p in problems), problems
    with pytest.raises(SystemExit) as stop:
        aggregate.main(d)
    assert stop.value.code == 2


def fail_first(x):
    x.loc[0, 'rc'] = 498
    x.loc[0, 'model'] = np.nan
    x.loc[0, 'n_retained'] = np.nan
    for c in x.columns:
        if c[:-1] in PER and c[:-1] != 'truth':
            x.loc[0, c] = np.nan


def test_failed_fits_stay_in_the_denominator(run):
    d, man = run
    edit(d, 'a__match__0.csv', fail_first)
    out = aggregate.main(d)
    assert (out['complete'] and len(out['fit_failures']) == 1
            and out['fit_failures'][0]['rc'] == 498 and out['fit_failures'][0]['rep'] == 0)
    row = [c for c in out['cells']
           if c['cell'] == 'a' and c['deletion'] == 'match' and c['component'] == 'worker'][0]
    other = [c for c in out['cells'] if c['cell'] == 'a' and c['deletion'] == 'observation'
             and c['component'] == 'worker'][0]
    assert (row['attempted'] == 10 and row['fit_failed'] == 1 and row['no_interval'] == 1
            and row['coverage'] <= 0.9)
    assert other['fit_failed'] == 0 and other['no_interval'] == 0


def test_coverage_and_the_recomputed_package_rule(run):
    d, man = run
    out = aggregate.main(d)
    assert out['package_rule_mismatches'] == []
    frame = pkgmc.check(d, quiet=True)[1]
    one = frame[(frame['cell'] == 'a') & (frame['deletion'] == 'match')]
    row = [c for c in out['cells']
           if c['cell'] == 'a' and c['deletion'] == 'match' and c['component'] == 'firm'][0]
    assert (row['coverage'] == pytest.approx(np.mean(np.abs(one['b2'] - 1.0) <= 0.3))
            and row['q1_used'] == 1.0)
    assert row['coverage_normal'] == pytest.approx(np.mean(np.abs(one['b2'] - 1.0) <= 0.2))
    assert (row['share_median'] == pytest.approx(0.2) and row['length'] == pytest.approx(0.6)
            and row['length_normal'] == pytest.approx(0.4))
    total = [c for c in out['cells']
             if c['cell'] == 'a' and c['deletion'] == 'match' and c['component'] == 'sum'][0]
    assert total['q1_used'] == 0.0 and total['length'] == pytest.approx(0.4)  # the sum stays normal
    rules = {r['rule']: r for r in out['rules']}
    assert (rules['always_q1']['mean_q1_used'] == 1.0
            and rules['always_normal']['mean_q1_used'] == 0.0)
    assert rules['share>0.10']['mean_q1_used'] == 1.0 and rules['share>0.50']['mean_q1_used'] == 0.0
    assert rules['kappa>0.3']['mean_q1_used'] == 1.0 and rules['kappa>0.5']['mean_q1_used'] == 0.0
    assert rules['always_q1']['median_length_ratio'] == pytest.approx(1.5)
    assert (rules['always_normal']['none_posted'] == 1.0
            and rules['always_q1']['none_posted'] == 0.0
            and rules['share>0.05']['none_q1_used'] == 1.0)
    assert (rules['share>0.10&kappa>0.3']['mean_q1_used'] == 1.0
            and rules['share>0.10&kappa>0.3']['none_posted'] == 0.0)


def test_a_recorded_choice_that_contradicts_the_rule_is_reported(run):
    d, man = run

    def flip(x):
        x.loc[2, 'regime1'] = 0.0
        x.loc[2, 'lb1'] = x.loc[2, 'nlb1']
        x.loc[2, 'ub1'] = x.loc[2, 'nub1']
    edit(d, 'b__observation__1.csv', flip)
    out = aggregate.main(d)
    assert out['package_rule_mismatches'] == [
        dict(cell='b', deletion='observation', component='worker', replications=[7])]


def test_rule_logic_without_a_normal_or_a_q1_interval():
    n = 4
    nan = np.nan
    p = dict(b=np.ones(n), nlb=np.array([0.8, nan, 0.8, nan]), nub=np.array([1.2, nan, 1.2, nan]),
             qlb=np.array([0.7, 0.7, nan, nan]), qub=np.array([1.3, 1.3, nan, nan]),
             normal_ok=np.array([True, False, True, False]),
             q1_ok=np.array([True, True, False, False]))
    lo, hi, regime = aggregate.apply_rule(p, np.array([False, False, True, True]), 1)
    # no normal: q = 1; triggered without q = 1: normal; neither: none
    assert regime[0] == 0 and regime[1] == 1 and regime[2] == 0 and np.isnan(regime[3])
    assert lo[1] == 0.7 and hi[2] == 1.2 and np.isnan(lo[3])
    assert not aggregate.cover(lo, hi, np.ones(n))[3]
    # the sum never takes the q = 1 interval
    lo, hi, regime = aggregate.apply_rule(p, np.ones(n, bool), 4)
    assert regime[0] == 0 and np.isnan(regime[1])


TOL = dict(min_success_rate=0.995, min_coverage=0.6, min_mean_coverage=0.6,
           max_length_ratio_none=1.6, min_posted_none=0.0)


def with_tolerances(d, **change):
    man = json.load(open(d + '/manifest.json'))
    man['tolerances'] = dict(TOL, **change)
    write_manifest(d, man)


def test_confirmation_needs_frozen_tolerances_and_passes_a_clean_run(run):
    d, man = run
    with pytest.raises(SystemExit):  # no tolerances in the manifest
        confirm.verdict(d)
    with_tolerances(d)
    assert confirm.main(d) == 0 and json.load(open(d + '/confirmation.json'))['verdict'] == 'PASS'
    with pytest.raises(SystemExit):  # a verdict is not overwritten
        confirm.main(d)


@pytest.mark.parametrize('change, criterion, cases', [
    # every cell, deletion mode and component is listed, not the first one
    (dict(min_coverage=1.01), 'min_coverage', 16),
    # group "none" only (cell a), three components with the q = 1 interval, two modes
    (dict(max_length_ratio_none=1.4), 'max_length_ratio_none', 6),
    (dict(min_posted_none=0.5), 'min_posted_none', 2),
    (dict(min_mean_coverage=1.01), 'min_mean_coverage', 1),
])
def test_confirmation_lists_every_failing_case(run, change, criterion, cases):
    d, man = run
    for t in pkgmc.tasks_of(man):
        edit(d, f"{t['cell']}__{t['deletion']}__{t['task']}.csv",
             lambda x: x.__setitem__('joint_posted', 0))
    with_tolerances(d, **change)
    out = confirm.verdict(d)
    assert out['verdict'] == 'FAIL'
    assert [f['criterion'] for f in out['failures']] == [criterion] * cases


def test_confirmation_fails_on_failed_fits_and_on_an_incomplete_run(run):
    d, man = run
    with_tolerances(d)
    edit(d, 'a__match__0.csv', fail_first)
    out = confirm.verdict(d)
    assert out['verdict'] == 'FAIL' and out['fit_failures'] == 1
    assert ([(f['criterion'], f['cell'], f['deletion']) for f in out['failures']]
            == [('min_success_rate', 'a', 'match')])
    os.remove(d + '/out/b__match__1.csv')
    out = confirm.verdict(d)
    assert out['verdict'] == 'FAIL' and any(
        f['criterion'] == 'inventory' and 'missing b__match__1.csv' in f['detail']
        for f in out['failures'])


def test_the_recomputed_rule_uses_the_threshold_each_fit_reports(run):
    d, man = run
    # The synthetic share is 0.2: a fit that reports a threshold of 0.5 and still chose q = 1
    # contradicts its own rule.
    man2 = json.load(open(d + '/manifest.json'))
    del man2['expect_threshold']
    write_manifest(d, man2)
    edit(d, 'a__match__0.csv', lambda x: x.__setitem__('threshold', 0.5))
    out = aggregate.main(d)
    assert out['package_threshold'] == [0.05, 0.5]
    assert ([(m['cell'], m['deletion'], m['component'], m['replications'])
             for m in out['package_rule_mismatches']]
            == [('a', 'match', c, [0, 1, 2, 3, 4]) for c in ('worker', 'firm', 'cov')])


def fall_back(x, row=0):
    """Turn one fit into the default's fallback: point estimates, no model name, no intervals, a
    reason."""
    x['model'] = x['model'].astype(object)
    x['reason'] = x['reason'].astype(object)
    x.loc[row, 'model'] = np.nan
    x.loc[row, 'status'] = 'KSS_POINT_ESTIMATES_ONLY'
    x.loc[row, 'reason'] = 'the calculation that carried them stopped (JLA_CONSTRAINT_FAILED)'
    x.loc[row, 'threshold'] = np.nan
    for c in x.columns:
        if c[:-1] in PER and c[:-1] not in ('truth', 'b'):
            x.loc[row, c] = np.nan


def test_a_fallback_to_point_estimates_is_a_replication_without_an_interval(run):
    d, man = run
    with_tolerances(d, min_success_rate=0.85)
    edit(d, 'a__match__0.csv', fall_back)
    problems, frame = pkgmc.check(d, quiet=True)
    assert problems == [] and len(frame) == 40  # not an inventory problem
    out = aggregate.main(d)
    assert ([(f['cell'], f['deletion'], f['rep'], f['reason'][:15]) for f in out['fallbacks']]
            == [('a', 'match', 0, 'the calculation')])
    row = [c for c in out['cells']
           if c['cell'] == 'a' and c['deletion'] == 'match' and c['component'] == 'firm'][0]
    assert row['no_interval'] == 1 and row['coverage'] <= 0.9  # counted as not covered
    v = confirm.verdict(d)
    # 90% of the fits of that case have intervals: above 85%
    assert v['verdict'] == 'PASS' and len(v['fallbacks']) == 1
    with_tolerances(d, min_success_rate=0.95)
    v = confirm.verdict(d)
    assert (v['verdict'] == 'FAIL'
            and [(f['criterion'], f['cell'], f['deletion'], f['value']) for f in v['failures']]
            == [('min_success_rate', 'a', 'match', 0.9)])


def test_a_fit_with_another_model_or_a_half_empty_fallback_is_still_rejected(run):
    d, man = run

    def half(x):  # claims to be a fallback but carries an interval
        fall_back(x)
        x.loc[0, 'lb2'] = 0.5
    edit(d, 'a__match__0.csv', half)
    assert any('did not use the model' in p for p in pkgmc.check(d, quiet=True)[0])


# Designs of the campaign runs and the sha256 their manifests record for the exported CSV: the
# generator must reproduce them bit for bit (all four variance kinds, a gap, a control).
RECORDED = [
    ('dev1', dict(name='J100_bnone_g0_flat_skew', nJ=100, nW=12000, n_bridge=None, design_seed=12,
                  gap=0.0, variance='flat'),
     'c652ca529fe20ffa135765f6b912d6ff548aa4cc4dedbb2e48c68c4c1887e24d', 47917),
    ('dev1', dict(name='J100_b30_g3_profile', nJ=100, nW=12000, n_bridge=30, design_seed=12,
                  gap=3.0, variance='profile'),
     '46b5da89057d702dd7109c697fa633852d69f87016b44f869fe4733a83d49578', 48195),
    ('conf2', dict(name='c_J400_b1000_g3_movers', nJ=400, nW=12000, n_bridge=1000, design_seed=36,
                   gap=3.0, variance='movers'),
     'db3f2a5488c266d4a8074e40b8562a0c91fca1b146ffeb59a2959d9657e7ced4', 48469),
    ('conf2', dict(name='c_J100_b100_g0_bridge', nJ=100, nW=12000, n_bridge=100, design_seed=43,
                   gap=0.0, variance='bridge'),
     '7bf52edd7f417fa1eac3fbc3d57fc91af8ada081b8091d292b4916531b9ec7b6', 48283),
    ('dev2_controls', dict(name='x_J100_b30_g0_profile', nJ=100, nW=12000, n_bridge=30,
                           design_seed=64, gap=0.0, variance='profile', control=True),
     'dd5fa4b2375e36ca95204bfef4bc4bd936e00a7ebf5dd95ab20cda56d271546b', 47926),
]


@pytest.mark.parametrize('run_name, cell, digest, person_years', RECORDED,
                         ids=[r[1]['name'] for r in RECORDED])
def test_the_generator_reproduces_the_recorded_designs(tmp_path, run_name, cell, digest,
                                                       person_years):
    path = tmp_path / 'design.csv'
    info = designs.write_design(cell, str(path))
    assert hashlib.sha256(path.read_bytes()).hexdigest() == digest
    assert info['person_years'] == person_years


SMALL = dict(name='s', nJ=40, nW=1500, n_bridge=10, design_seed=12, gap=0.0, variance='profile')


def test_a_design_with_a_control_carries_x_and_its_contribution(tmp_path):
    designs.write_design(dict(SMALL, control=True), str(tmp_path / 'x.csv'))
    designs.write_design(SMALL, str(tmp_path / 'plain.csv'))
    # %.17g round-trips every double, so xb is exactly 0.3 x once both are read back exactly
    x = pd.read_csv(tmp_path / 'x.csv', float_precision='round_trip')
    plain = pd.read_csv(tmp_path / 'plain.csv', float_precision='round_trip')
    assert list(x.columns) == ['worker', 'firm', 'year', 'alpha', 'psi', 'sd', 'x', 'xb']
    assert list(plain.columns) == ['worker', 'firm', 'year', 'alpha', 'psi', 'sd']
    assert np.array_equal(x['xb'].to_numpy(), 0.3 * x['x'].to_numpy())
    assert x[list(plain.columns)].equals(plain)  # the control leaves the design itself unchanged
    assert x.groupby('worker')['x'].nunique().min() > 1  # it varies within every worker


def fake_package(root):
    os.makedirs(root)
    for f in ('fevc_rust_macos_arm64.plugin', 'fevc.ado', 'fevc_residual_se.mata',
              'fevc__residual_se_post.ado', 'fevc__component_model_route.ado'):
        open(f'{root}/{f}', 'w').write(f)


def small_spec(tmp_path, **change):
    pkg = str(tmp_path / 'pkg')
    if not os.path.exists(pkg):
        fake_package(pkg)
    spec = dict(run='small', run_seed=1, pkgroot=pkg, source_commit='0000000',
                deletions=['match', 'observation'], reps_per_task=3, tasks_per_cell=2, threads=1,
                options='none', expect_model='repeat_measurement',
                cells=[dict(SMALL, name='s_profile', group='none')], **change)
    path = tmp_path / f"spec_{len(list(tmp_path.glob('spec_*')))}.json"
    json.dump(spec, open(path, 'w'))
    return str(path)


def test_make_needs_no_stata_and_copies_a_realised_regime_verbatim(tmp_path, monkeypatch):
    monkeypatch.delenv('FEVC_STATA', raising=False)
    d = str(tmp_path / 'run')
    pkgmc.make(d, small_spec(tmp_path))
    man = json.load(open(d + '/manifest.json'))
    assert open(d + '/manifest.sha256').read().strip() == pkgmc.sha(d + '/manifest.json')
    assert man['schema'] == 'PKGMC-MANIFEST-V1' and man['tasks'] == 4
    rec = man['designs']['s_profile']
    assert 'realised' not in rec
    assert rec['sha256'] == pkgmc.sha(d + '/designs/s_profile.csv')
    assert set(man['hashes']['design_code']) == {'designs.py'}
    with pytest.raises(SystemExit, match='refusing to overwrite'):
        pkgmc.make(d, small_spec(tmp_path))
    realised = {'person': {'kappa': 0.25, 'share': 0.01, 'note': 'from elsewhere'}}
    spec = json.load(open(small_spec(tmp_path)))
    spec['cells'][0]['realised'] = realised
    json.dump(spec, open(tmp_path / 'with_realised.json', 'w'))
    pkgmc.make(str(tmp_path / 'run2'), str(tmp_path / 'with_realised.json'))
    man2 = json.load(open(tmp_path / 'run2/manifest.json'))
    assert man2['designs']['s_profile']['realised'] == realised


def test_run_needs_a_stata_executable(tmp_path, monkeypatch):
    monkeypatch.delenv('FEVC_STATA', raising=False)
    d = str(tmp_path / 'run')
    pkgmc.make(d, small_spec(tmp_path))
    with pytest.raises(SystemExit, match='--stata PATH or set FEVC_STATA'):
        pkgmc.run(d, 1)
    with pytest.raises(SystemExit, match='--stata PATH or set FEVC_STATA'):
        pkgmc.main(['run', d, '1'])
    missing = str(tmp_path / 'no_stata')
    with pytest.raises(SystemExit, match='does not exist or is not executable'):
        pkgmc.main(['run', d, '1', '--stata', missing])
    monkeypatch.setenv('FEVC_STATA', missing)
    with pytest.raises(SystemExit, match='does not exist or is not executable'):
        pkgmc.run(d, 1)
    assert os.listdir(d + '/out') == []
