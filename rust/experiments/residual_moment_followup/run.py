"""Local, atomic, source-bound residual-moment follow-up experiments."""
from pathlib import Path
import argparse
import collections
import hashlib
import importlib.util
import json
import math
import os
import platform
import subprocess
import tarfile
import time
import numpy as np

ROOT = Path(__file__).resolve().parents[3]
PLAN_PATH = ROOT/'fevc/docs/observation_residual_moments_followup_v1.json'
PLAN = json.loads(PLAN_PATH.read_text())
TARGETS = ('worker', 'firm', 'covariance', 'total')
ARMS = {'development': PLAN['development']['arms'], 'paired': PLAN['same_design']['arms'], 'confirmation': PLAN['confirmation']['arms']}
spec = importlib.util.spec_from_file_location('original_gates', ROOT/'fevc/tools/run_structured_inference_confirmation.py')
gates = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gates)

def sha(path):
    return hashlib.file_digest(Path(path).open('rb'), 'sha256').hexdigest()

def write_json(path, value):
    payload = json.dumps(value, indent=2, allow_nan=False)+'\n'
    with Path(path).open('x') as stream:
        stream.write(payload)

def capture(exe, args, path):
    path = Path(path)
    if path.exists():
        raise ValueError(f'refusing to overwrite {path}')
    partial = path.with_suffix(path.suffix+'.partial')
    with partial.open('x') as stream, path.with_suffix(path.suffix+'.stderr').open('x') as err:
        subprocess.run([str(exe), *map(str,args)], stdout=stream, stderr=err, check=True)
    os.replace(partial, path)
    return [json.loads(line) for line in path.read_text().splitlines()]

def geometry(exe, directory, cell, k, probes, seed):
    directory.mkdir(parents=True, exist_ok=True)
    path=directory/f'{cell}-{k}-{probes}-{seed}.jsonl'
    rows=capture(exe, ['geometry',cell,k,probes,seed],path)
    assert len(rows)==1
    d=rows[0]
    assert (d['cell'],d['k'],d['probes'],d['seed'])==(cell,k,probes,seed)
    binary=path.with_suffix('.bin')
    if d['status']=='success':
        a=np.asarray([d['h'],*d['b']],dtype='<f8')
        assert a.shape==(4,d['n']) and np.isfinite(a).all()
        assert d['full_residual']<=d['full_residual_gate']
        exact=np.asarray([d['exact_h'],*d['exact_b']])
        diagnostics={'relative_rmse':(np.linalg.norm(a-exact,axis=1)/np.linalg.norm(exact,axis=1)).tolist(),
                     'maximum_leverage_absolute_error':float(np.max(np.abs(a[0]-exact[0]))),
                     'h_min':float(a[0].min()),'h_max':float(a[0].max())}
    else:
        a=np.full((4,d['n']),np.nan,dtype='<f8')
        diagnostics={'capture_failure':d['status'],'detail':d['detail']}
    with binary.open('xb') as stream:
        stream.write(a.tobytes())
    return {'path':str(path),'sha256':sha(path),'binary':str(binary),'binary_sha256':sha(binary),
            **{q:d[q] for q in ('cell','k','n','probes','seed','status','seconds')},**diagnostics}

def semantic_seed(master, cell, k, rep):
    # Independent translation of the frozen oracle's semantic addressing.
    def mix(x):
        x=(x+0x9e3779b97f4a7c15)&((1<<64)-1)
        x=((x^(x>>30))*0xbf58476d1ce4e5b9)&((1<<64)-1)
        x=((x^(x>>27))*0x94d049bb133111eb)&((1<<64)-1)
        return x^(x>>31)
    # Checked against the source below in tests; never a task-local RNG state.
    tag=0xcbf29ce484222325
    for byte in cell.encode():
        tag=((tag^byte)*0x1000000001b3)&((1<<64)-1)
    return mix(master ^ mix(tag) ^ mix(k) ^ mix(rep))

