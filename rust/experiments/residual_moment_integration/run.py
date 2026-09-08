"""Source-bound local native development campaign; no confirmation promotion."""
from pathlib import Path
import argparse
import collections
import concurrent.futures
import hashlib
import importlib.util
import json
import math
import os
import subprocess
import tarfile
import time
import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PROTOCOL = ROOT/'fevc/docs/observation_residual_moments_integration_v1.json'
PLAN = json.loads(PROTOCOL.read_text())['development']
spec = importlib.util.spec_from_file_location('followup', HERE.parent/'residual_moment_followup/run.py')
old = importlib.util.module_from_spec(spec)
spec.loader.exec_module(old)
TARGETS = old.TARGETS
ARMS = ('native', 'exact_same_variance')

def finite(value):
    if isinstance(value, float) and not math.isfinite(value):
        raise ValueError('nonfinite output')
    if isinstance(value, dict):
        for v in value.values(): finite(v)
    if isinstance(value, list):
        for v in value: finite(v)

def validate(rows, task):
    finite(rows)
    if any(r.get('kind') not in ('target','call','design') for r in rows):
        raise ValueError('unknown record kind')
    designs = [r for r in rows if r['kind']=='design']
    if len(designs)!=1: raise ValueError('design inventory')
    d=designs[0]
    if (d['cell'],d['k'])!=(task['cell'],task['k']): raise ValueError('wrong design')
    if d['n']<=d['p'] or not 0<d['max_h']<1 or d['min_variance']<=0: raise ValueError('invalid design')
    if len(d['truth'])!=4 or len(d['leading_share'])!=4 or len(d['remainder_share'])!=4: raise ValueError('design dimensions')
    if task['cell'].startswith('dominant') and not d['leading_share'][1]>.5: raise ValueError('dominant fixture not realized')
    if task['cell'].startswith('diffuse') and not d['leading_share'][1]<.25: raise ValueError('diffuse fixture not realized')
    reps=set(range(task['start'],task['start']+task['reps']))
    calls=[r for r in rows if r['kind']=='call']
    if len(calls)!=len(reps) or {r['replication'] for r in calls}!=reps: raise ValueError('call inventory')
    calls={r['replication']:r for r in calls}
    geometry=None
    for r in calls.values():
        if r['seconds']<0: raise ValueError('invalid time')
        if r['status']!='success':
            if not r.get('phase') or not r.get('detail') or not r['status'].isupper(): raise ValueError('unclassified failure')
            continue
        if r['projection_count']!=512 or r['gram_atoms']!=512*d['n'] or r['gram_words']!=1024*d['n']: raise ValueError('projection inventory')
        if not 0<=r['projection_residual']<=r['projection_gate'] or not 0<=r['full_residual']<=r['full_gate']: raise ValueError('uncertified projection')
        terms=3 if task['cell']=='dominant_leverage' else 15
        if len(r['h'])!=d['n'] or len(r['b'])!=3 or any(len(b)!=d['n'] for b in r['b']) or len(r['gram'])!=terms**2: raise ValueError('geometry dimensions')
        g=(r['h'],r['b'],r['gram'])
        if geometry is not None and geometry!=g: raise ValueError('outcome-dependent geometry')
        geometry=g
        if r['words']!=2*r['atoms'] or r['atoms']<d['n']*(1000+128+2+512): raise ValueError('Counter accounting')
        if r['rcond']<=0 or r['memory']<=0 or not 0<=r['floored']<=d['n']: raise ValueError('invalid fit diagnostics')
    expected={(rep,arm,target) for rep in reps for arm in ARMS for target in TARGETS}
    seen=set()
    for r in [r for r in rows if r['kind']=='target']:
        key=(r['replication'],r['arm'],r['target'])
        if key in seen: raise ValueError('duplicate target')
        seen.add(key)
        if (r['cell'],r['k'],r['numseed'])!=(task['cell'],task['k'],task['numseed']): raise ValueError('wrong task identity')
        if r['seed']!=old.semantic_seed(task['master'],r['cell'],r['k'],r['replication']): raise ValueError('wrong semantic seed')
        if (calls[r['replication']]['status']!='success')!=(r['status']=='native_call_failed'): raise ValueError('inconsistent call failure')
        if r['status'] not in ('success','native_call_failed','q1_target_failed','q0_variance_failed','q1_covariance_failed'): raise ValueError('unknown target status')
        if r['status']=='success':
            for field in ('point_error','variance','estimated_sd','width','covered','lower_miss','upper_miss'):
                if field not in r: raise ValueError('partial success')
            if r['variance']<=0 or r['width']<=0 or r['estimated_sd']<=0: raise ValueError('invalid interval')
            if not math.isclose(r['estimated_sd']**2,r['variance'],rel_tol=1e-12): raise ValueError('inconsistent SE')
            if any(type(r[x]) is not bool for x in ('covered','lower_miss','upper_miss')) or sum(r[x] for x in ('covered','lower_miss','upper_miss'))!=1: raise ValueError('invalid coverage')
        if r['arm']=='native' and r['status']!='native_call_failed':
            if abs(r['point_kernel_identity'])>1e-8: raise ValueError('point/kernel identity')
            if 'remainder_identity' in r and abs(r['remainder_identity'])>1e-8: raise ValueError('q1 identity')
    if seen!=expected: raise ValueError('target inventory')
    return d,calls,[r for r in rows if r['kind']=='target']

