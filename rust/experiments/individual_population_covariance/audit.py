"""Independent dense Gaussian moments for small, fixed synthetic designs only."""
import argparse
import collections
import json
import math
from pathlib import Path
import sys

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import run
from numerics import critical, ellipse, summarize_intervals

TARGETS = ('worker','firm','covariance','total')
MAP = np.array([[1.,0,0], [0,1,0], [0,0,1], [1,1,2]])


def error(a, b):
    a, b = np.asarray(a, dtype=float), np.asarray(b, dtype=float)
    if a.shape != b.shape or not np.isfinite(a).all() or not np.isfinite(b).all():
        raise ValueError('nonfinite or mismatched numerical arrays')
    return float(np.max(abs(a-b)/np.maximum(1., np.maximum(abs(a),abs(b)))))


def gaussian_quadratic(kernel, mean, variance):
    """Cumulants from the Gaussian quadratic-form generating function.

    For A=Sigma^(1/2) K Sigma^(1/2), b=Sigma^(1/2) K mu,
    kappa_r=2^(r-1)(r-1)! tr(A^r)+2^(r-1)r! b'A^(r-2)b.
    The independent deterministic quadrature test checks moments through four.
    """
    k, mu, s = map(lambda a: np.asarray(a, dtype=float), (kernel,mean,variance))
    if k.shape != (len(mu),len(mu)) or s.shape != mu.shape or mu.ndim != 1:
        raise ValueError('Gaussian moment dimensions')
    if not all(np.isfinite(a).all() for a in (k,mu,s)) or np.any(s <= 0) or error(k,k.T)>1e-12:
        raise ValueError('invalid Gaussian kernel or variance')
    root = np.sqrt(s)
    a = k*root[:,None]*root[None,:]
    b = root*(k@mu)
    a2 = a@a
    var = float(2*np.sum(a*a)+4*(b@b))
    k4 = float(48*np.sum(a2*a2)+192*np.dot(a@b,a@b))
    return dict(mean=float(mu@k@mu+np.dot(np.diag(k),s)), variance=var,
                fourth_cumulant=k4, fourth_central_moment=k4+3*var*var)


def sample_variance_mcse(variance, fourth_central_moment, n):
    if n < 2 or variance < 0 or fourth_central_moment < variance**2:
        raise ValueError('invalid sample variance moments')
    return math.sqrt((fourth_central_moment-(n-3)/(n-1)*variance**2)/n)


def covariance_of_quadratics(kernels, mu, s):
    u = np.array([k@mu for k in kernels])
    return np.array([[4*np.dot(a*s,b)+2*np.einsum('ij,ij,i,j->',ka,kb,s,s)
                      for b,kb in zip(u,kernels)] for a,ka in zip(u,kernels)])


def validate(rows, family, entry):
    by = collections.defaultdict(list)
    for row in rows:
        by[row.get('kind')].append(row)
    expected = dict(task=1, design=1, population=1 if family=='observation' else 400,
                    input=400, geometry=2, vectors=8, call=2)
    if {k:len(v) for k,v in by.items()} != expected:
        raise ValueError('capture inventory mismatch')
    task = by['task'][0]
    if rows[0] != task or task['family'] != family or task['profile'] != 'development' or task['start'] != 0 or task['reps'] != 400 or task['master'] != entry['master'] or task['numseed'] != 8675309:
        raise ValueError('task identity mismatch')
    if by['design'][0]['truth'] != entry['truth']:
        raise ValueError('target truth mismatch')
    if [r['replication'] for r in by['input']] != list(range(400)) or [r['replication'] for r in by['call']] != [0,399]:
        raise ValueError('missing, duplicate or reordered draws')
    if [r['target'] for r in by['vectors']] != list(range(4))*2:
        raise ValueError('target vector order')
    for r in by['input']:
        if r['seed'] != entry['calls'][r['replication']]['seed']:
            raise ValueError('semantic seed changed')
        if any(r[k] != by['input'][0][k] for k in ('worker','firm','deletion','frequency','target_weight','controls')):
            raise ValueError('changing physical design')
    if any(row != by['population'][0] for row in by['population']):
        raise ValueError('changing population design')
    strip = lambda row: {k:v for k,v in row.items() if k != 'seconds'}
    for row in by['call']:
        if not run.same(strip(row),strip(entry['calls'][row['replication']])):
            raise ValueError('native replay identity failed')
    # Canonical row order within a match can change with outcomes. Membership
    # sets and the ordered inference units, not within-unit order, define X.
    if [sorted(g) for g in by['geometry'][0]['members']] != [sorted(g) for g in by['geometry'][1]['members']]:
        raise ValueError('changing inference-unit membership')
    for k in ('maker_inverse','ratio'):
        if not run.same(by['geometry'][0][k],by['geometry'][1][k]):
            raise ValueError('changing numerical geometry')
    for a,b in zip(by['vectors'][:4],by['vectors'][4:]):
        if any(not run.same(a[k],b[k]) for k in ('mode','ratio','eigenvalue')):
            raise ValueError('changing q1 numerical kernel')
    # Associate captures with the immediately preceding input, not merely counts.
    current = None
    capture_reps = []
    for row in rows:
        if row['kind']=='input': current=row['replication']
        if row['kind']=='geometry': capture_reps.append(current)
    if capture_reps != [0,399]:
        raise ValueError('misassociated geometry')
    return by