def validate(rows, task):
    if any(r.get('kind') not in ('target','prepare','design') for r in rows):raise ValueError('unknown record kind')
    targets=[r for r in rows if r.get('kind')=='target']
    expected={(rep,arm,target) for rep in range(task['start'],task['start']+task['reps']) for arm in ARMS[task['mode']] for target in TARGETS}
    seen=set()
    for r in targets:
        key=(r['replication'],r['arm'],r['target'])
        if key in seen: raise ValueError('duplicate target key')
        seen.add(key)
        if (r['cell'],r['k'])!=(task['cell'],task['k']): raise ValueError('wrong cell identity')
        for field in ('gate','reference'):
            if field in task and r[field]!=task[field]:raise ValueError('wrong registered scientific role')
        if r['seed']!=semantic_seed(task['master'],r['cell'],r['k'],r['replication']): raise ValueError('wrong semantic seed')
        if r['status'] not in ('success','variance_fit_failed','q0_variance_failed','q1_covariance_failed'): raise ValueError('unknown status')
        for key2,v in r.items():
            if isinstance(v,float) and not math.isfinite(v): raise ValueError('nonfinite output')
        if r['status']=='success':
            for field in ('point_error','variance','estimated_sd','width','covered','lower_miss','upper_miss'):
                if field not in r: raise ValueError('partial success')
            if r['variance']<=0 or r['estimated_sd']<=0 or r['width']<=0: raise ValueError('invalid success scale')
            if any(type(r[x]) is not bool for x in ('covered','lower_miss','upper_miss')): raise ValueError('invalid coverage indicator')
            if sum(r[x] for x in ('covered','lower_miss','upper_miss'))!=1: raise ValueError('inconsistent coverage')
            if not math.isclose(r['estimated_sd']**2,r['variance'],rel_tol=1e-12): raise ValueError('inconsistent se')
    if seen!=expected: raise ValueError('partial target inventory')
    designs=[r for r in rows if r.get('kind')=='design']
    if len(designs)!=1 or designs[0]['cell']!=task['cell'] or designs[0]['k']!=task['k']: raise ValueError('design inventory')
    if not (0<=designs[0]['max_h']<1 and designs[0]['min_variance']>0): raise ValueError('unidentified or nonpositive design')
    return targets

def summaries(rows):
    groups=collections.defaultdict(list)
    for r in rows:
        if r.get('kind')=='target':groups[(r['cell'],r['k'],r['arm'],r['target'])].append(r)
    result=[]
    for (cell,k,arm,target),rs in sorted(groups.items()):
        # Original helper needs some diagnostic fields absent in this oracle.
        adapted=[{**{q:0.0 for q in gates.NUMERIC_FIELDS},**r} for r in rs]
        a=gates._summary(adapted,len(rs))
        for field in gates.NUMERIC_FIELDS[2:]:
            if field not in rs[0]:a[f'mean_{field}']=None
        a.update(cell=cell,k=k,arm=arm,target=target,gate=rs[0]['gate'],reference=rs[0]['reference'],
                 coverage_all=sum(r.get('covered',False) for r in rs if r['status']=='success')/len(rs),
                 mean_leading_share=rs[0]['leading_share'],mean_remainder_share=rs[0]['remainder_share'])
        result.append(a)
    return result

def task_run(exe, directory, task):
    directory.mkdir(parents=True,exist_ok=True)
    start=time.monotonic()
    rows=capture(exe,['run',task['cell'],task['k'],task['start'],task['reps'],task['master'],task['numseed'],task['mode'],task['input'],'v1'],directory/'rows.jsonl')
    targets=validate(rows,task)
    result={'task':task,'seconds':time.monotonic()-start,'output_sha256':sha(directory/'rows.jsonl'),
            'binary_sha256':sha(exe),'target_rows':len(targets),'design':[r for r in rows if r['kind']=='design'][0],
            'preparation':[r for r in rows if r['kind']=='prepare'],'summaries':summaries(rows)}
    write_json(directory/'receipt.json',result)
    return result

