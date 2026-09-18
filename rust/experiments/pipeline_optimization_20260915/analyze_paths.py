#!/usr/bin/env python3
"""Compare immutable core-only attachment diagnostics; never imply acceptance."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def read(path):
    value = json.loads(path.read_text())
    assert all(c['status'] == 'pass' and c['returncode'] == 0 for c in value['calls'])
    calls = {(c['mode'], c['threads']): c for c in value['calls']}
    assert len(calls) == len(value['calls']) == 12
    return calls


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('baseline', type=Path)
    parser.add_argument('candidate', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    before, after = read(args.baseline), read(args.candidate)
    assert before.keys() == after.keys()
    text = ['# Attachment diagnostic comparison', '',
            'Single capture-enabled core calls on public 8k inputs. Not repeated',
            'native-command performance, RSS, SCC or Matlab evidence. No acceptance claim.', '',
            '| Path | Threads | Before (s) | After (s) | Core speedup |',
            '|---|---:|---:|---:|---:|']
    maximum_gap = 0.0
    for key in sorted(before):
        old, new = before[key], after[key]
        values = []
        for call in [old, new]:
            fields = dict(re.findall(r'(worker|firm|covariance|total|residual|gate)=([^ ]+)', call['receipt'][0]))
            assert float(fields['residual']) <= float(fields['gate'])
            values.append([float(fields[k]) for k in ['worker', 'firm', 'covariance', 'total']])
        gap = max(abs(a-b)/max(1.0, abs(a), abs(b)) for a, b in zip(*values))
        assert gap <= 1e-8
        maximum_gap = max(maximum_gap, gap)
        if 'component' in key[0]:
            status = lambda c: re.search(r'q0_status=\[([^]]+)\]', c['receipt'][1]).group(1)
            assert status(old) == status(new)
        seconds = [c['phases']['command']['inclusive_seconds'] for c in [old, new]]
        text.append(f'| {key[0]} | {key[1]} | {seconds[0]:.6f} | {seconds[1]:.6f} | {seconds[0]/seconds[1]:.3f} |')
    text += ['', f'Maximum scaled corrected-target gap: {maximum_gap:.3g}.', '',
             'q=0 computation/withholding statuses are unchanged; all residual gates pass.', '',
             '## Seven-thread component phases', '',
             '| Path | Phase | Before (s) | After (s) |', '|---|---|---:|---:|']
    for mode in ['observation-component', 'match-component']:
        for phase in ['component_gram', 'component_spectrum', 'component_prepare', 'component_statistics']:
            a = before[mode, 7]['phases'][phase]['inclusive_seconds']
            b = after[mode, 7]['phases'][phase]['inclusive_seconds']
            text.append(f'| {mode} | {phase} | {a:.6f} | {b:.6f} |')
    text += ['', '## Source evidence', '']
    for name, path in [('Before', args.baseline), ('After', args.candidate)]:
        text.append(f'{name}: `{path}`, SHA256 `{hashlib.sha256(path.read_bytes()).hexdigest()}`.')
        text.append('')
    with args.output.open('x') as stream:
        stream.write('\n'.join(text))
    print(args.output)


if __name__ == '__main__':
    main()