def matrices(inp, population, geometry, family):
    nphys = len(inp['worker'])
    members = geometry['members']
    if any(not group for group in members) or sorted(i for group in members for i in group) != list(range(nphys)):
        raise ValueError('physical membership is not a partition')
    f, tw = (np.asarray(inp[k],dtype=float) for k in ('frequency','target_weight'))
    mu = np.asarray(population['mean'],dtype=float)
    if any(len(inp[k]) != nphys for k in ('firm','deletion','frequency','target_weight','outcome')) or mu.shape != (nphys,) or np.any(f <= 0) or np.any(tw < 0) or tw.sum() <= 0:
        raise ValueError('invalid physical columns')
    if not all(np.isfinite(a).all() for a in (f,tw,mu)):
        raise ValueError('nonfinite physical columns')
    n = len(members)
    collapse = np.zeros((n,nphys))
    reps = np.array([group[0] for group in members])
    mass = np.array([f[group].sum() for group in members])
    weights = np.array([tw[group].sum() for group in members]); weights /= weights.sum()
    for i,group in enumerate(members):
        collapse[i,group] = f[group]/np.sqrt(mass[i])
        for field in ('worker','firm','deletion'):
            if len(set(inp[field][j] for j in group)) != 1:
                raise ValueError('mixed identifiers in inference unit')
    if family=='observation':
        if any(len(g)!=1 for g in members) or not np.all(f==1):
            raise ValueError('registered unit observation design changed')
        s = np.array(population['variance'])[reps]
    else:
        ids = np.unique(inp['deletion'])
        if len(ids)!=n or inp['controls']:
            raise ValueError('registered match units or controls changed')
        order = np.searchsorted(ids, np.array(inp['deletion'])[reps])
        s = np.array(population['variance'])[order]
    if s.shape!=(n,) or not np.isfinite(s).all() or np.any(s<=0):
        raise ValueError('population unit variance dimensions/positivity')
    wi = np.unique(inp['worker'],return_inverse=True)[1][reps]
    fi = np.unique(inp['firm'],return_inverse=True)[1][reps]
    nw,nf,nc = int(wi.max())+1,int(fi.max())+1,len(inp['controls'])
    p = nw+nf-1+nc
    w,z = np.zeros((n,p)),np.zeros((n,p))
    w[np.arange(n),wi]=1
    z[np.flatnonzero(fi<nf-1), nw+fi[fi<nf-1]]=1
    x = np.sqrt(mass)[:,None]*(w+z)
    if nc:
        controls = np.asarray(inp['controls'],dtype=float)
        if controls.shape!=(nc,nphys) or not np.isfinite(controls).all():
            raise ValueError('control dimensions')
        x[:,-nc:] = collapse@controls.T
    chol = np.linalg.cholesky(x.T@x)
    inverse = np.linalg.solve(chol.T,np.linalg.solve(chol,np.eye(p)))
    a = x@inverse
    maker = np.eye(n)-a@x.T
    if error(maker,maker.T)>1e-12 or error(maker@x,np.zeros_like(x))>1e-10:
        raise ValueError('independent projection failed')
    center = np.diag(weights)-np.outer(weights,weights)
    aw,af,cross = w.T@center@w,z.T@center@z,w.T@center@z
    targets = [aw,af,(cross+cross.T)/2,aw+af+cross+cross.T]
    plugins = np.array([a@target@a.T for target in targets])
    ratios = MAP@np.asarray(geometry['ratio'])
    if ratios.shape!=(4,n):
        raise ValueError('ratio dimensions')
    kernels = [b-(r[:,None]*maker+maker*r[None,:])/2 for b,r in zip(plugins,ratios)]
    ideal = [b-(r[:,None]*maker+maker*r[None,:])/2 for b in plugins
             for r in [np.diag(b)/np.diag(maker)]]
    return collapse,collapse@mu,s,maker,plugins,kernels,ideal,ratios


