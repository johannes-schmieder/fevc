"""Local source-bound public-ABI development, with complete attempt accounting."""
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
import re
import statistics
import subprocess
import sys
import tarfile
import time

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
PROTOCOL=ROOT/'fevc/docs/individual_inference_development_v1.json'
PLAN=json.loads(PROTOCOL.read_text())
TARGETS=('worker','firm','covariance','total')
def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    result=importlib.util.module_from_spec(spec);sys.modules[name]=result;spec.loader.exec_module(result)
    return result
OBS=module('individual_obs_gates',ROOT/'fevc/tools/run_structured_inference_confirmation.py')
Q1=module('individual_match_gates',ROOT/'fevc/tools/run_inference_repair_campaign.py')
Q0=module('individual_q0_gates',ROOT/'fevc/tools/run_rc_match_q0.py').CAMPAIGN
GATES={'observation':OBS,'match_q0':Q0,'match_q1':Q1}
CELLS=[dict(c,family='observation')for c in json.loads((ROOT/'fevc/docs/rc_observation_inference_v1.json').read_text())['cells']]
CELLS += [dict(c,family=f,k=20)for f,m in [('match_q0',Q0),('match_q1',Q1)]for c in m.CELLS]
assert len(CELLS)==48

def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def write(path,value):
    with Path(path).open('x') as stream:json.dump(value,stream,indent=2,allow_nan=False)
def digest(value):return hashlib.sha256(json.dumps(value,sort_keys=True,allow_nan=False,separators=(',',':')).encode()).hexdigest()
def seed(master,cell,k,rep):
    mask=(1<<64)-1
    def mix(x):
        x=(x+0x9e3779b97f4a7c15)&mask;x=((x^(x>>30))*0xbf58476d1ce4e5b9)&mask;x=((x^(x>>27))*0x94d049bb133111eb)&mask
        return x^(x>>31)
    tag=0xcbf29ce484222325
    # Preserve the frozen fixture's literal, which differs from standard FNV.
    for byte in cell.encode():tag=((tag^byte)*0x1000000001b3)&mask
    return mix(master^mix(tag)^mix(k)^mix(rep))
def tasks(profile):
    count=PLAN['profiles'][profile]['replications_per_cell'];step=2 if profile=='tiny'else 100
    for c in CELLS:
        master=PLAN['rng'][f'{profile}_master']+list(GATES).index(c['family'])*PLAN['rng']['family_master_increment']
        for start in range(0,count,step):
            yield dict(profile=profile,cell=c['cell'],family=c['family'],k=c['k'],start=start,reps=min(step,count-start),master=master,numseed=8675309,
                       id=f'{c["family"]}-{c["cell"]}-{c["k"]}-{start:04d}')
def spec_for(task):return next(c for c in CELLS if all(c[k]==task[k]for k in ('cell','family','k')))
def is_q1(c):return c.get('reference',c.get('reference_distribution','')).lower()=='q1'
def finite(x):
    if isinstance(x,float)and not math.isfinite(x):raise ValueError('nonfinite JSON value')
    if isinstance(x,dict):
        for v in x.values():finite(v)
    if isinstance(x,list):
        for v in x:finite(v)
def array(row,key,length,nullable=False):
    value=row.get(key)
    if not isinstance(value,list)or len(value)!=length:raise ValueError(f'{key} shape')
    if not nullable and any(type(x)not in(int,float)for x in value):raise ValueError(f'{key} missing value')
    return value
