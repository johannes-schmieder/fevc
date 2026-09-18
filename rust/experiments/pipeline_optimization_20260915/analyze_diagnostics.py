#!/usr/bin/env python3
"""Validate and summarize the bounded core diagnostics, not performance gates."""
import argparse
import json
import math
from pathlib import Path
import re


def numbers(record):
    values = {key: float(value) for key, value in re.findall(
        r'\b(estimate|worker|firm|covariance|total)=([^\s]+)', record['result'])}
    assert len(values) == 5 and all(map(math.isfinite, values.values()))
    return values


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('baseline', type=Path)
    parser.add_argument('candidate', type=Path)
    parser.add_argument('report', type=Path)
    args = parser.parse_args()
    before = json.loads(args.baseline.read_text())
    after = json.loads(args.candidate.read_text())
    assert len(before) == 30 and len(after) == 45
    assert all(row['passed'] and row['returncode'] == 0 for row in before + after)
    baseline = {row['key']: row for row in before}
    gaps = []
    comparisons = []
    for row in after:
        key = row['key']
        reference = baseline[key.replace('-generic-match-', '-match-')]
        assert row['input_sha256'] == reference['input_sha256']
        a, b = numbers(reference), numbers(row)
        for target in ('worker', 'firm', 'covariance', 'total'):
            gap = abs(a[target] - b[target])
            assert gap <= 1e-8 * max(1, abs(a[target]), abs(b[target])), (key, target, gap)
            gaps.append(gap)
        if '-generic-match-' not in key:
            comparisons.append(dict(key=key, baseline=a['estimate'], candidate=b['estimate'],
                                    speedup=a['estimate']/b['estimate']))
    lines = ['# First-slice core diagnostics', '',
        'All 45 candidate calls PASS; all four corrected targets pass the registered',
        'same-draw tolerance against the 30 baseline calls. The additional generic-',
        'match calls are compared numerically, not timed as incremental improvements',
        'over the baseline compressed route.', '',
        f'Maximum absolute corrected-target gap: {max(gaps):.4g}.', '',
        'These are single capture-enabled core timings on 100k public inputs.',
        'They exclude Stata/native-command preparation and are not rotated paired',
        'performance acceptance, representative-scale claims, or Matlab comparisons.', '',
        '| Configuration | Baseline core seconds | Candidate core seconds | Baseline/candidate |',
        '|---|---:|---:|---:|']
    for row in comparisons:
        lines.append(f"| {row['key']} | {row['baseline']:.4f} | {row['candidate']:.4f} | {row['speedup']:.3f} |")
    lines += ['', '## Identity', '',
        'Candidate source snapshots: ' + ', '.join(sorted({row['source_receipt_sha256'] for row in after})),
        'Candidate binaries: ' + ', '.join(sorted({row['executable_sha256'] for row in after})),
        'Baseline binaries: ' + ', '.join(sorted({row['executable_sha256'] for row in before})),
        '', 'Every attempted diagnostic remains in the input JSON files and per-call logs.', '']
    with args.report.open('x') as stream:
        stream.write('\n'.join(lines))
    print(json.dumps(dict(passed=len(after), maximum_gap=max(gaps), comparisons=comparisons)))


if __name__ == '__main__':
    main()
