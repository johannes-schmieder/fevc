"""Verdict of a confirmation run against the tolerances frozen in its manifest.

  confirm.py <run dir>

The manifest must carry "tolerances" (written by `pkgmc.py make` from the spec, before any task
ran):
  min_success_rate   share of attempted replications of a cell and deletion mode with a fit and a
                     reported interval for all four components (a fit that fell back to point
                     estimates has none)
  min_coverage       coverage of the reported interval, every cell, deletion mode and component
                     (the sum included); a replication without a fit or without an interval counts
                     as not covered
  min_mean_coverage  mean of those coverages over all cases
  max_length_ratio_none   median length of the reported interval over that of the normal
                     interval, in the designs of group "none" (no bottleneck), every component
  min_posted_none    share of replications in which the joint covariance matrix is posted, in the
                     designs of group "none", every cell and deletion mode
The run must be complete (inventory without problems) and the package's rule, recomputed from the
recorded pieces, must agree with the recorded choice in every replication. Every criterion is
evaluated on every case and all failing cases are listed; nothing is excluded after the fact. Writes
<run dir>/confirmation.json (refuses to overwrite) and exits 0 for PASS, 1 for FAIL.
"""

import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import aggregate  # noqa: E402
from pkgmc import check, sha  # noqa: E402

KEYS = ('min_success_rate', 'min_coverage', 'min_mean_coverage', 'max_length_ratio_none',
        'min_posted_none')


def verdict(run_dir):
    run_dir = os.path.abspath(run_dir)
    man = json.load(open(run_dir + '/manifest.json'))
    tol = man.get('tolerances')
    if not isinstance(tol, dict) or set(tol) != set(KEYS):
        raise SystemExit('the manifest carries no complete set of tolerances')
    problems, frame = check(run_dir, quiet=True)
    failures = []
    if problems:
        failures += [dict(criterion='inventory', detail=p) for p in problems]
    out = dict(schema='PKGMC-CONFIRMATION-V1', run=man['run'],
               manifest_sha256=sha(run_dir + '/manifest.json'), tolerances=tol,
               inventory_problems=len(problems))
    if len(frame):
        cells, rule_table, by_rule, mismatches = aggregate.summarise(frame, man)
        failures += [dict(criterion='rule_recomputation', **m) for m in mismatches]
        groups = {c['name']: c.get('group', '') for c in man['cells']}
        for (cell, deletion), d in frame.groupby(['cell', 'deletion'], sort=False):
            ok = (d['rc'] == 0)
            for j in range(1, 5):
                ok &= np.isfinite(d[f'lb{j}']) & np.isfinite(d[f'ub{j}'])
            if ok.mean() < tol['min_success_rate']:
                failures.append(dict(criterion='min_success_rate', cell=cell, deletion=deletion,
                                     value=float(ok.mean()), attempted=int(len(d))))
            if groups.get(cell) == 'none':
                posted = float(((d['joint_posted'] == 1) & (d['rc'] == 0)).mean())
                if posted < tol['min_posted_none']:
                    failures.append(dict(criterion='min_posted_none', cell=cell, deletion=deletion,
                                         value=posted))
        for r in cells.to_dict('records'):
            if not r['coverage'] >= tol['min_coverage']:
                failures.append(dict(criterion='min_coverage', cell=r['cell'],
                                     deletion=r['deletion'], component=r['component'],
                                     value=r['coverage'], attempted=r['attempted']))
            if r['group'] == 'none':
                ratio = r['length'] / r['length_normal'] if r['length_normal'] > 0 else float('nan')
                if not ratio <= tol['max_length_ratio_none']:
                    failures.append(dict(criterion='max_length_ratio_none', cell=r['cell'],
                                         deletion=r['deletion'], component=r['component'],
                                         value=ratio))
        mean_coverage = float(cells['coverage'].mean())
        if not mean_coverage >= tol['min_mean_coverage']:
            failures.append(dict(criterion='min_mean_coverage', value=mean_coverage))
        fb = frame[(frame['rc'] == 0) & frame['model'].isna()]
        out['fallbacks'] = fb[[c for c in ('cell', 'deletion', 'rep', 'seed', 'stream', 'status',
                                           'reason') if c in fb]].to_dict('records')
        out.update(replications=int(len(frame)), fit_failures=int((frame['rc'] != 0).sum()),
                   cases=int(len(cells)), mean_coverage=mean_coverage,
                   min_coverage_observed=float(cells['coverage'].min()),
                   coverage_by_case=cells[['cell', 'group', 'deletion', 'component', 'attempted',
                                           'coverage', 'coverage_normal', 'coverage_q1',
                                           'q1_used']].to_dict('records'))
    else:
        failures.append(dict(criterion='inventory', detail='no complete task output'))
    out['failures'] = failures
    out['verdict'] = 'PASS' if not failures else 'FAIL'
    return out


def main(run_dir):
    target = os.path.abspath(run_dir) + '/confirmation.json'
    if os.path.exists(target):
        raise SystemExit(f'{target} exists; refusing to overwrite')
    out = verdict(run_dir)
    tmp = target + '.tmp'
    json.dump(out, open(tmp, 'w'), indent=1)
    os.replace(tmp, target)
    print('confirmation', out['run'], out['verdict'], 'failing cases', len(out['failures']))
    for f in out['failures'][:50]:
        print('  ', json.dumps(f))
    return 0 if out['verdict'] == 'PASS' else 1


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    raise SystemExit(main(sys.argv[1]))