def development(exe,out):
    p=PLAN['development'];results=[];geometries=[]
    for cell in p['cells']:
        for probes in p['jla_probes']:
            for seed in p['numerical_seeds']:
                g=geometry(exe,out/'geometry',cell,p['k'],probes,seed);geometries.append(g)
                t={'cell':cell,'k':p['k'],'mode':'development','reps':p['replications'],'start':0,'master':p['outcome_master'],'numseed':seed,'input':g['binary']}
                r=task_run(exe,out/'tasks'/f'{cell}-{probes}-{seed}',t);results.append(r)
                print(json.dumps({'stage':'development','cell':cell,'probes':probes,'seed':seed,'geometry':g['status'],
                    'firm':[{q:s[q] for q in ('arm','coverage_all','se_ratio','success_rate')} for s in r['summaries'] if s['target']=='firm']}),flush=True)
    scales=[]
    for cell in p['scaling']['cells']:
        for k in p['scaling']['k']:
            g=geometry(exe,out/'scaling',cell,k,p['scaling']['jla_probes'],p['scaling']['numerical_seed'])
            r=capture(exe,['scale',cell,k,p['scaling']['numerical_seed']],out/'scaling'/f'fit-{cell}-{k}.jsonl')[0]
            scales.append({'geometry':g,'candidate':r});print(json.dumps({'stage':'scaling',**r}),flush=True)
    write_json(out/'development-result.json',{'protocol_sha256':sha(PLAN_PATH),'geometries':geometries,'tasks':results,'scaling':scales})

def paired(exe,out):
    p=PLAN['same_design'];results=[]
    for cell in p['cells']:
        for k in p['k']:
            t={'cell':cell,'k':k,'mode':'paired','reps':p['replications'],'start':0,'master':p['outcome_master'],'numseed':p['numerical_seed'],'input':'-'}
            r=task_run(exe,out/'tasks'/f'{cell}-{k}',t);results.append(r)
            print(json.dumps({'stage':'paired','cell':cell,'k':k,'firm':[{q:s[q] for q in ('arm','coverage_all','se_ratio','success_rate')} for s in r['summaries'] if s['target']=='firm']}),flush=True)
    write_json(out/'paired-result.json',{'protocol_sha256':sha(PLAN_PATH),'tasks':results})

def freeze(exe,out):
    p=PLAN['confirmation'];tasks=[];geo=[];preflight=[]
    cells=[json.loads(x) for x in subprocess.check_output([str(exe),'cells'],text=True).splitlines()]
    assert len(cells)==20
    for cell in cells:
        g=geometry(exe,out/'geometry',cell['cell'],cell['k'],p['jla_probes'],p['numerical_seed']);geo.append(g)
        path=out/'geometry'/f'preflight-{cell["cell"]}-{cell["k"]}.jsonl'
        preflight+=capture(exe,['preflight',cell['cell'],cell['k'],p['numerical_seed'],'confirmation',g['binary'],'v1'],path)
        tasks.append({**cell,'mode':'confirmation','reps':p['replications'],'start':0,'master':p['outcome_master'],'numseed':p['numerical_seed'],'input':g['binary'],'input_sha256':g['binary_sha256'],'id':f'{cell["cell"]}-{cell["k"]}'})
    files=set((ROOT/'rust/crates/vckss-core/src').rglob('*.rs'))
    files.update((ROOT/'rust/vendor/cmg').rglob('*.rs'))
    files.update((ROOT/'rust/experiments/residual_moment_followup').glob('*'))
    files.update((ROOT/'rust').rglob('Cargo.toml'))
    files.update([ROOT/'rust/Cargo.lock',ROOT/'rust/crates/vckss-core/examples/rc_observation_inference.rs',ROOT/'rust/crates/vckss-core/examples/common/q1_reference.rs',ROOT/'fevc/tools/run_structured_inference_confirmation.py',PLAN_PATH,PLAN_PATH.with_name('observation_residual_moments_followup_v1_addendum.json')])
    files.add(PLAN_PATH.with_name('observation_residual_moments_followup_v1_capture_scale.json'))
    files=sorted(f for f in files if f.is_file() and 'target' not in f.parts)
    identities={str(f.relative_to(ROOT)):sha(f) for f in files}
    bundle=out/'source-bundle.tar.gz'
    with tarfile.open(bundle,'x:gz') as tar:
        for f in files:tar.add(f,arcname=str(f.relative_to(ROOT)),recursive=False)
        tar.add(exe,arcname='build/followup',recursive=False)
        tar.add(exe.with_suffix('.rs'),arcname='build/followup.rs',recursive=False)
    manifest={'schema':'FEVC-RESIDUAL-MOMENTS-CONFIRMATION-MANIFEST-V1','created_unix':time.time(),
        'base_commit':PLAN['base_commit'],'source_files':identities,'source_bundle_sha256':sha(bundle),'executable_sha256':sha(exe),
        'protocol':PLAN,'development_addendum':json.loads(PLAN_PATH.with_name('observation_residual_moments_followup_v1_addendum.json').read_text()),
        'capture_scale_addendum':json.loads(PLAN_PATH.with_name('observation_residual_moments_followup_v1_capture_scale.json').read_text()),
        'runtime':{'python':platform.python_version(),'numpy':np.__version__,'platform':platform.platform(),'rust':'1.85.1; release -O; offline locked'},
        'thresholds':gates.THRESHOLDS,'tasks':tasks,'expected_target_rows':400000,'geometry':geo,'preflight':preflight,
        'expected_inventory':[f'tasks/{t["id"]}/{name}' for t in tasks for name in ('rows.jsonl','receipt.json')]}
    write_json(out/'manifest.json',manifest)
    print(json.dumps({'stage':'frozen','manifest_sha256':sha(out/'manifest.json'),'bundle_sha256':sha(bundle),'tasks':len(tasks)}),flush=True)