def summary(targets):
    groups=collections.defaultdict(list)
    for r in targets: groups[(r['cell'],r['numseed'],r['arm'],r['target'])].append(r)
    out=[]
    for key,rs in sorted(groups.items()):
        ok=[r for r in rs if r['status']=='success']
        a=dict(zip(('cell','numseed','arm','target'),key),attempted=len(rs),success=len(ok),failures=dict(collections.Counter(r['status'] for r in rs if r['status']!='success')),
            coverage_all=sum(r.get('covered',False) for r in ok)/len(rs),coverage_success=sum(r['covered'] for r in ok)/len(ok) if ok else None)
        if len(ok)>1:
            a.update(empirical_sd=float(np.std([r['point_error'] for r in ok],ddof=1)),mean_se=float(np.mean([r['estimated_sd'] for r in ok])))
            a['sd_to_mean_se']=a['empirical_sd']/a['mean_se']
        if key[2]=='native':
            numeric=[r for r in rs if 'point_delta' in r]
            if numeric:
                for field in ('point_delta','covariance_delta','critical_delta'):
                    values=[r[field] for r in numeric if field in r]
                    if values:a[field]={'mean':float(np.mean(values)),'rms':float(np.sqrt(np.mean(np.square(values)))),'max_absolute':max(map(abs,values))}
                scaled=[r['covariance_delta']/r['trace_mcse'] for r in numeric if r.get('trace_mcse',0)>0]
                if scaled:a['covariance_error_in_mcse']={'mean':float(np.mean(scaled)),'rms':float(np.sqrt(np.mean(np.square(scaled)))),'max_absolute':max(map(abs,scaled))}
                scaled=[r['point_delta']/r['point_mcse'] for r in numeric if r.get('point_mcse',0)>0 and r['target']!='total']
                if scaled:a['point_error_in_mcse']={'mean':float(np.mean(scaled)),'rms':float(np.sqrt(np.mean(np.square(scaled)))),'max_absolute':max(map(abs,scaled))}
                if 'empirical_sd' in a:a['point_noise_to_outcome_sd']=a['point_delta']['rms']/a['empirical_sd']
        out.append(a)
    return out

