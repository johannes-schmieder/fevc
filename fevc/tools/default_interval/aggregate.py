"""Aggregation of a package Monte Carlo run (see pkgmc.py).

  aggregate.py <run dir> [--allow-incomplete]

Refuses a run whose inventory has problems (missing, partial, duplicate or mislabelled task outputs,
a fit on a pruned sample or with another standard-error model) unless --allow-incomplete is given,
and then says so in the output. Every attempted replication is in every denominator: a replication
whose fit failed, that returned point estimates only (the default's fallback when its standard-error
pass stops), or that has no interval for a component, counts as not covered. Failures and fallbacks
are listed by cell.

Writes <run dir>/summary.json, cells.csv (one row per cell, deletion mode and component) and
rules.csv (one row per candidate rule), and prints a short table.

A candidate rule chooses, replication by replication, between the two intervals the package
computed. It reads only recorded quantities, so it reproduces what the package would have reported
under that rule; the rule the package ran (share above the threshold each fit reports) is recomputed
too and must agree with the recorded choice in every replication.
"""

import json
import os
import sys

import numpy as np
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pkgmc import check  # noqa: E402

COMP = ('worker', 'firm', 'cov', 'sum')
# runs made before the task file recorded the threshold used the package's first value
EARLY_THRESHOLD = 0.05


def pieces(d, j):
    """Recorded pieces of component j (1..4) as arrays; a failed fit has missing values
    throughout."""
    def g(name):
        return d[f'{name}{j}'].to_numpy(float)
    se, lam, score = g('se'), g('lam'), g('b1_')
    # The variance of the leading score the fit used. Runs made before the task file recorded it
    # had no controls, where it is the larger of the two candidates.
    leadvar = g('vb') if f'vb{j}' in d else np.fmax(g('vb_lo'), g('vb_fit'))
    variance = se ** 2
    normal_ok = np.isfinite(se) & (se > 0)
    q1_ok = (g('q1status') == 0) & np.isfinite(g('qlb')) & np.isfinite(g('qub'))
    with np.errstate(invalid='ignore', divide='ignore'):
        square = 2 * lam ** 2 * leadvar ** 2 / variance
        share = (4 * lam ** 2 * np.maximum(score ** 2 - leadvar, 0) * leadvar) / variance + square
        # A bounded version: the direction's part over the direction's part plus the remainder's
        # variance (no cross term, no use of the high-rank variance estimate). Missing when the
        # decomposition is.
        lead = (4 * lam ** 2 * np.maximum(score ** 2 - leadvar, 0) * leadvar
                + 2 * lam ** 2 * leadvar ** 2)
        bounded = lead / (lead + np.maximum(g('vr'), 0))
    threshold = (d['threshold'].to_numpy(float) if 'threshold' in d
                 else np.full(len(d), EARLY_THRESHOLD))
    return dict(threshold=threshold, truth=g('truth'), b=g('b'), se=se, nlb=g('nlb'), nub=g('nub'),
                qlb=g('qlb'), qub=g('qub'), lb=g('lb'), ub=g('ub'), regime=g('regime'),
                share_recorded=g('share'), share=share, square=square, bounded=bounded,
                kappa=g('kappa'), eigshare=g('eigshare'), normal_ok=normal_ok, q1_ok=q1_ok)


def rules():
    """Candidate rules: name -> function of the pieces giving the trigger (True = q = 1 wanted).
    The sum is always normal."""
    out = {'always_normal': lambda p: np.zeros(len(p['b']), bool),
           'always_q1': lambda p: np.ones(len(p['b']), bool)}
    for cut in (0.01, 0.02, 0.05, 0.10, 0.20, 0.50):
        out[f'share>{cut:.2f}'] = lambda p, c=cut: p['share'] > c
    for cut in (0.01, 0.02, 0.05, 0.10):
        out[f'square>{cut:.2f}'] = lambda p, c=cut: p['square'] > c
    for cut in (0.1, 0.2, 0.3, 0.5, 1.0):
        out[f'kappa>{cut:.1f}'] = lambda p, c=cut: p['kappa'] > c
    out['eigshare>=0.10'] = lambda p: p['eigshare'] >= 0.10
    for cut in (0.05, 0.10, 0.15, 0.20, 0.30):
        out[f'bounded>{cut:.2f}'] = lambda p, c=cut: p['bounded'] > c
    for cut in (0.15, 0.30):
        out[f'share>{cut:.2f}'] = lambda p, c=cut: p['share'] > c
    for cut in (0.05, 0.10, 0.20):
        for k in (0.1, 0.2, 0.3):
            out[f'share>{cut:.2f}&kappa>{k:.1f}'] = (
                lambda p, c=cut, k=k: (p['share'] > c) & (p['kappa'] > k))
    return out


