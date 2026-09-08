"""Frozen local confirmation with atomic gzip tasks and complete failure audit."""
from pathlib import Path
import argparse
import collections
import concurrent.futures
import gzip
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

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
PROTOCOL=ROOT/'fevc/docs/observation_residual_moments_confirmation_v1.json'
PLAN=json.loads(PROTOCOL.read_text())
MATRIX_PATH=ROOT/PLAN['matrix_source'].split(':')[0]
CELLS=json.loads(MATRIX_PATH.read_text())['cells']
CELL_MAP={(c['cell'],c['k']):c for c in CELLS}
spec=importlib.util.spec_from_file_location('integration',HERE.parent/'residual_moment_integration/run.py')
previous=importlib.util.module_from_spec(spec);spec.loader.exec_module(previous)
gates=previous.old.gates
TARGETS=previous.TARGETS
ARMS=previous.ARMS
SCHEMA='FEVC-NATIVE-RESIDUAL-CONFIRMATION-TASK-V1'
NUMSEED=PLAN['numerics']['numerical_seed']
ERROR_CODES=set(__import__('re').findall(r'=> "([A-Z_]+)"',(ROOT/'rust/crates/vckss-core/src/error.rs').read_text()))-{'OK'}
sha=previous.old.sha
write_json=previous.old.write_json

def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':'),allow_nan=False).encode()).hexdigest()

def geometry_hash(row):
    return digest({q:row[q] for q in ('h','b','gram')})

def read_rows(path):
    with gzip.open(path,'rt') as stream:
        return [json.loads(line) for line in stream]

def preflight(exe,out):
    out.mkdir(parents=True,exist_ok=False)
    path=out/'rows.jsonl'
    with path.open('x') as stream,(out/'stderr.log').open('x') as err:
        subprocess.run([str(exe),'preflight'],stdout=stream,stderr=err,check=True)
    rows=[json.loads(line) for line in path.read_text().splitlines()]
    previous.finite(rows)
    if len(rows)!=20 or {(r['cell'],r['k']) for r in rows}!=set(CELL_MAP):raise ValueError('preflight inventory')
    spectral={};fixtures={}
    for r in rows:
        c=CELL_MAP[r['cell'],r['k']];n=r['n'];p=r['p'];terms=3 if c['variance_model']=='structured_leverage' else 15
        if r['kind']!='preflight' or r['terms']!=terms or p!=2*c['k']-1+2*int(c['controls']) or n<=p:raise ValueError('preflight dimensions')
        if r['key_contract']!='FEVC-OBSERVATION-OUTCOME-FREE-KEY-V1' or r['maker_minimum']<=0 or r['rcond']<=0:raise ValueError('preflight identification')
        s=np.array(r['variance']);h=np.array(r['h']);z=np.array(r['z']).reshape(n,terms);ez=np.array(r['exact_z']).reshape(n,terms)
        gram=np.array(r['gram']).reshape(terms,terms)
        if not np.all(s>0) or not np.all((h>=0)&(h<1)) or len(r['b'])!=3 or any(len(b)!=n for b in r['b']):raise ValueError('preflight positivity')
        errors=[];conditions=[]
        for basis in (ez,z):
            sv=np.linalg.svd(basis,compute_uv=False)
            if sv[-1]/sv[0]<1e-10:raise ValueError('preflight basis rank')
            conditions.append(float(sv[-1]/sv[0]));errors.append(float(np.linalg.norm(s-basis@np.linalg.lstsq(basis,s,rcond=None)[0])/np.linalg.norm(s)))
        if c['gate']=='correct' and errors[0]>1e-8:raise ValueError('correct variance DGP outside original basis')
        diagonal=np.sqrt(np.diag(gram));normalized=gram/diagonal[:,None]/diagonal[None,:]
        if np.linalg.eigvalsh(normalized)[0]<=0:raise ValueError('preflight Gram not positive definite')
        if c['beta']=='zero' and max(map(abs,r['truth']))>1e-12:raise ValueError('null fixture not null')
        if c['dominant']:
            if any(v<=.75 for v in r['leading_share'][:2]) or any(v>=.15 for v in r['remainder_share'][:2]):raise ValueError('one-mode fixture regime')
            if r['leading_share'][2]<=.95 or r['remainder_share'][2]<=.50:raise ValueError('multimode covariance fixture regime')
        elif any(v>=.11 for v in r['leading_share'][:2]):raise ValueError('diffuse fixture regime')
        for i,target in enumerate(TARGETS):
            spectral[c['cell'],c['k'],target]={'mean_leading_share':r['leading_share'][i],'mean_remainder_share':r['remainder_share'][i]}
        fixtures[f'{c["cell"]}/{c["k"]}']={q:r[q] for q in ('cell','k','n','p','terms','truth','leading_share','remainder_share','maker_minimum','rcond')}
        fixtures[f'{c["cell"]}/{c["k"]}'].update(geometry_sha256=geometry_hash(r),variance_minimum=float(s.min()),exact_basis_relative_variance_error=errors[0],native_basis_relative_variance_error=errors[1],basis_singular_ratio=conditions,maximum_leverage_error=float(np.max(np.abs(h-r['exact_h']))))
    failed=gates._spectral_failures(spectral)
    if failed:raise ValueError(f'spectral preflight: {failed}')
    receipt={'status':'outcome_free_preflight_pass','binary_sha256':sha(exe),'protocol_sha256':sha(PROTOCOL),'output_sha256':sha(path),'fixtures':fixtures}
    write_json(out/'receipt.json',receipt)
    return receipt