def analyze(by, entry, family):
    geometry = by['geometry'][0]
    collapse,mu,s,maker,plugins,kernels,ideal,ratios = matrices(by['input'][0],by['population'][0],geometry,family)
    ys = np.array([row['outcome'] for row in by['input']])@collapse.T
    inv = np.array(geometry['maker_inverse'])
    calls = [entry['calls'][i] for i in range(400)]
    saved = np.array([row['q1'] for row in calls]).reshape(400,4,20)
    if not np.isfinite(saved).all() or np.any(saved[:,:,16]!=0):
        raise ValueError('saved q1 target failure or nonfinite result')
    checks = {'native_calls':2, 'dense':{}, 'all_saved_draws':400}
    check = checks['dense']
    check['truth'] = error([mu@b@mu for b in plugins],entry['truth'])
    check['captured_outcomes'] = error(ys[[0,399]], [g['y'] for g in by['geometry']])
    point = np.array([np.einsum('ni,ij,nj->n',ys,k,ys) for k in kernels]).T
    check['all_point_estimates'] = error(point,np.array([r['point'] for r in calls]))
    targets=[]; interval_rows=[]
    for t,name in enumerate(TARGETS):
        vectors = by['vectors'][t]
        v,lam = np.array(vectors['mode']),vectors['eigenvalue']
        ratio = ratios[t]-lam*v*v*inv
        r = plugins[t]-lam*np.outer(v,v)-(ratio[:,None]*maker+maker*ratio[None,:])/2
        second_r = kernels[t]-lam*np.outer(v,v)+(np.diag(lam*v*v*inv)@maker+maker@np.diag(lam*v*v*inv))/2
        check[f'remainder_forms_{t}'] = error(r,second_r)
        check[f'q1_ratio_{t}'] = error(ratio,vectors['ratio'])
        influence = ys@r
        score, rem = ys@v, np.sum(ys*influence,axis=1)
        q = saved[:,t,:]
        check[f'all_scores_{t}'] = error(score,q[:,1])
        check[f'all_remainders_{t}'] = error(rem,q[:,4])
        check[f'all_recenterings_{t}'] = error(np.sum(ys*(ys@maker)*(v*v*inv),axis=1),q[:,14])
        check[f'influences_{t}'] = error(influence[[0,399]], [by['vectors'][j+t]['influence'] for j in (0,4)])
        if abs(v@v-1)>1e-10:
            raise ValueError('leading mode normalization')
        leading_variance = float(np.dot(v*v,s))
        mom = gaussian_quadratic(r,mu,s)
        cross = float(2*np.dot(v*s,r@mu))
        population = np.array([[leading_variance,cross],[cross,mom['variance']]])
        np.linalg.cholesky(population)
        empirical = np.cov(np.column_stack((score,rem)),rowvar=False,ddof=1)
        modeled = np.array([[[a[2],a[5]],[a[5],a[6]]] for a in q])
        model_mean,model_mcse = modeled.mean(axis=0),modeled.std(axis=0,ddof=1)/20
        oracle_trace = float(2*np.einsum('ij,ij,i,j->',r,r,s,s))
        true_realized = np.array([np.full(400,leading_variance), 2*influence@(v*s),
                                 4*(influence*influence)@s-oracle_trace]).T
        reported = q[:,[2,5,6]]
        paired_difference = reported-true_realized
        empirical_mcse = np.array([sample_variance_mcse(leading_variance,3*leading_variance**2,400),
                                    sample_variance_mcse(mom['variance'],mom['fourth_central_moment'],400)])
        point_mom = gaussian_quadratic(kernels[t],mu,s)
        ideal_mom = gaussian_quadratic(ideal[t],mu,s)
        exact_r = ideal[t]-lam*np.outer(v,v)
        correction = lam*v*v/np.diag(maker)
        exact_r += (correction[:,None]*maker+maker*correction[None,:])/2
        ideal_remainder = gaussian_quadratic(exact_r,mu,s)
        cond = mom['variance']-cross*cross/leading_variance
        curvature = 2*abs(lam)*leading_variance/math.sqrt(cond)
        radius = critical(curvature)
        intervals = [ellipse([a,b],population,radius,lam) for a,b in zip(score,rem)]
        diag = np.diag(population)
        targets.append(dict(target=name, primary=name!='covariance', eigenvalue=lam,
            leading_share=calls[0]['spectrum'][15*t+6], population_covariance=population.tolist(),
            empirical_covariance=empirical.tolist(), mean_reported_covariance=model_mean.tolist(),
            reported_covariance_mean_mcse=model_mcse.tolist(),
            empirical_variance_over_population=(np.diag(empirical)/diag).tolist(),
            mean_reported_variance_over_population=(np.diag(model_mean)/diag).tolist(),
            reported_mean_variance_ratio_mcse=(np.diag(model_mcse)/diag).tolist(),
            empirical_variance_mcse=empirical_mcse.tolist(),
            empirical_variance_z=((np.diag(empirical)-diag)/empirical_mcse).tolist(),
            true_variance_realized_mean=true_realized.mean(axis=0).tolist(),
            true_variance_realized_mean_mcse=(true_realized.std(axis=0,ddof=1)/20).tolist(),
            paired_reported_minus_true_variance_mean=paired_difference.mean(axis=0).tolist(),
            paired_difference_mean_mcse=(paired_difference.std(axis=0,ddof=1)/20).tolist(),
            population_point=point_mom, ideal_exact_diagonal_point=ideal_mom,
            point_truth=entry['truth'][t], point_bias_in_population_sd=(point_mom['mean']-entry['truth'][t])/math.sqrt(point_mom['variance']),
            remainder_population=mom, ideal_exact_diagonal_remainder=ideal_remainder,
            population_coordinate_means=[float(v@mu),mom['mean']],
            original_intervals=summarize_intervals(q[:,10:12],entry['truth'][t]),
            population_covariance_intervals=summarize_intervals(intervals,entry['truth'][t]),
            population_curvature=curvature, population_critical_radius=radius))
        interval_rows.append(dict(target=name, intervals=intervals))
    if max(check.values())>1e-8:
        raise ValueError(f'dense reconstruction mismatch: {check}')
    point_cov = covariance_of_quadratics(kernels,mu,s)
    reported_cov = np.array([c['covariance'] for c in calls],dtype=float).reshape(400,4,4)
    available = np.isfinite(reported_cov).all(axis=(1,2))
    # Joint admission is separate from the complete q1 coordinate inventory.
    # This optional descriptive mean explicitly discloses its conditioning.
    joint_summary = dict(attempts=400, available=int(available.sum()),
                         mean_among_available=reported_cov[available].mean(axis=0).tolist() if available.any() else None)
    return dict(family=family, physical_rows=collapse.shape[1], independent_units=collapse.shape[0],
                checks=checks, targets=targets, population_point_covariance=point_cov.tolist(),
                empirical_point_covariance=np.cov(point,rowvar=False).tolist(),
                reported_joint_covariance=joint_summary), interval_rows