def validate(rows,task):
    finite(rows)
    header={'kind':'task','schema':'individual-v1',**{k:task[k]for k in ('family','profile','cell','k','start','reps','master','numseed')}}
    if not rows or rows[0]!=header:raise ValueError('task identity/schema')
    designs=[r for r in rows if r.get('kind')=='design'];calls=[r for r in rows if r.get('kind')=='call']
    if len(designs)!=1 or len(rows)!=2+task['reps']or len(calls)!=task['reps']:raise ValueError('record inventory')
    d=designs[0];array(d,'truth',4)
    if type(d.get('n'))is not int or d['n']<=0 or d.get('min_variance',0)<=0:raise ValueError('design dimensions/variance')
    c=spec_for(task);q1=is_q1(c)
    if c['family']=='observation':
        if d.get('p')!=2*c['k']-1+2*int(c['controls'])or not 0<d.get('max_h',0)<1:raise ValueError('observation identification')
        array(d,'leading_share',4);array(d,'remainder_share',4)
    keys=set()
    for r in calls:
        rep=r.get('replication')
        if type(rep)is not int or rep not in range(task['start'],task['start']+task['reps'])or rep in keys:raise ValueError('duplicate/out-of-range replication')
        keys.add(rep)
        if r.get('seed')!=seed(task['master'],task['cell'],task['k'],rep):raise ValueError('semantic RNG key')
        if type(r.get('seconds'))not in(int,float)or r['seconds']<0:raise ValueError('runtime')
        if r.get('status')=='shared_failure':
            if not isinstance(r.get('detail'),str)or not re.fullmatch(r'[A-Za-z0-9 ]+: status [1-9][0-9]*: .+',r['detail']):raise ValueError('unclassified shared failure')
            continue
        if r.get('status')!='success':raise ValueError('unknown status')
        array(r,'point',4);array(r,'point_mcse',4);array(r,'targets',8)
        array(r,'spectrum',60,True);array(r,'q1',80,True);array(r,'primitive',9,True);array(r,'covariance',16,True)
        if r.get('fit')!=(2 if c['family']=='observation'else 1)or r.get('joint')not in(0,1,2):raise ValueError('fit/joint code')
        if r['joint']==0:
            array(r,'primitive',9);array(r,'covariance',16)
        elif any(x is not None for x in r['primitive']+r['covariance']):raise ValueError('invalid joint covariance exported')
        if c['family']=='observation'and(r.get('gram_probes')!=512 or array(r,'gram_rcond',1)[0]<=0):raise ValueError('residual Gram receipt')
        if c['family']!='observation'and(r.get('gram_probes')!=0 or r.get('gram_rcond')!=[None]):raise ValueError('inapplicable Gram')
        if not 0<=r.get('max_residual',-1)<=r.get('residual_gate',0)or r.get('residual_gate',0)<=0:raise ValueError('uncertified solver')
        for field in ('units','solver_columns','counter_atoms','counter_words','peak'):
            if type(r.get(field))is not int or r[field]<=0:raise ValueError(f'invalid {field}')
        if not 0<=r.get('floored',-1)<r['units']:raise ValueError('floor count')
        computed=0
        critical=0
        for t in range(4):
            v,status=r['targets'][2*t:2*t+2]
            if status not in (0,1,4,6)or(status==0 and v<=0):raise ValueError('invalid q0 target')
            if r['joint']==0 and not math.isclose(v,r['covariance'][5*t],rel_tol=1e-12,abs_tol=1e-14):raise ValueError('scalar/joint mismatch')
            if q1:
                q=r['q1'][20*t:20*t+20];status=q[16]
                if status not in(0,1,2,3,6):raise ValueError('invalid q1 target code')
                critical+=100000*(status in(0,3))
                if status==0:
                    if any(q[j]is None for j in(2,6,8,9,10,11,17))or min(q[2],q[6],q[9],q[17])<=0 or q[10]>q[11]:raise ValueError('invalid q1 interval')
                    det=1-q[5]**2/(q[2]*q[6])
                    if not math.isclose(det,q[17],rel_tol=1e-9,abs_tol=1e-10):raise ValueError('q1 determinant')
                elif q[10]is not None or q[11]is not None:raise ValueError('unavailable q1 endpoints')
            computed+=status==0
        if r.get('computed')!=computed:raise ValueError('computed-target count')
        if r.get('critical_draws')!=critical:raise ValueError('critical-draw count')
        if not q1 and any(x is not None for x in r['q1']):raise ValueError('inapplicable q1 output')
    return d,calls