def confirmation(exe,out):
    m=json.loads((out/'manifest.json').read_text())
    assert sha(exe)==m['executable_sha256'] and sha(out/'source-bundle.tar.gz')==m['source_bundle_sha256']
    for g in m['geometry']:assert sha(g['path'])==g['sha256'] and sha(g['binary'])==g['binary_sha256']
    for path,digest in m['source_files'].items():assert sha(ROOT/path)==digest,f'changed frozen source {path}'
    results=[]
    for t in m['tasks']:
        assert sha(t['input'])==t['input_sha256']
        r=task_run(exe,out/'tasks'/t['id'],t);results.append(r)
        print(json.dumps({'stage':'confirmation','cell':t['cell'],'k':t['k'],'seconds':r['seconds'],'firm':[{q:s[q] for q in ('arm','coverage_all','se_ratio','success_rate')} for s in r['summaries'] if s['target']=='firm']}),flush=True)
    assert sum(r['target_rows'] for r in results)==m['expected_target_rows']
    ss=[s for r in results for s in r['summaries']]
    reports={}
    for arm in ARMS['confirmation']:
        selected=[s for s in ss if s['arm']==arm]
        failures,stress=gates._scientific_failures(selected,'confirmation')
        reports[arm]={'status':'FAIL' if failures else 'PASS','failures':failures,'severe_stress':stress,'summaries':selected}
    write_json(out/'confirmation-result.json',{'schema':'FEVC-RESIDUAL-MOMENTS-CONFIRMATION-RESULT-V1','manifest_sha256':sha(out/'manifest.json'),
        'expected_target_rows':m['expected_target_rows'],'actual_target_rows':sum(r['target_rows'] for r in results),'arms':reports,
        'failed_cells':[s for s in ss if s['failure_counts']],'receipts':{r['task']['id']:sha(out/'tasks'/r['task']['id']/'receipt.json') for r in results}})
    print(json.dumps({'stage':'confirmation_complete','arms':{a:{k:v[k] for k in ('status','failures')} for a,v in reports.items()}}),flush=True)

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('stage',choices=('development','paired','freeze','confirmation'));parser.add_argument('output');parser.add_argument('--executable',required=True)
    args=parser.parse_args();out=Path(args.output).resolve();out.mkdir(parents=True,exist_ok=True)
    globals()[args.stage](Path(args.executable).resolve(),out)