def apply_rule(p, trigger, j):
    """The package's logic with another trigger: q = 1 when it exists and (triggered or no normal
    interval), else the normal interval when it exists, else none. Returns (lower, upper, regime)
    with regime 1, 0 or NaN."""
    want = p['q1_ok'] & (j < 4) & (~p['normal_ok'] | trigger)
    regime = np.where(want, 1.0, np.where(p['normal_ok'], 0.0, np.nan))
    lo = np.where(regime == 1, p['qlb'], np.where(regime == 0, p['nlb'], np.nan))
    hi = np.where(regime == 1, p['qub'], np.where(regime == 0, p['nub'], np.nan))
    return lo, hi, regime


def cover(lo, hi, truth):
    with np.errstate(invalid='ignore'):
        return (lo <= truth) & (truth <= hi)  # a missing bound is not covered


def rate(x):
    return float(np.mean(x)) if len(x) else float('nan')


def med(x):
    x = x[np.isfinite(x)]
    return float(np.median(x)) if len(x) else float('nan')


def summarise(frame, man):
    cells, by_rule = [], {name: [] for name in rules()}
    mismatches = []
    groups = {c['name']: c.get('group', '') for c in man['cells']}
    posted = {name: [] for name in rules()}
    for (cell, deletion), d in frame.groupby(['cell', 'deletion'], sort=False):
        d = d.sort_values('rep')
        n = len(d)
        fit_failed = int((d['rc'] != 0).sum())
        any_q1 = {name: np.zeros(n, bool) for name in rules()}
        for j in range(1, 5):
            p = pieces(d, j)
            truth = float(p['truth'][0])
            ok = np.isfinite(p['b'])
            reported = cover(p['lb'], p['ub'], p['truth'])
            normal = cover(p['nlb'], p['nub'], p['truth'])
            q1 = cover(p['qlb'], p['qub'], p['truth'])
            with np.errstate(invalid='ignore'):
                lo, hi, regime = apply_rule(p, p['share'] > p['threshold'], j)
            same = ((np.nan_to_num(regime, nan=-1) == np.nan_to_num(p['regime'], nan=-1))
                    & (np.nan_to_num(lo, nan=-9e9) == np.nan_to_num(p['lb'], nan=-9e9)))
            if not same.all():
                mismatches.append(dict(cell=cell, deletion=deletion, component=COMP[j - 1],
                                       replications=d['rep'].to_numpy()[~same][:10].tolist()))
            sd = float(np.std(p['b'][ok], ddof=1)) if ok.sum() > 1 else float('nan')
            se_over_sd = (float(np.sqrt(np.mean(p['se'][p['normal_ok']] ** 2)) / sd)
                          if p['normal_ok'].any() and sd > 0 else float('nan'))
            cells.append(dict(
                cell=cell, group=groups.get(cell, ''), deletion=deletion, component=COMP[j - 1],
                attempted=n, fit_failed=fit_failed,
                no_interval=int((~np.isfinite(p['lb']) | ~np.isfinite(p['ub'])).sum()),
                no_normal=int((~p['normal_ok']).sum()), no_q1=int((~p['q1_ok']).sum()),
                truth=truth, mean=float(np.mean(p['b'][ok])) if ok.any() else float('nan'), sd=sd,
                se_over_sd=se_over_sd,
                coverage=rate(reported), coverage_normal=rate(normal), coverage_q1=rate(q1),
                q1_used=rate(p['regime'] == 1),
                length=med(p['ub'] - p['lb']), length_normal=med(p['nub'] - p['nlb']),
                length_q1=med(p['qub'] - p['qlb']),
                share_median=med(p['share']), share_over_one=rate(p['share'] > 1),
                kappa_median=med(p['kappa']), eigshare_median=med(p['eigshare']),
                seconds=float(d['seconds'].mean()), joint_posted=rate(d['joint_posted'] == 1)))
            for name, trig in rules().items():
                lo, hi, regime = apply_rule(p, trig(p), j)
                any_q1[name] |= (regime == 1)
                by_rule[name].append(dict(
                    cell=cell, group=groups.get(cell, ''), deletion=deletion, component=COMP[j - 1],
                    coverage=rate(cover(lo, hi, p['truth'])), q1_used=rate(regime == 1),
                    length=med(hi - lo), length_normal=med(p['nub'] - p['nlb'])))
        # The joint covariance matrix is posted when no component takes the q = 1 interval.
        for name in rules():
            posted[name].append(dict(cell=cell, group=groups.get(cell, ''), deletion=deletion,
                                     posted=rate(~any_q1[name] & (d['rc'].to_numpy() == 0))))
    cells = pd.DataFrame(cells)
    rows = []
    for name, lst in by_rule.items():
        r = pd.DataFrame(lst)
        r3 = r[r['component'] != 'sum']
        worst = r3.loc[r3['coverage'].idxmin()]
        with np.errstate(invalid='ignore', divide='ignore'):
            ratio = (r3['length'] / r3['length_normal']).to_numpy(float)
        pr = pd.DataFrame(posted[name])
        calm = pr[pr['group'] == 'none']
        r3c = r3[r3['group'] == 'none']
        with np.errstate(invalid='ignore', divide='ignore'):
            ratio_calm = (r3c['length'] / r3c['length_normal']).to_numpy(float)
        rows.append(dict(
            rule=name, min_coverage=float(worst['coverage']),
            worst=f"{worst['cell']}/{worst['deletion']}/{worst['component']}",
            below_0935=int((r3['coverage'] < 0.935).sum()),
            below_092=int((r3['coverage'] < 0.92).sum()), cases=len(r3),
            mean_q1_used=float(r3['q1_used'].mean()), max_length_ratio=float(np.nanmax(ratio)),
            median_length_ratio=float(np.nanmedian(ratio)),
            # designs without a bottleneck (group "none"): how often the rule gives up the joint
            # matrix, and what it costs in length
            none_q1_used=float(r3c['q1_used'].mean()) if len(r3c) else float('nan'),
            none_posted=float(calm['posted'].mean()) if len(calm) else float('nan'),
            none_max_length_ratio=float(np.nanmax(ratio_calm)) if len(r3c) else float('nan')))
    return cells, pd.DataFrame(rows), by_rule, mismatches