def verify_shards(exe,out,preflight_receipt):
    out.mkdir(parents=True,exist_ok=False);results=[]
    for name in ('diffuse_leverage','dominant_common_controls'):
        fixture=preflight_receipt['fixtures'][f'{name}/16']
        task=next(t for t in tasks_for('tiny') if t['cell']==name)
        base=out/name/'whole'
        task_run(exe,base,task,fixture,'pipeline-only','pipeline-only')
        pieces=[]
        for rep in (1,0):
            piece={**task,'start':rep,'reps':1,'id':f'{name}-{rep}'};folder=out/name/f'shard-{rep}'
            task_run(exe,folder,piece,fixture,'pipeline-only','pipeline-only')
            pieces+=read_rows(folder/'rows.jsonl.gz')
        def normalized(rows):
            selected=[]
            for row in rows:
                if row['kind'] not in ('call','target'):continue
                row={k:v for k,v in row.items() if k!='seconds'}
                selected.append(row)
            return sorted(selected,key=lambda x:(x['replication'],x['kind'],x.get('arm',''),x.get('target','')))
        whole=normalized(read_rows(base/'rows.jsonl.gz'));split=normalized(pieces)
        if whole!=split:raise ValueError('split/reverse execution changed numerical or outcome records')
        results.append({'cell':name,'sha256':digest(whole),'call_and_target_records':len(whole)})
    write_json(out/'receipt.json',{'status':'reverse_and_split_pipeline_pass','binary_sha256':sha(exe),'cases':results})

def tasks_for(profile):
    p=PLAN['profiles'][profile];master=PLAN['rng']['confirmation_master' if profile=='confirmation' else 'pipeline_master']
    tasks=[]
    for c in CELLS:
        for start in range(0,p['replications_per_cell'],p['shard_size']):
            task=dict(profile=profile,cell=c['cell'],k=c['k'],start=start,reps=min(p['shard_size'],p['replications_per_cell']-start),master=master,numseed=NUMSEED)
            task['id']=f'{c["cell"]}-{c["k"]}-{start:04d}'
            tasks.append(task)
    return tasks

