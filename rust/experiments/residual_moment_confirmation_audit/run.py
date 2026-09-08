"""Exact, existing-draw diagnosis; never replaces a confirmation failure."""
from pathlib import Path
import argparse
import importlib.util
import json
import subprocess
import numpy as np

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
spec=importlib.util.spec_from_file_location('confirmation',HERE.parent/'residual_moment_confirmation/run.py')
confirmation=importlib.util.module_from_spec(spec);spec.loader.exec_module(confirmation)
sha=confirmation.sha
write_json=confirmation.write_json

def kernels(x,ratios,factors):
    return np.array([np.column_stack((x,r[:,None]*x))@f@np.column_stack((x,r[:,None]*x)).T-np.diag(r) for r,f in zip(ratios,factors)])

def quadratic_draws(x,ratios,factors,g):
    values=[]
    for r,f in zip(ratios,factors):
        v=np.column_stack((x,r[:,None]*x));u=g@v
        values.append(np.sum((u@f)*u,axis=1)-(g*g)@r)
    return np.array(values)

def exact_covariance(c,y,s):
    u=c@y;weighted=c*np.sqrt(s[:,None]*s[None,:])
    influence=4*(u*s)@u.T
    return influence-2*np.einsum('tij,sij->ts',weighted,weighted),influence

def describe(c):
    values,vectors=np.linalg.eigh(c);scale=max(1e-30,float(np.max(np.abs(np.diag(c)))))
    v=np.array([1.,1.,2.]);primary=[float(c[0,0]),float(c[1,1]),float(v@c@v)]
    return {'eigenvalues':values.tolist(),'minimum_eigenvalue_over_scale':float(values[0]/scale),
        'minimum_eigenvector':vectors[:,0].tolist(),'diagonal':np.diag(c).tolist(),
        'primary_marginal_variances':primary,'all_primary_marginals_positive':all(q>0 for q in primary),
        'dense_registered_psd_pass':bool(np.all(np.diag(c)>0) and values[0]>=-1e-8*scale),
        'worker_firm_principal_psd_pass':bool(np.all(np.diag(c)[:2]>0) and np.linalg.eigvalsh(c[:2,:2])[0]>=-1e-8*max(1e-30,float(np.max(np.abs(np.diag(c)[:2])))))}