def main(run_dir, allow_incomplete=False):
    run_dir = os.path.abspath(run_dir)
    man = json.load(open(run_dir + '/manifest.json'))
    problems, frame = check(run_dir, quiet=True)
    if problems and not allow_incomplete:
        print('inventory problems:', len(problems))
        for p in problems[:30]:
            print('  ', p)
        raise SystemExit(2)
    if not len(frame):
        raise SystemExit('no complete task output')
    cells, rule_table, by_rule, mismatches = summarise(frame, man)
    failures = frame[frame['rc'] != 0][['cell', 'deletion', 'rep', 'seed', 'rc', 'status']]
    failures = failures.to_dict('records')
    # Fits that returned point estimates only: the default's fallback when its standard-error pass
    # stops.
    fb = frame[(frame['rc'] == 0) & frame['model'].isna()]
    fallbacks = fb[[c for c in ('cell', 'deletion', 'rep', 'seed', 'stream', 'status', 'reason')
                    if c in fb]].to_dict('records')
    package_threshold = (sorted(set(float(v) for v in frame['threshold'].dropna()))
                         if 'threshold' in frame else [EARLY_THRESHOLD])
    out = dict(schema='PKGMC-SUMMARY-V1', run=man['run'],
               manifest_sha256=open(run_dir + '/manifest.sha256').read().strip(),
               complete=not problems, inventory_problems=problems, replications=int(len(frame)),
               fit_failures=failures, fallbacks=fallbacks, package_threshold=package_threshold,
               package_rule_mismatches=mismatches, cells=cells.to_dict('records'),
               rules=rule_table.to_dict('records'), by_rule={k: v for k, v in by_rule.items()})
    for name, text in (('summary.json', json.dumps(out, indent=1)),
                       ('cells.csv', cells.to_csv(index=False)),
                       ('rules.csv', rule_table.to_csv(index=False))):
        tmp = f'{run_dir}/{name}.tmp'
        open(tmp, 'w').write(text)
        os.replace(tmp, f'{run_dir}/{name}')
    pd.set_option('display.width', 250)
    pd.set_option('display.max_rows', 500)
    pd.set_option('display.max_columns', 40)
    print('run', man['run'],
          'complete' if not problems else f'INCOMPLETE ({len(problems)} inventory problems)',
          'replications', len(frame), 'fit failures', len(failures), 'fallbacks', len(fallbacks),
          'rule mismatches', len(mismatches))
    show = cells[['cell', 'group', 'deletion', 'component', 'attempted', 'fit_failed',
                  'no_interval', 'se_over_sd', 'coverage', 'coverage_normal', 'coverage_q1',
                  'q1_used', 'share_median', 'kappa_median']].copy()
    show['len/normal'] = cells['length'] / cells['length_normal']
    show['q1len/normal'] = cells['length_q1'] / cells['length_normal']
    print(show.to_string(index=False, float_format=lambda v: f'{v:.3f}'))
    print(rule_table.to_string(index=False, float_format=lambda v: f'{v:.3f}'))
    return out


if __name__ == '__main__':
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    main(sys.argv[1], '--allow-incomplete' in sys.argv[2:])