def validate(rows,task,fixture):
    previous.finite(rows)
    if any(r.get('kind') not in ('task','design','call','target') for r in rows):raise ValueError('unknown record kind')
    headers=[r for r in rows if r['kind']=='task']
    expected={'kind':'task','schema':SCHEMA,**{q:task[q] for q in ('profile','cell','k','start','reps','master','numseed')}}
    if headers!=[expected]:raise ValueError('task/profile identity')
    designs=[r for r in rows if r['kind']=='design']
    if len(designs)!=1:raise ValueError('design inventory')
    d=designs[0];c=CELL_MAP[task['cell'],task['k']]
    for q in ('cell','k','n','p','truth','leading_share','remainder_share'):
        if d[q]!=fixture[q]:raise ValueError(f'design mismatch: {q}')
    if d['min_variance']!=fixture['variance_minimum'] or not 0<d['max_h']<1:raise ValueError('variance/support mismatch')
    reps=set(range(task['start'],task['start']+task['reps']))
    call_rows=[r for r in rows if r['kind']=='call']
    if len(call_rows)!=len(reps) or {r['replication'] for r in call_rows}!=reps:raise ValueError('call inventory')
    calls={r['replication']:r for r in call_rows}
    for r in call_rows:
        if r['seconds']<0:raise ValueError('negative runtime')
        if r['status']!='success':
            if r['status'] not in ERROR_CODES or not r.get('phase') or not r.get('detail'):raise ValueError('unclassified native failure')
            continue
        if r['projection_count']!=512 or r['gram_atoms']!=512*d['n'] or r['gram_words']!=1024*d['n']:raise ValueError('Gram Counter/projection inventory')
        if not 0<=r['projection_residual']<=r['projection_gate'] or not 0<=r['full_residual']<=r['full_gate']:raise ValueError('uncertified solve')
        if geometry_hash(r)!=fixture['geometry_sha256']:raise ValueError('outcome-dependent or wrong geometry')
        if r['rcond']<=0 or r['memory']<=0 or not 0<=r['floored']<=d['n']:raise ValueError('invalid fit diagnostic')
    targets=[r for r in rows if r['kind']=='target'];seen=set();by_rep=collections.defaultdict(list)
    for r in targets:
        key=(r['replication'],r['arm'],r['target'])
        if key in seen:raise ValueError('duplicate target')
        seen.add(key)
        if (r['cell'],r['k'],r['numseed'])!=(task['cell'],task['k'],task['numseed']):raise ValueError('target identity')
        if r['replication'] not in reps or r['seed']!=previous.old.semantic_seed(task['master'],task['cell'],task['k'],r['replication']):raise ValueError('semantic outcome seed')
        if (calls[r['replication']]['status']!='success')!=(r['status']=='native_call_failed'):raise ValueError('inconsistent native failure')
        statuses={'native':{'success','native_call_failed','q1_target_failed'},'exact_same_variance':{'success','native_call_failed','q0_variance_failed','q1_covariance_failed'}}
        if r['arm'] not in statuses or r['status'] not in statuses[r['arm']]:raise ValueError('unknown target status')
        if r['status']=='success':
            for q in ('point_error','variance','estimated_sd','width','covered','lower_miss','upper_miss'):
                if q not in r:raise ValueError('partial interval')
            if r['variance']<=0 or r['estimated_sd']<=0 or r['width']<=0 or not math.isclose(r['estimated_sd']**2,r['variance'],rel_tol=1e-12):raise ValueError('invalid interval scale')
            if any(type(r[q]) is not bool for q in ('covered','lower_miss','upper_miss')) or sum(r[q] for q in ('covered','lower_miss','upper_miss'))!=1:raise ValueError('invalid coverage indicator')
        if r['arm']=='native' and r['status']!='native_call_failed':
            if abs(r['point_kernel_identity'])>1e-8 or abs(r.get('remainder_identity',0))>1e-8:raise ValueError('kernel identity')
            if c['reference']=='Q1':
                if r.get('q1_status') not in (0,1,2,3,6) or ((r['status']=='success')!=(r['q1_status']==0)):raise ValueError('q1 status identity')
                if r['status']=='success' and (r['critical']<=0 or r['critical_exact']<=0):raise ValueError('critical value')
            elif 'q1_status' in r or r['status']!='success':raise ValueError('q0 status identity')
            by_rep[r['replication']].append(r)
    expected={(rep,arm,t) for rep in reps for arm in ARMS for t in TARGETS}
    if seen!=expected:raise ValueError('target inventory')
    for rep,r in calls.items():
        if r['status']=='success':
            critical=100000*sum(t['status']=='success' for t in by_rep[rep]) if c['reference']=='Q1' else 0
            atoms=d['n']*(1000+128+2+512)+2*critical
            if r['atoms']!=atoms or r['words']!=2*atoms:raise ValueError('component Counter count')
    return targets,call_rows

