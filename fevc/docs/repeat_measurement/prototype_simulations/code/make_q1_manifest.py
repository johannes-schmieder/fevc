"""Writes a manifest of the q = 1 simulation. Usage: make_q1_manifest.py <out.json> <run> <stream> <design_seed> <tasks> <reps_per_task> [--bind]
--bind stores the sha256 of the source files (confirmation runs); tasks then refuse to run on other sources."""
import sys, json
from lp_q1sim import source_hashes
out, run, stream, dseed, tasks, reps = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]), int(sys.argv[6])
cells = []
def add(name, b, gap, var, role='acceptance', errors='normal'):
    cells.append(dict(name=name, nJ=100, nW=12000, n_bridge=b, design_seed=dseed, gap=gap, variance=var, errors=errors, role=role))
for b in (10, 30, 100):
    for gi, gap in (('0', 0.0), ('3', 0.3)):
        for var in ('flat', 'profile', 'movers'): add(f'b{b}_g{gi}_{var}', b, gap, var)
for var in ('flat', 'profile', 'movers'): add(f'none_{var}', None, 0.0, var)
for b in (10, 30):
    add(f'b{b}_g3_profile_skew', b, 0.3, 'profile', errors='skew'); add(f'b{b}_g3_bridge', b, 0.3, 'bridge')
for gi, gap in (('0', 0.0), ('3', 0.3)): add(f'b3_g{gi}_profile', 3, gap, 'profile', role='stress')
man = dict(schema='LP-Q1-MANIFEST-V2', run=run, stream=stream, probes=1600, draws=16, reps_per_task=reps, tasks=tasks, cells=cells,
           acceptance=dict(components=['person', 'firm', 'cov'], q1_cover_min=0.92, q1_cover_mean_min=0.94, no_interval_max=0.01, sum_normal_cover_min=0.92,
                           len_ratio_no_bottleneck_max=1.10, failed_replications_max=0))
if '--bind' in sys.argv: man['sources'] = source_hashes()
json.dump(man, open(out, 'w'), indent=1); print(out, len(cells), 'cells', tasks * reps, 'draws per cell')
