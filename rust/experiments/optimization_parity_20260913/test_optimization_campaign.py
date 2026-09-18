"""Unique module name so this suite can run alongside historical campaigns."""
from pathlib import Path

import numpy as np
import pytest

from campaign import PROFILES, development_cells, paper_cells, protocol
from development_input import STAYER_PROFILES, build as build_development_input
from generate import PATTERNS, connectivity, generate, graph, half_degrees


def test_campaign_is_bounded_and_still_gated():
    value = protocol()
    assert value['status'] == 'GATED_NOT_SUBMITTED'
    assert value['candidate_source'] is None and value['input_hashes'] is None
    for stage, measured, warmups in [('development', 96, 32), ('paper', 156, 52)]:
        attempts = value[stage]
        assert sum(not call['warmup'] for call in attempts) == measured
        assert sum(call['warmup'] for call in attempts) == warmups
    assert len(development_cells()) == 16
    assert len(paper_cells()) == 26
    cells = paper_cells()
    assert len({(c['graph'], c['rows'], c['threads'], c['deletion']) for c in cells}) == 26
    assert all(c['rows'] <= 1_600_000 for c in cells)
    assert not any(c['graph'] == 'degree4_well_mixed' for c in cells)
    assert [c['threads'] for c in cells if c['block'] == 'veneto_private_local'] == [1, 8]


@pytest.mark.parametrize('pattern', PATTERNS)
@pytest.mark.parametrize('rows', [8_000, 100_000])
def test_exact_graph_dimensions_and_nonvanishing_bottleneck(pattern, rows):
    assignment, offsets, meta = graph(rows, pattern)
    mean_degree = 4 if pattern.startswith('degree4') else 5
    assert len(assignment) == rows
    assert meta['workers'] == rows // mean_degree
    assert meta['firms'] == rows // 100
    assert offsets[-1] == rows
    assert np.all(np.bincount(assignment) == 100)
    assert meta['mean_worker_degree'] == mean_degree
    if pattern.startswith('mixed'):
        assert set(meta['worker_degree_histogram']) == set(map(str, range(2, 9)))
    else:
        assert meta['worker_degree_histogram'] == {str(mean_degree): rows // mean_degree}
    diagnostic = connectivity(assignment, offsets, meta['firms'])
    assert diagnostic['components'] == 1
    if meta['mobility'] == 'segmented':
        assert meta['crossing_share'] == .01
        assert .001 < diagnostic['fixed_half_cut_conductance'] < .02
    else:
        assert diagnostic['fixed_half_cut_conductance'] > .2


def test_mixed_rounding_is_balanced_for_every_remainder():
    for workers in range(7, 35):
        degree = half_degrees(workers, 'mixed')
        assert len(degree) == workers and degree.sum() == workers * 5


def test_generated_inputs_are_write_once_and_receipted(tmp_path):
    path = tmp_path / 'smoke.csv'
    meta = generate(path, 8_000, 'degree4_bottleneck')
    assert meta['rows'] == 8_000 and meta['components'] == 1
    assert len(meta['sha256']) == 64 and len(meta['generator_sha256']) == 64
    with pytest.raises(ValueError, match='already exists'):
        generate(path, 8_000, 'degree4_bottleneck')
    with pytest.raises(ValueError, match='unregistered'):
        graph(6_400_000, 'degree4_bottleneck')


@pytest.mark.parametrize('profile', ['mover_match_both', 'pooled_stayer_match',
                                     'observation_projection'])
def test_development_inputs_are_exact_and_stayer_specific(tmp_path, profile):
    rows = 8_000 if PROFILES.index(profile) >= 5 else 100_000
    receipt = build_development_input(tmp_path / f'{profile}.csv', profile, rows)
    assert receipt['rows'] == rows and len(receipt['sha256']) == 64
    assert receipt['repeats_per_match'] == 4
    data = np.genfromtxt(tmp_path / f'{profile}.csv', delimiter=',', names=True)
    workers = data['worker'].astype(np.int64)
    firms = data['firm'].astype(np.int64)
    if profile not in STAYER_PROFILES:
        distinct_degrees = {
            len(set(firms[workers == worker])) for worker in np.unique(workers)
        }
        assert distinct_degrees == set(range(2, 9))
        _, match_counts = np.unique(data['match'].astype(np.int64), return_counts=True)
        assert set(match_counts) == {4}
    if profile in STAYER_PROFILES:
        assert receipt['converted_stayer_workers'] > 0
        assert receipt['actual_stayer_workers'] == receipt['converted_stayer_workers']
    else:
        assert receipt['converted_stayer_workers'] == 0
        assert receipt['actual_stayer_workers'] == 0


def test_development_launchers_freeze_resources_and_call_accounting():
    root = Path(__file__).parent
    cell = (root / 'run_development_cell.sge').read_text()
    smoke = (root / 'run_development_smoke.sge').read_text()
    submit = (root / 'submit_development.sh').read_text()
    stata = (root / 'development_stata.do').read_text()
    assert '#$ -pe omp 14' in cell and '#$ -l mem_per_core=3G' in cell
    assert '#$ -l h_rt=00:45:00' in cell and 'test "${call_count}" = 8' in cell
    assert '#$ -pe omp 4' in smoke and '#$ -l h_rt=00:25:00' in smoke
    assert 'deliberate-failure' in smoke and 'development_tasks\\t16' in submit
    assert 'timed_calls\\t96' in submit and 'warmups\\t32' in submit
    assert 'probes(200)' in stata and 'tolerance(' not in stata
    assert 'memory_gib(' not in stata and 'set processors `threads\'' in stata
    assert stata.count('preconditioner(cmg)') == 5
    assert stata.count('preconditioner(diagonal)') == 3
    assert 'preconditioner(auto)' not in stata
