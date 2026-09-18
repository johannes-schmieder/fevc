"""Approved finite optimization screen. Defines calls; never submits jobs."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

BASELINE_NATIVE_SHA256 = '6d8e86aafbe1224e29217ce4565a7c91c22fcc60583bd97b2805b910298ae18d'
SEEDS = (104729, 8675309, 20260819, 32452843, 49979687)
GRAPHS = ('degree4_bottleneck', 'degree5_well_mixed', 'degree5_segmented',
          'mixed_well_mixed', 'mixed_segmented')
PROFILES = {
    'mover_match_both': dict(rows=100_000, deletion='match', controls=0, weights=False),
    'pooled_stayer_match': dict(rows=100_000, deletion='match', controls=0, weights=False),
    'observation_stayers': dict(rows=100_000, deletion='observation', controls=0, weights=False),
    'controlled_weighted_match': dict(rows=100_000, deletion='match', controls=2, weights=True),
    'controlled_weighted_observation': dict(rows=100_000, deletion='observation', controls=2, weights=True),
    'observation_projection': dict(rows=8_000, deletion='observation', controls=1, weights=False),
    'structured_observation_inference': dict(rows=8_000, deletion='observation', controls=1, weights=False),
    'fixed_offset_match_inference': dict(rows=8_000, deletion='match', controls=1, weights=True),
    'mover_observation_both': dict(rows=100_000, deletion='observation', controls=0, weights=False),
    'match_projection': dict(rows=8_000, deletion='match', controls=1, weights=False),
    'exact_observation': dict(rows=800, deletion='observation', controls=2, weights=True),
    'exact_match': dict(rows=800, deletion='match', controls=2, weights=True),
}

# Each profile is a complete supported request, not a product of independent
# options. Inference populations must not inherit the point default blindly.
for _name, _profile in PROFILES.items():
    _projection = _name in ('observation_projection', 'match_projection')
    _inference = _name in ('structured_observation_inference', 'fixed_offset_match_inference')
    _exact = _name.startswith('exact_')
    _profile.update(
        family='exact' if _exact else 'projection' if _projection else 'component_inference' if _inference else 'point',
        population='movers' if _inference or _name=='observation_projection' else 'both',
        nuisance='fixedoffset' if _name=='fixed_offset_match_inference' else 'joint',
        algorithm='exact' if _exact else 'jla',
        engine='auto' if _name in ('pooled_stayer_match','observation_stayers','mover_observation_both') or _exact else 'generic',
        preconditioner='auto' if _exact else 'diagonal' if _projection or _inference else 'cmg',
        batch='auto', probes=200, exact_limit=500 if _exact else 2,
        maxiter=10000, tolerance=None, memory_gib=None, memorycheck='warn',
        target_weight=_profile['weights'] or _projection or _inference,
        deletion_id='match' if _profile['deletion']=='match' else None,
        projection=dict(variable='projection',effect='firm',weight='target') if _projection else None,
        inference=dict(request='highrank',model='structured_common',probes=129,gram_probes=513,
            seed=8675309,level=95,spectrum_probes=128,spectrum_iterations=128) if _inference else None,
        fixture=dict(graph='mixed_well_mixed', degree_min=2,degree_max=8,rows_per_match=4,
            stayer_fraction=.05 if _name in ('pooled_stayer_match','observation_stayers') else 0,
            firms=20 if _profile['rows']<=8000 else 250,
            workers=_profile['rows']//20,graph_seed=20260913,outcome_seed=20260913),
    )


def calls(stage, cells, variants_for, repetitions):
    output = []
    for cell_id, configuration in enumerate(cells):
        variants = list(variants_for(configuration))
        for position, variant in enumerate(variants):
            output.append(dict(stage=stage, cell=cell_id, configuration=configuration,
                variant=variant, warmup=True, repetition=0, seed=SEEDS[0], position=position))
        for repetition in range(repetitions):
            offset = (cell_id + repetition) % len(variants)
            rotated = variants[offset:] + variants[:offset]
            for position, variant in enumerate(rotated):
                output.append(dict(stage=stage, cell=cell_id, configuration=configuration,
                    variant=variant, warmup=False, repetition=repetition+1,
                    seed=SEEDS[repetition], position=position))
    return output


def protocol():
    small = [dict(profile=profile, threads=t, **parameters)
             for profile, parameters in PROFILES.items() for t in (1, 4, 7)]
    screen = [dict(graph=g, rows=400_000, threads=t, deletion=d)
              for g in GRAPHS for d in ('observation', 'match') for t in (4, 28)]
    large = [dict(graph=g, rows=1_600_000, threads=28, deletion=d)
             for g in ('degree4_bottleneck', 'mixed_segmented', 'degree5_well_mixed')
             for d in ('observation', 'match')]
    private = [dict(graph='private_veneto', rows=1_319_994, threads=4, deletion=d)
               for d in ('observation', 'match')]
    stages = {
        'supported_paths': calls('supported_paths', small, lambda _: ('baseline', 'candidate'), 3),
        'screen_400k': calls('screen_400k', screen, lambda c: ('baseline', 'candidate', 'matlab')
                             if c['threads'] == 28 else ('baseline', 'candidate'), 3),
        'validation_1_6m': calls('validation_1_6m', large, lambda _: ('baseline', 'candidate', 'matlab'), 5),
        'private_veneto': calls('private_veneto', private, lambda _: ('baseline', 'candidate'), 3),
    }
    return dict(
        schema='FEVC-COMPLETE-PIPELINE-CAMPAIGN-V1', status='GATED_NOT_SUBMITTED',
        baseline_native_sha256=BASELINE_NATIVE_SHA256,
        candidate_native_sha256=None, candidate_binary_sha256=None, input_hashes=None,
        gates_required=['source-bound Rust and native qualification',
                        'scheduler/application/output-validated compute-node smoke',
                        'frozen sources, inputs, command options and expected inventory',
                        'previous stage passed corrected-target, residual, lifecycle and timing gates'],
        point_probes=200, tolerance_override=None, point_population_default='both',
        population='profile-specific; inference restrictions unchanged',
        inference=dict(probes=129, gram_probes=513, spectrum_probes=128, spectrum_iterations=128),
        small_exact=dict(rows=800, workers=40, firms=20, degree='mixed 2–8', rows_per_match=4,
                         controls=2, frequency_weights=True, target_weights=True,
                         nuisance='joint', native_additive_capability_required=True),
        execution='Each cell sequentially on one host; rotated measured order; separate per-variant warm-ups',
        clocks=['complete command including package/pool initialization', 'estimator only', 'whole process'],
        acceptance=dict(corrected_targets='fevc/docs/development_acceptance_v1.json',
                        bundle_geomean_improvement_min=.03, median_regression_max=.05,
                        simplicity_tie=.02, observation_400k_improvement_goal=.20,
                        matlab_competitiveness_ratio=1.0, matlab_development_goal=.5,
                        full_residual_and_refinement_gates='unchanged',
                        single_core_diagnostics_are_not_acceptance=True),
        resources=dict(smoke=dict(slots=4, gib_per_slot=3, minutes=25),
                       thread_boundary_smoke=dict(slots=7, gib_per_slot=3, minutes=25),
                       supported_paths=dict(slots=14, gib_per_slot=3, minutes=45),
                       comparisons=dict(slots=28, gib_per_slot=3, minutes=45),
                       private_veneto=dict(slots=28, gib_per_slot=3, minutes=45),
                       project='welfgr', queue=None, host=None,
                       stata_processors='min(4, native threads)',
                       native_threads='verified isolated adapter; not inferred from Stata MP'),
        rss_guards_gib=dict(smoke=10, small_fevc=20, large_or_private_fevc=45, matlab=72),
        scheduler_vmem_is_not_physical_rss=True,
        operational_repairs_max=1, scientific_failure_stops_advancement=True,
        automatic_resource_increase=False,
        privacy='Private Veneto raw data and detail outputs remain in the private work area; aggregate report only',
        exclusions=['6.4m', 'publication campaign', 'paper edits', 'PLUS replacement',
                    'fusion promotion', 'upstream CMG edits', 'commit', 'push', 'branch/worktree'],
        measured_counts={stage: sum(not call['warmup'] for call in values) for stage, values in stages.items()},
        stages=stages,
        definition_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    )


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    with args.output.open('x') as stream:
        json.dump(protocol(), stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