def audit(campaign,build,out):
    result_path=campaign/'result.json'
    result=json.loads(result_path.read_text());manifest=json.loads((campaign/'manifest.json').read_text())
    assert result['profile']=='confirmation' and result['counts']['native_calls']==50000
    assert result['manifest_sha256']==sha(campaign/'manifest.json')
    confirmation.check_sources(manifest,build/'confirmation')
    cell='dominant_common_t8';k=16
    failed=[r for r in result['failed_calls'] if (r['cell'],r['k'])==(cell,k)]
    assert failed and all(r['phase']=='component_inference_psd' for r in failed)
    reps=sorted(r['replication'] for r in failed);assert len(reps)==len(set(reps))
    check=next(i for i in range(2500) if i not in reps)
    out.mkdir(parents=True,exist_ok=False)
    generated=(build/'confirmation.rs').read_text();assert generated.count('fn main()')==1
    source=out/'export.rs'
    source.write_text(generated.replace('fn main()','fn confirmation_main()',1)+'\n'+(HERE/'export.rs').read_text())
    libraries=list((ROOT/'rust/target/release/deps').glob('libvckss_core-*.rlib'));assert len(libraries)==1
    compiler=Path('/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin/rustc')
    subprocess.run([str(compiler),'--edition=2021','-O','-Awarnings',str(source),'--extern',f'vckss_core={libraries[0]}','-L',f'dependency={libraries[0].parent}','-o',str(out/'export')],check=True)
    plan={'status':'EXISTING_DRAW_DIAGNOSTIC_NOT_CONFIRMATION','confirmation_manifest_sha256':sha(campaign/'manifest.json'),
        'confirmation_result_sha256':sha(result_path),'cell':cell,'k':k,'failed_replications':reps,'check_replication':check,
        'master':manifest['protocol']['rng']['confirmation_master'],'numseed':manifest['protocol']['numerics']['numerical_seed'],
        'source_sha256':{str(p.relative_to(ROOT)):sha(p) for p in (HERE/'export.rs',HERE/'run.py')},
        'generated_sha256':sha(source),'binary_sha256':sha(out/'export'),
        'scope':'All rejected heavy-tailed dominant draws, no selection, no new outcomes or probes; compare the original 1000 probes with exact traces. Native failures and gates remain unchanged. Positive marginal variances do not establish valid q1 intervals. True variances replace inputs in the same realized-y covariance estimator; this is not the population sampling covariance.'}
    write_json(out/'manifest.json',plan)
    command=[str(out/'export'),cell,str(k),str(plan['master']),str(plan['numseed']),str(check),','.join(map(str,reps))]
    with (out/'export.jsonl').open('x') as stream,(out/'stderr.log').open('x') as err:subprocess.run(command,stdout=stream,stderr=err,check=True)
    rows=[json.loads(line) for line in (out/'export.jsonl').read_text().splitlines()];confirmation.previous.finite(rows)
    a=rows[0];outcomes=rows[1:];assert a['kind']=='geometry' and [r['replication'] for r in outcomes]==reps
    assert confirmation.geometry_hash(a)==manifest['preflight']['fixtures'][f'{cell}/{k}']['geometry_sha256']
    for r in outcomes:assert r['seed']==confirmation.previous.old.semantic_seed(plan['master'],cell,k,r['replication'])
    n=a['n'];p=a['p'];terms=a['terms'];x=np.array(a['x']).reshape(n,p);inv=np.array(a['inverse']).reshape(p,p)
    hat=x@inv@x.T;z=np.array(a['z']).reshape(n,terms);gram=np.array(a['gram']).reshape(terms,terms)
    h=np.array(a['h']);ratios=np.array(a['b'])*np.array(a['maker']);factors=np.array(a['factors']).reshape(4,2*p,2*p)[:3]
    c=kernels(x,ratios,factors);exact_c=kernels(x,np.array(a['exact_ratios'])[:3],factors)
    gaussian=np.array(a['gaussian']).reshape(1000,n);truth=np.array(a['variance_true'])
    def fitted(y):
        e=y-hat@y;raw=z@np.linalg.solve(gram,z.T@(e*e));floor=1e-8*np.median(e*e/(1-h))
        return np.maximum(raw,floor),int(np.sum(raw<floor))
    def matrices(y,s):
        exact,influence=exact_covariance(c,y,s)
        q=quadratic_draws(x,ratios,factors,gaussian*np.sqrt(s))
        return influence-np.cov(q,ddof=1),exact,exact_covariance(exact_c,y,s)[0],exact_covariance(c,y,truth)[0]
    y0=np.array(a['y0']);f0,_=fitted(y0);fit_error=float(np.max(np.abs(f0-a['fit0'])))
    assert fit_error<1e-8,fit_error
    noisy0,*_=matrices(y0,f0);native0=np.array(a['covariance0']).reshape(4,4)[:3,:3]
    covariance_error=float(np.max(np.abs(noisy0-native0)));assert covariance_error<1e-8,covariance_error
    output=[]
    for r in outcomes:
        y=np.array(r['y']);s,floored=fitted(y);mat=matrices(y,s)
        names=('original_1000_probes','exact_trace_native_kernel','exact_trace_exact_kernel','true_variance_exact_trace')
        d={name:describe(value) for name,value in zip(names,mat)}
        if d[names[0]]['dense_registered_psd_pass']:raise ValueError('independent replay did not reproduce original rejection')
        output.append({'replication':r['replication'],'semantic_seed':r['seed'],'floored_predictions':floored,**d})
    summary={'status':'DIAGNOSTIC_COMPLETE_NO_REPLACEMENT','diagnostic_manifest_sha256':sha(out/'manifest.json'),
        'export_sha256':sha(out/'export.jsonl'),'rejected_draws':len(reps),'fit_reconstruction_max_error':fit_error,
        'covariance_reconstruction_max_error':covariance_error,'psd_pass_counts':{name:sum(r[name]['dense_registered_psd_pass'] for r in output) for name in names},
        'all_primary_marginals_positive_counts':{name:sum(r[name]['all_primary_marginals_positive'] for r in output) for name in names},
        'worker_firm_principal_psd_pass_counts':{name:sum(r[name]['worker_firm_principal_psd_pass'] for r in output) for name in names},
        'draws_with_floored_predictions':sum(r['floored_predictions']>0 for r in output),'draws':output,
        'limitation':'Descriptive existing-draw audit, not replacement inference or a target-specific q1 validity test. Full confirmation remains under its original gates.'}
    write_json(out/'result.json',summary)
    print(json.dumps({k:v for k,v in summary.items() if k!='draws'},indent=2))

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('campaign',type=Path);parser.add_argument('build',type=Path);parser.add_argument('out',type=Path)
    args=parser.parse_args();audit(args.campaign.resolve(),args.build.resolve(),args.out.resolve())