def task_run(exe,out,task,fixture,manifest_hash,source_hash):
    out.mkdir(parents=True,exist_ok=False)
    partial=out/'rows.jsonl.gz.partial';final=out/'rows.jsonl.gz'
    command=[str(exe),task['profile'],task['cell'],str(task['k']),str(task['start']),str(task['reps'])]
    with partial.open('xb') as raw,gzip.GzipFile(filename='',mode='wb',fileobj=raw,mtime=0) as gz,(out/'stderr.log').open('xb') as err:
        process=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=err)
        assert process.stdout is not None
        for line in process.stdout:gz.write(line)
        process.stdout.close();rc=process.wait()
    if rc:raise subprocess.CalledProcessError(rc,command)
    targets,calls=validate(read_rows(partial),task,fixture)
    os.replace(partial,final)
    receipt={'schema':SCHEMA,'status':'execution_and_inventory_pass','task':task,'manifest_sha256':manifest_hash,'source_bundle_sha256':source_hash,'binary_sha256':sha(exe),'output_sha256':sha(final),
        'native_calls':len(calls),'target_rows':len(targets),'native_success':sum(c['status']=='success' for c in calls),'call_failures':dict(collections.Counter(c['status']+':'+c['phase'] for c in calls if c['status']!='success')),'native_seconds':sum(c['seconds'] for c in calls)}
    write_json(out/'receipt.json',receipt)
    return receipt

def source_paths():
    paths=[PROTOCOL,MATRIX_PATH,ROOT/'fevc/tools/run_structured_inference_confirmation.py',ROOT/'rust/Cargo.toml',ROOT/'rust/Cargo.lock',ROOT/'rust/rust-toolchain.toml',ROOT/'rust/RNG_CONTRACT.md',ROOT/'fevc/docs/observation_residual_moments_integration_v1.json',ROOT/'fevc/docs/observation_residual_moments_followup_v1.json']
    for directory in ('rust/crates','rust/vendor/cmg'):
        paths += [p for p in (ROOT/directory).rglob('*') if p.is_file() and not any(q in ('target','.git','__pycache__') for q in p.parts)]
    for directory in (HERE,HERE.parent/'residual_moment_integration',HERE.parent/'residual_moment_followup'):
        paths += list(directory.glob('*.py'))+list(directory.glob('*.rs'))
    return sorted(set(paths))