def execute(binary,out,task):
    out.mkdir(parents=True,exist_ok=False)
    temporary=out/'rows.jsonl.partial'
    with temporary.open('x')as stream,(out/'stderr.log').open('x')as err:
        result=subprocess.run([str(binary),task['profile'],task['cell'],str(task['k']),str(task['start']),str(task['reps']),str(task['master']),'individual-v1'],stdout=stream,stderr=err,timeout=1800)
    if result.returncode:raise ValueError(f'native process exit {result.returncode}: {out}')
    rows=[json.loads(line)for line in temporary.read_text().splitlines()]
    d,calls=validate(rows,task)
    with (out/'rows.jsonl.gz').open('xb')as stream:
        stream.write(gzip.compress(temporary.read_bytes(),mtime=0))
    temporary.unlink()
    receipt={'task':task,'status':'validated','binary_sha256':sha(binary),'output_sha256':sha(out/'rows.jsonl.gz'),'calls':len(calls),'targets':4*len(calls),'design':d,'shared_failures':sum(r['status']!='success'for r in calls)}
    write(out/'receipt.json',receipt)
    return receipt

def summaries(all_tasks,out):
    groups=collections.defaultdict(list);designs={}
    for task in all_tasks:
        folder=out/'tasks'/task['id'];receipt=json.loads((folder/'receipt.json').read_text())
        if receipt['task']!=task or receipt['output_sha256']!=sha(folder/'rows.jsonl.gz'):raise ValueError('task receipt binding')
        rows=[json.loads(x)for x in gzip.decompress((folder/'rows.jsonl.gz').read_bytes()).decode().splitlines()]
        d,calls=validate(rows,task);c=spec_for(task);key=(c['family'],c['cell'],c['k'])
        if key in designs and designs[key]!=d:raise ValueError('design changed across shards')
        designs[key]=d
        for r in calls:
            for t,target in enumerate(TARGETS):
                row={'status':'shared_failure','point_error':None}
                if r['status']=='success':
                    v,status=r['targets'][2*t:2*t+2];point=r['point'][t]
                    if is_q1(c):
                        q=r['q1'][20*t:20*t+20];status=q[16];lo,hi=q[10:12]
                    else:
                        radius=1.959963984540054*math.sqrt(v)if status==0 else 0
                        lo,hi=point-radius,point+radius
                    row={'status':'success'if status==0 else f'target_{int(status)}','point_error':point-d['truth'][t],
                         'estimated_sd':math.sqrt(v)if v>0 else None}
                    if status==0:row.update(covered=lo<=d['truth'][t]<=hi,lower_miss=d['truth'][t]<lo,upper_miss=d['truth'][t]>hi,interval_width=hi-lo)
                groups[key+(target,)].append(row)
    result=[]
    for (family,cell,k,target),rows in groups.items():
        c=next(c for c in CELLS if(c['family'],c['cell'],c['k'])==(family,cell,k))
        # This shared summary counts every returned point, including target-
        # local interval failures. Its mean helper excludes inapplicable SEs.
        s=Q1._summary(rows,len(rows));s.update(c,target=target)
        s['se_denominator']=sum(r.get('status')=='success'and r.get('estimated_sd')is not None for r in rows)
        if family=='observation':
            d=designs[family,cell,k];t=TARGETS.index(target)
            s.update(mean_leading_share=d['leading_share'][t],mean_remainder_share=d['remainder_share'][t])
        result.append(s)
    reports={}
    for family in GATES:
        selected=[s for s in result if s['family']==family]
        failures,stress=GATES[family]._scientific_failures(selected,'confirmation'if family=='observation'else'development')
        for s in selected:
            primary=s['gate']=='correct'and(not is_q1(s)or s['target']!='covariance')
            if primary and s['se_denominator']!=s['successes']:
                failures.append(f"{s['cell']}/{s['k']}/{s['target']}: SE calibration unavailable for some computed intervals")
        reports[family]={'status':'FAIL'if failures else'PASS','failures':failures,'stress':stress}
    return {'status':'FAIL'if any(r['status']=='FAIL'for r in reports.values())else'PASS','reports':reports,'summaries':result}

