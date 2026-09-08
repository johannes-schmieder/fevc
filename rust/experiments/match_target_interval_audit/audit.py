"""Independent scalar-match matrix audit and strict replay inventory validation."""
from pathlib import Path
import argparse
import collections
import gzip
import importlib.util
import json
import math
import numpy as np
run_spec = importlib.util.spec_from_file_location('match_audit_run', Path(__file__).with_name('run.py'))
run = importlib.util.module_from_spec(run_spec)
run_spec.loader.exec_module(run)

spec = importlib.util.spec_from_file_location('q1_oracle', run.HERE.parent/'residual_moment_q1_target_audit/audit.py')
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
MAP = np.array([[1.,0,0], [0,1,0], [0,0,1], [1,1,2]])


def finite(value):
    if isinstance(value, dict):
        for v in value.values(): finite(v)
    elif isinstance(value, list):
        for v in value: finite(v)
    elif isinstance(value, float) and not math.isfinite(value):
        raise ValueError('nonfinite raw value')


def validate(rows, c):
    finite(rows)
    by = collections.defaultdict(list)
    for r in rows:
        by[r.get('kind', 'baseline')].append(r)
    expected = {'start':1, 'joint':1, 'spectrum':4, 'end':1}
    if c['dense']: expected.update(input=1, geometry=1)
    if c['q'] == 'q1':
        expected['q1'] = 4
        if c['dense']: expected['q1_vectors'] = 4
    if c['group'] == 'comparison': expected['baseline'] = 4
    if {k:len(v) for k,v in by.items()} != expected:
        raise ValueError('capture inventory mismatch')
    start, end = by['start'][0], by['end'][0]
    if rows[0] != start or start['cell'] != c['cell'] or start['replication'] != c['replication'] or end['replication'] != c['replication'] or start['dense'] != c['dense']:
        raise ValueError('call key mismatch')
    if start['seed'] != c['baseline']['worker']['semantic_seed']:
        raise ValueError('original semantic seed changed')
    if c['group'] == 'rejected':
        if (end['status'],end.get('phase'),end.get('code')) != ('failed','component_inference_psd','JLA_CONSTRAINT_FAILED'):
            raise ValueError('original rejection changed')
    elif end['status'] != 'success':
        raise ValueError('successful baseline rejected')
    for kind in ('spectrum','q1','q1_vectors'):
        if kind in by and [r['target'] for r in by[kind]] != list(range(4)):
            raise ValueError('target key/order mismatch')
    joint = by['joint'][0]
    if len(joint['primitive']) != 9 or len(joint['trace']) != 9:
        raise ValueError('joint dimensions')
    for r in by.get('q1', []):
        if len(r['q']) != len(oracle.Q_FIELDS) or r['status'] not in (0,1,2,3,6):
            raise ValueError('q1 status/schema')
        nullable = {'determinant','curvature','critical','lower','upper'}
        for field, v in zip(oracle.Q_FIELDS, r['q']):
            if v is None and (r['status'] == 0 or field not in nullable):
                raise ValueError('missing q1 numerical field')
        if r['critical_draws'] != (4000 if r['status'] == 0 else 0):
            raise ValueError('critical accounting')
    if 'baseline' in by and sorted(r['target'] for r in by['baseline']) != sorted(run.TARGETS):
        raise ValueError('baseline target inventory')
    for r in by.get('baseline', []):
        old = c['baseline'][r['target']]
        for key in ('status','q1_status','outer_fold_fingerprint','variance_floor_count'):
            if r.get(key) != old.get(key): raise ValueError(f'baseline {key} changed')
        for key in ('point_estimate','estimated_variance','confidence_lower','confidence_upper','leading_variance',
                    'raw_leading_variance_correction','remainder_variance','leading_remainder_covariance'):
            if key in old:
                if old[key] is None or r.get(key) is None:
                    if r.get(key) != old[key]: raise ValueError(f'baseline {key} missingness changed')
                elif oracle.relative_error([r[key]], [old[key]]) > 1e-8:
                    raise ValueError(f'baseline {key} numerical change')
    return by


