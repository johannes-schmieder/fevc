"""Spec of the registered confirmation of the plain default command with controls.
The twenty design shapes are those of conf2 (new design seeds 121-140); every design carries controls. Kinds:
adv  one control correlated with both effects (0.4 alpha + 0.8 psi + 0.25 noise), coefficient 0.3
two  that control and a second one trending with the year (coefficient 0.2)
ben  one control unrelated to the effects
wrk  one control loaded on the worker effect (0.8 alpha + 0.2 psi + 0.4 noise)
Assignment rule, fixed after the development checks dev3-dev7 and before any confirmation draw: the designs are
taken in conf2 order and the kinds rotate adv, two, ben, wrk; in a design whose two blocks differ (gap 3) a kind
loaded on the firm effect (adv, two) is replaced by wrk, ben alternately (development: there the joint estimate's
additional term makes the worker and firm standard errors 8-13% short, limit 1 of INFERENCE.md).
Usage: make_conf3_spec.py <conf2 spec> <pkgroot> <commit> <out.json>"""
import json
import sys

conf2 = json.load(open(sys.argv[1]))
KINDS = {'adv': {}, 'two': {'second': True}, 'ben': {'alpha': 0.0, 'psi': 0.0},
         'wrk': {'alpha': 0.8, 'psi': 0.2, 'noise': 0.4}}
ROTATION = ['adv', 'two', 'ben', 'wrk']
REPLACE = ['wrk', 'ben']
cells, replaced = [], 0
for i, c in enumerate(conf2['cells']):
    kind = ROTATION[i % 4]
    if c['gap'] != 0 and kind in ('adv', 'two'):
        kind = REPLACE[replaced % 2]
        replaced += 1
    d = {k: v for k, v in c.items() if k not in ('name', 'design_seed', 'realised')}
    d['name'] = c['name'].replace('c_', f'k{kind}_', 1)
    d['design_seed'] = 121 + i
    d['control'] = KINDS[kind]
    cells.append(d)
spec = dict(
    run='conf3_controls',
    purpose=('registered confirmation of the default as a user runs it with control variables: the plain fevc '
             'command (joint nuisance, rule share > 0.10) after the first-order term for estimated control '
             'coefficients, on the release-candidate source and its qualified macOS arm64 plugin. The twenty '
             'design shapes, the replications and the tolerances are those of conf2; the design seeds and the run '
             'seed are new. Every design carries controls (kinds and assignment rule in make_conf3_spec.py): one '
             'control correlated with both effects, that control with a second one, one control unrelated to the '
             'effects, one control loaded on the worker effect; controls loaded on the firm effect only in designs '
             'without a block gap. Not covered by design: a control index as large as the effects, error variances '
             'that move with a control inside a cell, a control loaded on the firm effect where the blocks differ.'),
    run_seed=53,
    pkgroot=sys.argv[2],
    source_commit=sys.argv[3],
    deletions=['match', 'observation'],
    reps_per_task=50, tasks_per_cell=20, threads=1, options='none',
    expect_model='repeat_measurement', expect_threshold=0.1,
    tolerances=conf2['tolerances'],
    cells=cells)
json.dump(spec, open(sys.argv[4], 'w'), indent=1)
print(len(cells), 'cells;', len(cells) * 2 * 1000, 'fits; tolerances', spec['tolerances'])
for c in cells:
    print(' ', c['name'], c['group'], c['gap'], c['design_seed'], c['control'])