def freeze(build,out,profile):
    out.mkdir(parents=True,exist_ok=False)
    receipt=json.loads((build/'receipt.json').read_text())
    for f in GATES:
        if receipt['binaries'][f]!=sha(build/f):raise ValueError('build binary mismatch')
    for p,h in receipt['adapter_sources'].items():
        if sha(HERE/p)!=h:raise ValueError('adapter changed after build')
    for p,h in receipt['sources'].items():
        if sha(ROOT/p)!=h:raise ValueError('production source changed after build')
    files=set()
    for directory in ('rust/crates','rust/vendor/cmg','rust/experiments/individual_inference'):
        files.update(p for p in(ROOT/directory).rglob('*')if p.is_file()and p.suffix in('.rs','.toml','.py')and 'target'not in p.parts and '__pycache__'not in p.parts)
    files.update((ROOT/'fevc').glob('*.ado'))
    files.update([ROOT/'rust/Cargo.toml',ROOT/'rust/Cargo.lock',PROTOCOL,ROOT/'fevc/docs/individual_inference_upgrade_v1.json'])
    for name in ('run_structured_inference_confirmation.py','run_inference_repair_campaign.py','run_rc_match_q0.py','run_match_inference_q0_campaign.py','check_individual_native_parity.py'):files.add(ROOT/'fevc/tools'/name)
    identities={str(p.relative_to(ROOT)):sha(p)for p in sorted(files)}
    with tarfile.open(out/'source.tar.gz','x:gz')as tar:
        for p in sorted(files):tar.add(p,arcname=str(p.relative_to(ROOT)),recursive=False)
        for f in GATES:
            tar.add(build/f,arcname=f'bin/{f}',recursive=False)
            tar.add(build/f'{f}.rs',arcname=f'build/{f}.rs',recursive=False)
    manifest={'schema':'individual-development-manifest-v1','profile':profile,'protocol':PLAN,'base_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
              'source_files':identities,'bundle_sha256':sha(out/'source.tar.gz'),'build':str(build),'build_receipt_sha256':sha(build/'receipt.json'),'binaries':receipt['binaries'],
              'cells':CELLS,'tasks':list(tasks(profile)),'thresholds':{f:g.THRESHOLDS for f,g in GATES.items()}}
    write(out/'manifest.json',manifest);return manifest

def match_preflight(build,out):
    reports={}
    for family in ('match_q0','match_q1'):
        rows=[];commands=[];gate=GATES[family]
        for c in gate.CELLS:
            command=[str(build/family),'oracle-preflight',c['cell'],'20',*gate._settings_arguments(gate.TASK_SETTINGS,preflight_spectrum_probes=4096)]
            result=subprocess.run(command,capture_output=True,text=True,timeout=180)
            commands.append(command)
            if result.returncode:raise ValueError(f'oracle preflight process: {result.stderr[-2000:]}')
            parsed=[json.loads(line)for line in result.stdout.splitlines()]
            if len(parsed)!=4 or {r.get('target')for r in parsed}!=set(TARGETS):raise ValueError('oracle preflight inventory')
            for row in parsed:gate._validate_preflight_row(row,c,20)
            rows.extend(parsed)
        failures=gate._preflight_failures(rows,gate.CELLS,20)
        reports[family]={'status':'FAIL'if failures else'PASS','failures':failures,'rows':rows,'commands':commands}
    write(out/'oracle-preflight.json',reports)
    return reports

def check_reproducibility(build,out,manifest):
    checked=[]
    for family in GATES:
        task=next(t for t in manifest['tasks']if t['family']==family)
        original=[json.loads(x)for x in gzip.decompress((out/'tasks'/task['id']/'rows.jsonl.gz').read_bytes()).decode().splitlines()]
        d,calls=validate(original,task)
        for rep in reversed(range(2)):
            split=dict(task,start=rep,reps=1,id=f'{task["id"]}-split-{rep}')
            folder=out/'reproducibility'/split['id']
            execute(build/family,folder,split)
            _,part=validate([json.loads(x)for x in gzip.decompress((folder/'rows.jsonl.gz').read_bytes()).decode().splitlines()],split)
            lhs={k:v for k,v in calls[rep].items()if k!='seconds'}
            rhs={k:v for k,v in part[0].items()if k!='seconds'}
            if lhs!=rhs:raise ValueError('split/reversed task changed numerical output')
            checked.append(split['id'])
    return checked