def matrices(inp, geom):
    """Construct physical-row aggregation independently of native match plans."""
    deletion = np.array(inp['deletion'])
    ids, group = np.unique(deletion, return_inverse=True)
    n = len(ids)
    if n != 400: raise ValueError('match count')
    order = np.searchsorted(ids, deletion[np.array(geom['physical_representative'])])
    if sorted(order) != list(range(n)): raise ValueError('match ordering')
    f, y, t = (np.array(inp[k]) for k in ('frequency','outcome','target_weight'))
    mass = np.bincount(group, weights=f)[order]
    yc = np.bincount(group, weights=f*y)[order]/np.sqrt(mass)
    tw = np.bincount(group, weights=t)[order]
    tw /= tw.sum()
    reps = np.array(geom['physical_representative'])
    wi, fi = np.array(inp['worker'])[reps]-10000, np.array(inp['firm'])[reps]-20000
    w = np.zeros((n,39)); z = np.zeros_like(w)
    w[np.arange(n),wi] = 1
    z[np.flatnonzero(fi < 19),20+fi[fi < 19]] = 1
    x = np.sqrt(mass)[:,None]*(w+z)
    inv = np.linalg.inv(x.T@x)
    a = x@inv
    center = np.diag(tw)-np.outer(tw,tw)
    qw, qf = w.T@center@w, z.T@center@z
    cross = w.T@center@z
    qs = [qw, qf, (cross+cross.T)/2, qw+qf+cross+cross.T]
    plugins = [a@q@a.T for q in qs]
    maker = np.eye(n)-a@x.T
    ratios = MAP@np.array(geom['ratio'])
    kernels = [b-(r[:,None]*maker+maker*r[None,:])/2 for b,r in zip(plugins,ratios)]
    return yc, np.array(inp['true_variance'])[order], maker, plugins, kernels, ratios


def dense_check(by, c):
    geom, inp = by['geometry'][0], by['input'][0]
    y, truth_s, maker, plugins, kernels, ratios = matrices(inp, geom)
    s, inv = np.array(geom['variance']), np.array(geom['maker_inverse'])
    if any(len(a) != 400 for a in (s,inv,y)) or min(s) <= 0:
        raise ValueError('geometry dimension/positivity')
    errors = {'collapsed_y':oracle.relative_error(y,geom['y'])}
    influences = np.array([a@y for a in kernels])
    errors['q0_influence'] = oracle.relative_error(influences[:3],geom['influence'])
    primitive = np.array(by['joint'][0]['primitive']).reshape(3,3)
    stochastic = np.array(by['joint'][0]['trace']).reshape(3,3)
    errors['q0_covariance'] = oracle.relative_error(4*(influences[:3]*s)@influences[:3].T-stochastic,primitive)
    arms = {}
    for name, variance in [('exact_trace_fitted',s), ('exact_trace_true_variance',truth_s)]:
        trace = np.array([[2*np.einsum('ij,ij,i,j->',a,b,variance,variance) for b in kernels[:3]] for a in kernels[:3]])
        cov = 4*(influences[:3]*variance)@influences[:3].T-trace
        arms[name] = dict(q0_variances=np.diag(MAP@cov@MAP.T).tolist(), joint_min_eigenvalue=float(np.linalg.eigvalsh(cov)[0]),q1=[])
    if c['q'] == 'q1':
        for t,(row, vectors) in enumerate(zip(by['q1'],by['q1_vectors'])):
            q = dict(zip(oracle.Q_FIELDS,row['q']))
            v, lam = np.array(vectors['mode']), row['eigenvalue']
            r = oracle.remainder_kernel(kernels[t],maker,inv,v,lam)
            errors[f'q1_influence_{t}'] = oracle.relative_error(r@y,vectors['influence'])
            errors[f'q1_ratio_{t}'] = oracle.relative_error(ratios[t]-lam*v*v*inv,vectors['ratio'])
            calc = oracle.calculate(y,s,v,lam,kernels[t],r,maker,inv,q['trace'],by['spectrum'][t]['certified'],radius=q['critical'])
            if calc['status'] != row['status']: raise ValueError('dense q1 status mismatch')
            keys = [k for k in calc if k not in ('status','identity_error') and q[k] is not None]
            errors[f'q1_quantities_{t}'] = oracle.relative_error([calc[k] for k in keys],[q[k] for k in keys])
            for name, variance in [('exact_trace_fitted',s), ('exact_trace_true_variance',truth_s)]:
                arms[name]['q1'].append(oracle.calculate(y,variance,v,lam,kernels[t],r,maker,inv,oracle.exact_trace(r,variance),by['spectrum'][t]['certified']))
    if max(errors.values()) > 1e-8:
        raise ValueError(f'dense reconstruction mismatch: {errors}')
    return arms, errors


def inspect(by, c):
    primitive = np.array(by['joint'][0]['primitive']).reshape(3,3)
    variances = np.diag(MAP@primitive@MAP.T)
    rows = []
    for t,target in enumerate(run.TARGETS):
        r = dict(q=c['q'],cell=c['cell'],replication=c['replication'],group=c['group'],target=target,
                 q0_variance=float(variances[t]), q0_computable=bool(variances[t]>0),
                 spectrum=by['spectrum'][t])
        if c['q'] == 'q1':
            native = by['q1'][t]
            q = dict(zip(oracle.Q_FIELDS,native['q']))
            status = oracle.pair_status(q['vb'],q['cross'],q['vr'],by['spectrum'][t]['certified'])
            if status != native['status']: raise ValueError('q1 pair status mismatch')
            r.update(q1_status=status, q1=q)
            if status == 0:
                endpoints = oracle.ellipse([q['score'],q['remainder']], [q['vb'],q['cross'],q['vr']],q['critical'],native['eigenvalue'])
                if oracle.relative_error(endpoints,[q['lower'],q['upper']]) > 1e-8:
                    raise ValueError('independent ellipse mismatch')
                radius = oracle.critical_value(q['curvature'])
                lo,hi = oracle.ellipse([q['score'],q['remainder']], [q['vb'],q['cross'],q['vr']],radius,native['eigenvalue'])
                r['quadrature_interval'] = [lo,hi]
        rows.append(r)
    return rows