def task_run(exe, out, task):
    out.mkdir(parents=True,exist_ok=False)
    rows=old.capture(exe,[task[q] for q in ('cell','k','start','reps','master','numseed')],out/'rows.jsonl')
    d,calls,targets=validate(rows,task)
    receipt={'task':task,'binary_sha256':old.sha(exe),'output_sha256':old.sha(out/'rows.jsonl'),'design':d,
        'native_calls':len(calls),'native_success':sum(r['status']=='success' for r in calls.values()),
        'call_failures':dict(collections.Counter((r['status']+':'+r['phase']) for r in calls.values() if r['status']!='success')),
        'native_seconds':sum(r['seconds'] for r in calls.values()),'target_attempts':len(targets),'summaries':summary(targets)}
    old.write_json(out/'receipt.json',receipt)
    return receipt

def freeze(out,exe,tasks):
    paths=[PROTOCOL,ROOT/'rust/Cargo.toml',ROOT/'rust/Cargo.lock']
    paths+=list((ROOT/'rust/crates/vckss-core').rglob('*.rs'))
    paths+=list(HERE.glob('*.py'))+list(HERE.glob('*.rs'))
    paths+=list((HERE.parent/'residual_moment_followup').glob('*.py'))+list((HERE.parent/'residual_moment_followup').glob('*.rs'))
    paths=sorted(set(paths))
    bundle=out/'source.tar.gz'
    with tarfile.open(bundle,'x:gz') as archive:
        for path in paths: archive.add(path,arcname=str(path.relative_to(ROOT)))
        archive.add(exe.parent/'integration.rs',arcname='generated/integration.rs')
    manifest={'schema':'FEVC-INTERNAL-NATIVE-DEVELOPMENT-V1','profile':'development','protocol_sha256':old.sha(PROTOCOL),
        'source_bundle_sha256':old.sha(bundle),'binary_sha256':old.sha(exe),'generated_sha256':old.sha(exe.parent/'integration.rs'),
        'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'sources':{str(p.relative_to(ROOT)):old.sha(p) for p in paths},'tasks':tasks,'expected_native_calls':sum(t['reps'] for t in tasks),'expected_target_attempts':8*sum(t['reps'] for t in tasks)}
    old.write_json(out/'manifest.json',manifest)
    return manifest

def run(exe,out,tiny=False):
    out.mkdir(parents=True,exist_ok=False)
    tasks=[{'cell':cell,'k':PLAN['k'],'start':0,'reps':1 if tiny else PLAN['replications'],
        'master':PLAN['outcome_master'],'numseed':seed} for cell in (PLAN['cells'][:1] if tiny else PLAN['cells']) for seed in (PLAN['numerical_seeds'][:1] if tiny else PLAN['numerical_seeds'])]
    manifest=freeze(out,exe,tasks)
    def work(t):
        for path,digest in manifest['sources'].items():
            if old.sha(ROOT/path)!=digest: raise ValueError('source changed after registration')
        return task_run(exe,out/f'{t["cell"]}-{t["numseed"]}',t)
    now=time.monotonic()
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        receipts=[]
        for future in concurrent.futures.as_completed([pool.submit(work,t) for t in tasks]):
            r=future.result();receipts.append(r)
            print(json.dumps({'task':r['task'],'calls':r['native_calls'],'success':r['native_success'],'failures':r['call_failures']}),flush=True)
    result={'status':'development_complete_not_confirmation','manifest_sha256':old.sha(out/'manifest.json'),'seconds':time.monotonic()-now,'native_calls':sum(r['native_calls'] for r in receipts),'native_success':sum(r['native_success'] for r in receipts),'target_attempts':sum(r['target_attempts'] for r in receipts),'tasks':sorted(receipts,key=lambda r:(r['task']['cell'],r['task']['numseed']))}
    if result['native_calls']!=manifest['expected_native_calls'] or result['target_attempts']!=manifest['expected_target_attempts']: raise ValueError('campaign inventory')
    old.write_json(out/'result.json',result)
    return result

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('exe',type=Path);p.add_argument('out',type=Path);p.add_argument('--tiny',action='store_true');a=p.parse_args()
    run(a.exe.resolve(),a.out.resolve(),a.tiny)
