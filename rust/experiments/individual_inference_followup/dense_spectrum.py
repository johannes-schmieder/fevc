"""Independent coefficient-space generalized eigenvalues on synthetic fixtures.

Diagnostic cross-check, not a new or changed production acceptance threshold.
The target weights are stored-row masses; frequencies enter regression only.
"""
import argparse
import json
from pathlib import Path
import numpy as np
from run import sha, write


def check(directory):
    result = json.loads((directory/'replay-result.json').read_text())
    rows = []
    for pair in result['pairs']:
        if pair['replication'] != 0 or 'extra_solver_columns' not in pair:
            continue
        folder = directory/'pairs'/pair['id']
        fixture = folder/'fixture.csv'
        data = np.genfromtxt(fixture, delimiter=',', names=True)
        _, worker = np.unique(data['worker'], return_inverse=True)
        _, firm = np.unique(data['firm'], return_inverse=True)
        nw, nf = max(worker)+1, max(firm)+1
        w = np.eye(nw)[worker]
        f = np.eye(nf)[firm, :-1]
        x = np.column_stack((w, f))
        h = x.T@(data['frequency'][:, None]*x)
        root = np.linalg.cholesky(h)
        weights = data['target']/sum(data['target'])
        w -= weights@w; f -= weights@f
        qw = np.zeros_like(h); qf = qw.copy(); qc = qw.copy()
        qw[:nw, :nw] = w.T@(weights[:, None]*w)
        qf[nw:, nw:] = f.T@(weights[:, None]*f)
        qc[:nw, nw:] = .5*w.T@(weights[:, None]*f)
        qc[nw:, :nw] = qc[:nw, nw:].T
        native = json.loads((folder/'candidate.jsonl').read_text().splitlines()[2])
        for t, a in enumerate((qw, qf, qc, qw+qf+2*qc)):
            whitened = np.linalg.solve(root, a)
            whitened = np.linalg.solve(root, whitened.T).T
            eigenvalues = np.linalg.eigvalsh(whitened)
            leading = np.sort(np.abs(eigenvalues))[-2:][::-1]
            reported = np.abs(native['spectrum'][15*t:15*t+2])
            relative_error = float(max(abs(leading-reported))/leading[0])
            exact_trace = float(eigenvalues@eigenvalues)
            estimated_trace = native['spectrum'][15*t+2]
            trace_mcse = native['spectrum'][15*t+4]
            rows.append({'cell': pair['key'], 'target': t, 'fixture_sha256': sha(fixture),
                         'exact_absolute_leading_eigenvalues': leading.tolist(),
                         'reported_absolute_leading_eigenvalues': reported.tolist(),
                         'leading_scaled_difference': relative_error,
                         'trace_difference_in_reported_mcse': (estimated_trace-exact_trace)/trace_mcse})
    if len(rows) != 60:
        raise ValueError('dense oracle inventory')
    return {'status': 'DIAGNOSTIC_COMPLETE', 'source_sha256': sha(Path(__file__)),
            'maximum_leading_scaled_difference': max(r['leading_scaled_difference'] for r in rows),
            'maximum_absolute_trace_difference_in_reported_mcse': max(abs(r['trace_difference_in_reported_mcse']) for r in rows),
            'rows': rows}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('directory', type=Path); args = parser.parse_args()
    result = check(args.directory)
    write(args.directory/'dense-spectrum-result.json', result)
    print(json.dumps({k: v for k, v in result.items() if k != 'rows'}))
