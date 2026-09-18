"""Finite, pre-result campaign definition. This module submits no jobs."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from generate import PATTERNS, sha

SEEDS = (104729, 8675309, 20260819)
PROFILES = (
    'mover_match_both', 'pooled_stayer_match', 'observation_stayers',
    'controlled_weighted_match', 'controlled_weighted_observation',
    'observation_projection', 'structured_observation_inference',
    'fixed_offset_match_inference',
)


def paper_cells():
    cells = []
    for graph in PATTERNS:
        for rows in (100_000, 400_000, 1_600_000):
            cells.append(dict(block='size', graph=graph, rows=rows, threads=28, deletion='match'))
    for graph in ('mixed_well_mixed', 'mixed_segmented'):
        for threads in (1, 7):
            cells.append(dict(block='scaling', graph=graph, rows=400_000, threads=threads, deletion='match'))
    for graph in PATTERNS:
        cells.append(dict(block='observation', graph=graph, rows=400_000, threads=28, deletion='observation'))
    for threads in (1, 8):
        cells.append(dict(block='veneto_private_local', graph=None, rows=1_319_994, threads=threads, deletion='observation'))
    return cells


def development_cells():
    return [dict(profile=profile, threads=threads,
                 rows=8_000 if index >= 5 else 100_000)
            for index, profile in enumerate(PROFILES) for threads in (1, 7)]


def calls(cells, variants):
    result = []
    for cell_id, cell in enumerate(cells):
        for variant in variants:
            result.append(dict(cell=cell_id, configuration=cell, variant=variant,
                               repetition=None, warmup=True, seed=SEEDS[0]))
        for repetition, seed in enumerate(SEEDS):
            offset = (cell_id + repetition) % len(variants)
            order = variants[offset:] + variants[:offset]
            for position, variant in enumerate(order):
                result.append(dict(cell=cell_id, configuration=cell, variant=variant,
                                   repetition=repetition + 1, warmup=False,
                                   seed=seed, position=position))
    return result


def protocol():
    return dict(
        schema='FEVC-PARITY-PAPER-CAMPAIGN-V1', status='GATED_NOT_SUBMITTED',
        baseline_manifest_sha256='4bdba22c0e6da99b3fd597aabab8df4d01307d01704951f3d5d2c089885cd2ad',
        candidate_source=None, candidate_binaries=None, input_hashes=None,
        source_and_input_freeze_required_before_timing=True,
        graph_generator_sha256=sha(Path(__file__).with_name('generate.py')),
        manifest_builder_sha256=sha(Path(__file__)),
        probes=200, fevc_tolerance_override=None, population='both',
        matlab_source='8b957ffe', seeds=SEEDS,
        primary_clock='after common import, before package initialization/pool creation, through available results',
        additional_clocks=['pool startup', 'estimator only', 'whole process'],
        rng='record actual Matlab client and worker streams; no assumed independent draws',
        development_gate=dict(point_geomean_improvement_min=.03,
                              profile_median_regression_max=.05,
                              all_numerical_native_and_route_checks_required=True),
        resources=dict(smoke=dict(slots=4, gib_per_slot=3, minutes=25),
                       development=dict(slots=14, gib_per_slot=3, minutes=45),
                       paper=dict(slots=28, gib_per_slot=3, minutes=45),
                       queue=None, host=None, project='welfgr'),
        physical_rss_guards_gib=dict(fevc_through_400k=20, fevc_1_6m=45,
                                    matlab_synthetic=72, private_veneto_local=96),
        new_synthetic_bundle_and_outputs_gib_max=20,
        operational_repairs_max=1, automatic_resource_increase=False,
        scientific_failure_stops_advancement=True,
        veneto=dict(location='existing private local environment only',
                    preserve_all_rows=True, published_count_discrepancy=22,
                    historical_numerical_failure_remains_failure=True,
                    publish='approved aggregate outputs only'),
        development=calls(development_cells(), ['baseline', 'candidate']),
        paper=calls(paper_cells(), ['fevc', 'matlab']),
        exclusions=['fresh 6.4m runs', 'Figure 4 campaign', 'fusion promotion',
                    'commit', 'push', 'PLUS replacement', 'Windows', 'upstream edits'],
    )


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    with args.output.open('x') as stream:
        json.dump(protocol(), stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