def campaign(build,out,profile,workers,pipeline=None,parity=None):
    if not 1<=workers<=8:raise ValueError('worker limit')
    if profile=='development':
        if pipeline is None:raise ValueError('development requires a validated tiny pipeline')
        previous=json.loads((pipeline/'result.json').read_text());pm=json.loads((pipeline/'manifest.json').read_text())
        if previous.get('status')!='PIPELINE_PASS'or previous.get('manifest_sha256')!=sha(pipeline/'manifest.json')or pm['build_receipt_sha256']!=sha(build/'receipt.json')or previous.get('protocol_sha256')!=sha(PROTOCOL):raise ValueError('pipeline source/receipt mismatch')
        if parity is None:raise ValueError('development requires Stata parity')
        pr=json.loads(parity.read_text())
        if pr.get('status')!='PASS'or pr.get('build_receipt_sha256')!=sha(build/'receipt.json')or pr.get('script_sha256')!=sha(ROOT/'fevc/tools/check_individual_native_parity.py'):raise ValueError('Stata parity binding')
        for p,h in pm['source_files'].items():
            if sha(ROOT/p)!=h:raise ValueError('pipeline source changed')
    manifest=freeze(build,out,profile)
    if profile=='development':write(out/'prerequisites.json',{'pipeline':str(pipeline),'pipeline_sha256':sha(pipeline/'result.json'),'parity':str(parity),'parity_sha256':sha(parity)})
    preflight=match_preflight(build,out)if profile=='tiny'else None
    if preflight and any(r['status']!='PASS'for r in preflight.values()):
        result={'status':'PREFLIGHT_FAIL','preflight':preflight,'manifest_sha256':sha(out/'manifest.json')}
        write(out/'result.json',result);return result
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers)as pool:
        futures={pool.submit(execute,build/t['family'],out/'tasks'/t['id'],t):t for t in manifest['tasks']}
        for future in concurrent.futures.as_completed(futures):
            receipt=future.result();print(json.dumps({'task':receipt['task']['id'],'calls':receipt['calls'],'shared_failures':receipt['shared_failures']}),flush=True)
    if profile=='tiny':
        design_rows={}
        for t in manifest['tasks']:
            if t['family']!='observation':continue
            d=json.loads((out/'tasks'/t['id']/'receipt.json').read_text())['design']
            for i,target in enumerate(TARGETS):design_rows[t['cell'],t['k'],target]={'mean_leading_share':d['leading_share'][i],'mean_remainder_share':d['remainder_share'][i]}
        failures=OBS._spectral_failures(design_rows)
        result={'status':'PREFLIGHT_FAIL'if failures else'PIPELINE_PASS','calls':96,'target_attempts':384,'scientific_claim':False,'design_failures':failures,
                'reproducibility':check_reproducibility(build,out,manifest)}
    else:result=summaries(manifest['tasks'],out)
    result.update(manifest_sha256=sha(out/'manifest.json'),protocol_sha256=sha(PROTOCOL))
    write(out/'result.json',result);return result

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('profile',choices=['tiny','development']);parser.add_argument('build',type=Path);parser.add_argument('output',type=Path);parser.add_argument('--workers',type=int,default=8);parser.add_argument('--pipeline',type=Path);parser.add_argument('--parity',type=Path)
    args=parser.parse_args();result=campaign(args.build.resolve(),args.output.resolve(),args.profile,args.workers,args.pipeline,args.parity)
    print(json.dumps({'status':result['status']}));sys.exit(result['status']not in('PASS','PIPELINE_PASS'))