def freeze(exe,out,profile,preflight_receipt):
    out.mkdir(parents=True,exist_ok=False)
    if preflight_receipt['binary_sha256']!=sha(exe) or preflight_receipt['protocol_sha256']!=sha(PROTOCOL):raise ValueError('preflight binding')
    paths=source_paths();source={str(p.relative_to(ROOT)):sha(p) for p in paths}
    bundle=out/'source.tar.gz'
    with tarfile.open(bundle,'x:gz') as archive:
        for path in paths:archive.add(path,arcname=str(path.relative_to(ROOT)))
        archive.add(exe.parent/'confirmation.rs',arcname='generated/confirmation.rs')
    tasks=tasks_for(profile)
    manifest={'schema':'FEVC-NATIVE-RESIDUAL-CONFIRMATION-MANIFEST-V1','profile':profile,'protocol':PLAN,'protocol_sha256':sha(PROTOCOL),'source_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'source_dirty':True,'source_files':source,'source_bundle_sha256':sha(bundle),'binary_sha256':sha(exe),'generated_sha256':sha(exe.parent/'confirmation.rs'),'preflight':preflight_receipt,'tasks':tasks,'expected_native_calls':sum(t['reps'] for t in tasks),'expected_native_targets':4*sum(t['reps'] for t in tasks),'expected_all_targets':8*sum(t['reps'] for t in tasks),'python':platform.python_version(),'platform':platform.platform()}
    write_json(out/'manifest.json',manifest)
    return manifest

def check_sources(manifest,exe):
    if sha(exe)!=manifest['binary_sha256']:raise ValueError('wrong executable')
    for path,digest_ in manifest['source_files'].items():
        if sha(ROOT/path)!=digest_:raise ValueError(f'changed frozen source: {path}')

def execute(exe,out):
    manifest=json.loads((out/'manifest.json').read_text());check_sources(manifest,exe)
    mh=sha(out/'manifest.json');source_hash=manifest['source_bundle_sha256']
    if sha(out/'source.tar.gz')!=source_hash:raise ValueError('source bundle')
    tasks=manifest['tasks'];started=time.monotonic();receipts=[]
    def worker(t):
        return task_run(exe,out/'tasks'/t['id'],t,manifest['preflight']['fixtures'][f'{t["cell"]}/{t["k"]}'],mh,source_hash)
    with concurrent.futures.ThreadPoolExecutor(max_workers=PLAN['execution']['maximum_concurrent_tasks']) as pool:
        for future in concurrent.futures.as_completed([pool.submit(worker,t) for t in tasks]):
            r=future.result();receipts.append(r)
            print(json.dumps({'task':r['task']['id'],'tasks_done':len(receipts),'tasks_total':len(tasks),'native_success':r['native_success'],'calls':r['native_calls'],'failures':r['call_failures'],'elapsed_seconds':time.monotonic()-started}),flush=True)
    check_sources(manifest,exe)
    return aggregate(out)

def scientific_summary(rows,cell,target,fixture):
    index=TARGETS.index(target)
    adapted=[{**r,'interval_width':r.get('width',0),**{q:0 for q in gates.NUMERIC_FIELDS[3:] if q not in r}} for r in rows]
    s=gates._summary(adapted,len(rows))
    # Spectrum describes the fixed design, including when all outcomes fail.
    s.update(cell=cell['cell'],k=cell['k'],target=target,gate=cell['gate'],reference=cell['reference'],mean_leading_share=fixture['leading_share'][index],mean_remainder_share=fixture['remainder_share'][index],coverage_all=sum(r.get('covered',False) for r in rows if r['status']=='success')/len(rows))
    for field in gates.NUMERIC_FIELDS[3:]:
        if field not in ('leading_share','remainder_share') and not any(field in r for r in rows):s.pop('mean_'+field,None)
    return s

def aggregate(out):
    manifest=json.loads((out/'manifest.json').read_text());mh=sha(out/'manifest.json')
    expected=tasks_for(manifest['profile'])
    if manifest['tasks']!=expected:raise ValueError('modified/overlapping task manifest')
    folders=sorted(p.name for p in (out/'tasks').iterdir() if p.is_dir())
    if folders!=sorted(t['id'] for t in expected):raise ValueError('task folder inventory')
    groups=collections.defaultdict(list);failed_calls=[];receipt_hashes={};counts=collections.Counter();paired_disagreements=0;comparable=0;numeric=collections.defaultdict(list)
    for t in expected:
        folder=out/'tasks'/t['id'];receipt=json.loads((folder/'receipt.json').read_text());path=folder/'rows.jsonl.gz'
        if receipt['task']!=t or receipt['manifest_sha256']!=mh or receipt['source_bundle_sha256']!=manifest['source_bundle_sha256'] or receipt['binary_sha256']!=manifest['binary_sha256'] or receipt['output_sha256']!=sha(path) or receipt['status']!='execution_and_inventory_pass':raise ValueError('task source/receipt binding')
        fixture=manifest['preflight']['fixtures'][f'{t["cell"]}/{t["k"]}']
        targets,calls=validate(read_rows(path),t,fixture)
        if (receipt['native_calls'],receipt['target_rows'])!=(len(calls),len(targets)):raise ValueError('receipt counts')
        receipt_hashes[t['id']]=sha(folder/'receipt.json');counts['native_calls']+=len(calls);counts['all_targets']+=len(targets)
        for c in calls:
            if c['status']!='success':failed_calls.append({'cell':t['cell'],'k':t['k'],**c})
        pairs=collections.defaultdict(dict)
        for r in targets:
            groups[r['arm'],r['cell'],r['k'],r['target']].append(r)
            counts[r['arm']+':'+r['status']]+=1
            pairs[r['replication'],r['target']][r['arm']]=r
            if r['arm']=='native' and r['status']=='success':
                for q in ('point_delta','point_kernel_identity','critical_delta','remainder_identity'):
                    if q in r:numeric[q].append(abs(r[q]))
                if r.get('trace_mcse',0)>0:numeric['covariance_error_in_mcse'].append(abs(r['covariance_delta'])/r['trace_mcse'])
        for pair in pairs.values():
            if all(r['status']=='success' for r in pair.values()):
                comparable+=1;paired_disagreements+=pair['native']['covered']!=pair['exact_same_variance']['covered']
    if counts['native_calls']!=manifest['expected_native_calls'] or counts['all_targets']!=manifest['expected_all_targets']:raise ValueError('aggregate inventory')
    summaries={arm:[] for arm in ARMS}
    for arm in ARMS:
        for cell in CELLS:
            fixture=manifest['preflight']['fixtures'][f'{cell["cell"]}/{cell["k"]}']
            for target in TARGETS:summaries[arm].append(scientific_summary(groups[arm,cell['cell'],cell['k'],target],cell,target,fixture))
    failures,severe=gates._scientific_failures(summaries['native'],manifest['profile'])
    report={'schema':'FEVC-NATIVE-RESIDUAL-CONFIRMATION-RESULT-V1','profile':manifest['profile'],'status':'FAIL' if failures else ('PASS' if manifest['profile']=='confirmation' else 'PIPELINE_PASS_NOT_CONFIRMATION'),'manifest_sha256':mh,'source_bundle_sha256':manifest['source_bundle_sha256'],'binary_sha256':manifest['binary_sha256'],'task_receipt_sha256':receipt_hashes,'counts':dict(counts),'scientific_failures':failures,'severe_misspecification':severe,'failed_calls':failed_calls,'summaries':summaries,'paired_coverage_comparable':comparable,'paired_coverage_disagreements':paired_disagreements,'maximum_numerical_diagnostics':{q:max(v) for q,v in numeric.items()},'limitation':'Only native results are gated. Paired exact rows depend on successful native variance export. This is internal, single-numerical-seed evidence; earlier failures, public options and release status are unchanged.'}
    write_json(out/'result.json',report)
    print(json.dumps({'status':report['status'],'counts':report['counts'],'scientific_failures':failures}),flush=True)
    return report

def main():
    p=argparse.ArgumentParser();p.add_argument('command',choices=('preflight','verify-shards','run','aggregate'));p.add_argument('out',type=Path);p.add_argument('--exe',type=Path);p.add_argument('--preflight',type=Path);p.add_argument('--profile',choices=('tiny','confirmation'),default='tiny');a=p.parse_args()
    out=a.out.resolve()
    if a.command=='preflight':preflight(a.exe.resolve(),out);return 0
    if a.command=='verify-shards':verify_shards(a.exe.resolve(),out,json.loads(a.preflight.read_text()));return 0
    if a.command=='aggregate':r=aggregate(out)
    else:
        pr=json.loads(a.preflight.read_text());freeze(a.exe.resolve(),out,a.profile,pr);r=execute(a.exe.resolve(),out)
    return 2 if r['status']=='FAIL' else 0

if __name__=='__main__':raise SystemExit(main())