def audit(output):
    manifest = json.loads((output/'manifest.json').read_text())
    for path,digest in manifest['inputs'].items():
        if run.sha(path)!=digest:
            raise ValueError(f'changed diagnostic input: {path}')
    if run.sha(output/'build-receipt.json')!=manifest['build_receipt_sha256']:
        raise ValueError('changed build receipt')
    execution = json.loads((output/'execution.json').read_text())
    if execution['status']!='PASS' or sorted(r['family'] for r in execution['receipts'])!=['match_q1','observation']:
        raise ValueError('execution failure/inventory')
    _, cells = run.prior.load_cells(Path(manifest['parent']))
    results=[];failures=[]
    for receipt in execution['receipts']:
        family=receipt['family']
        try:
            if receipt['exit_code'] or run.sha(output/f'{family}.jsonl')!=receipt['output_sha256'] or run.sha(output/f'{family}.stderr')!=receipt['stderr_sha256']:
                raise ValueError('capture receipt mismatch')
            key = next(k for k in run.prior.Q1_CELLS if k[0]==family)
            rows = [json.loads(line) for line in (output/f'{family}.jsonl').read_text().splitlines()]
            by = validate(rows,family,cells[key])
            result,intervals = analyze(by,cells[key],family)
            results.append(result)
            run.write(output/f'{family}-population-intervals.json', intervals)
        except Exception as exc:
            failures.append(dict(family=family,error=str(exc)))
    run.write(output/'result.json', dict(status='DIAGNOSTICS_PASS_NOT_QUALIFICATION' if not failures else 'FAIL',
              manifest_sha256=run.sha(output/'manifest.json'), execution_sha256=run.sha(output/'execution.json'),
              results=results,failures=failures,new_outcomes=0,original_development_status='FAIL',
              versions=dict(python=sys.version,numpy=np.__version__)))
    print(json.dumps(dict(status='FAIL' if failures else 'PASS',failures=failures)))
    return bool(failures)


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('output',type=Path)
    args=parser.parse_args();sys.exit(audit(args.output.resolve()))