def main():
    p = argparse.ArgumentParser()
    p.add_argument('output',type=Path)
    p.add_argument('--profile',choices=['tiny','replay'],default='replay')
    args = p.parse_args(); out = args.output.resolve(); dest=out/args.profile
    manifest=json.loads((out/'manifest.json').read_text())
    if run.sha(run.PROTOCOL) != manifest['protocol_sha256'] or run.sha(run.HERE.parent/'residual_moment_q1_target_audit/audit.py') != manifest['oracle_source_sha256']:
        raise ValueError('protocol/oracle identity changed')
    chosen=manifest['selected']
    if args.profile == 'tiny':
        chosen=[next(c for c in chosen if c['q']==q and c['group']==g) for q in ('q0','q1') for g in ('rejected','comparison')]
    execution=json.loads((dest/'execution.json').read_text())
    if execution['failed_processes'] or execution['expected_calls'] != len(chosen): raise ValueError('execution failed/incomplete')
    expected={f'{c["q"]}-{c["cell"]}-{c["replication"]:04d}' for c in chosen}
    if set(execution['receipt_hashes']) != expected or {p.name[:-9] for p in dest.glob('*.jsonl.gz')} != expected or {p.name[:-13] for p in dest.glob('*.receipt.json')} != expected:
        raise ValueError('file inventory mismatch')
    records=[]; dense=[]; maxima={}; failures=[]
    for i,c in enumerate(chosen):
        key=f'{c["q"]}-{c["cell"]}-{c["replication"]:04d}'
        receipt=json.loads((dest/f'{key}.receipt.json').read_text())
        try:
            if run.sha(dest/f'{key}.receipt.json') != execution['receipt_hashes'][key] or run.sha(dest/f'{key}.jsonl.gz') != receipt['output_sha256'] or run.sha(dest/f'{key}.stderr') != receipt['stderr_sha256'] or receipt['manifest_sha256'] != run.sha(out/'manifest.json') or receipt['exit_code']:
                raise ValueError('receipt/hash/exit mismatch')
            with gzip.open(dest/f'{key}.jsonl.gz','rt') as f: raw=[json.loads(l) for l in f]
            by=validate(raw,c)
            records.extend(inspect(by,c))
            if c['dense']:
                arms, errors=dense_check(by,c)
                dense.append(dict(q=c['q'],cell=c['cell'],replication=c['replication'],group=c['group'],arms=arms,errors=errors))
                for k,v in errors.items(): maxima[k]=max(maxima.get(k,0),v)
        except (ValueError,KeyError,IndexError,TypeError) as e:
            failures.append(dict(key=key,error=str(e)))
        if (i+1)%500==0: print(f'Audited {i+1}/{len(chosen)}',flush=True)
    summary={}
    for r in records:
        key=f'{r["q"]}/{r["cell"]}/{r["group"]}/{r["target"]}'
        s=summary.setdefault(key,dict(attempts=0,q0_computable=0,q1_status_counts={}))
        s['attempts']+=1; s['q0_computable']+=r['q0_computable']
        if 'q1_status' in r:
            status=str(r['q1_status']); s['q1_status_counts'][status]=s['q1_status_counts'].get(status,0)+1
    result=dict(schema='FEVC_MATCH_TARGET_INTERVAL_AUDIT_RESULT_V1',status='DIAGNOSTIC_PASS' if not failures else 'DIAGNOSTIC_FAIL',
                profile=args.profile,manifest_sha256=run.sha(out/'manifest.json'),calls=len(chosen),target_rows=len(records),dense_calls=len(dense),
                failures=failures,summary=summary,reconstruction_maxima=maxima,
                limitation='Existing draws selected on original full-call status; computability, not a new coverage or promotion claim.')
    run.write_json(dest/'audit-result.json',result)
    with gzip.open(dest/'target-audit.jsonl.gz','xt') as f:
        for r in records: f.write(json.dumps(r,allow_nan=False)+'\n')
    with gzip.open(dest/'dense-audit.jsonl.gz','xt') as f:
        for r in dense: f.write(json.dumps(r,allow_nan=False)+'\n')
    print(json.dumps(result,indent=2))
    if failures: raise SystemExit(1)


if __name__=='__main__': main()
