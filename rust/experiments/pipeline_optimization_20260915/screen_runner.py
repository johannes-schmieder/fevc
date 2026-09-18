#!/usr/bin/env python3
"""Manifest-driven sequential calls with deadline, physical-RSS and output guards."""
from __future__ import annotations
import argparse
import csv
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
from campaign import PROFILES

RSS_SOURCE = Path(__file__).parents[1]/'optimization_parity_20260913/monitor_rss.py'
spec = importlib.util.spec_from_file_location('pipeline_rss', RSS_SOURCE)
rss = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rss)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    path=Path(path)
    temporary=path.with_suffix(path.suffix+'.tmp')
    with temporary.open('w') as stream:
        json.dump(value,stream,indent=2,sort_keys=True,allow_nan=False)
        stream.write('\n')
    temporary.replace(path)


def command(profile, seed):
    p=PROFILES[profile]
    controls=' '.join(f'control_{i+1}' for i in range(p['controls']))
    weight='[fw=frequency]' if p['weights'] else ''
    options=['worker(worker)','firm(firm)','backend(rust)','rng(counter_v1)',
        f"deletion({p['deletion']})",f"stayers({p['population']})",f"nuisance({p['nuisance']})",
        f"algorithm({p['algorithm']})",f"engine({p['engine']})",f"preconditioner({p['preconditioner']})",
        'batch(auto)',f"probes({p['probes']})",f'seed({seed})',
        f"exact_limit({p['exact_limit']})",f"maxiter({p['maxiter']})",'nodisplay']
    if p['deletion_id']: options.append('deletionid(match)')
    if p['target_weight']: options.append('targetweight(target_weight)')
    if p['projection']:
        q=p['projection']
        options += [f"project({q['variable']})",f"projecteffect({q['effect']})",f"projectweight({q['weight']})"]
    if p['inference']:
        q=p['inference']
        options += [f"inference({q['request']})",f"inferencemodel({q['model']})",
            f"inferencesimulations({q['probes']})",f"inferencegramprobes({q['gram_probes']})",
            f"inferenceseed({q['seed']})",f"level({q['level']})"]
    return f"fevc y {controls} {weight}, "+' '.join(options)


def read_results(path):
    values={}
    with Path(path).open(newline='') as stream:
        reader=csv.DictReader(stream,delimiter='\t',quoting=csv.QUOTE_NONE)
        if reader.fieldnames!=['kind','name','row','column','value']:
            raise ValueError('invalid export schema')
        for row in reader:
            if None in row or any(v is None for v in row.values()):
                raise ValueError('partial export row')
            key=(row['kind'],row['name'],int(row['row']),int(row['column']))
            if key in values: raise ValueError('duplicate result field')
            values[key]=row['value'].strip()
    return values


def value(data,kind,name,row=0,column=0):
    return data[(kind,name,row,column)]


def scalar(data,name):
    result=float(value(data,'scalar',name))
    if not math.isfinite(result): raise ValueError(f'nonfinite scalar {name}')
    return result


def validate_output(path,call):
    data=read_results(path)
    p=PROFILES[call['profile']]
    if value(data,'macro','backend_selected')!='rust' or value(data,'macro','algorithm')!=p['algorithm']:
        raise ValueError('estimator/backend mismatch')
    if value(data,'macro','deletion')!=p['deletion'] or value(data,'macro','stayers')!=p['population']:
        raise ValueError('deletion/population mismatch')
    if scalar(data,'N_stored')!=call['rows']:
        raise ValueError('unexpected retained sample')
    if scalar(data,'memory_budget_supplied')!=0:
        raise ValueError('omitted-memory policy changed')
    residual='rust_max_complete_residual' if p['family']=='exact' else 'complete_residual_max'
    if scalar(data,residual)>scalar(data,'residual_acceptance_tolerance'):
        raise ValueError('original-system residual gate')
    if scalar(data,'active_processors')!=min(4,call['threads']):
        raise ValueError('Stata processor contract')
    seconds=float(value(data,'timer','command_seconds'))
    if not math.isfinite(seconds) or seconds<=0: raise ValueError('invalid command timer')
    for row in range(1,5):
        for column in range(1,5):
            if not math.isfinite(float(value(data,'matrix','results',row,column))):
                raise ValueError('nonfinite estimator output')
    if call['variant']=='candidate' and p['family']=='exact':
        if value(data,'macro','rust_execution_mode')!='exact_parallel':
            raise ValueError('candidate exact executor not selected')
        if float(value(data,'matrix','rust_exact_execution',1,4))!=call['threads']:
            raise ValueError('exact native thread mismatch')
        bound=float(value(data,'matrix','rust_exact_execution',1,5))
        if not 1<=bound<=call['threads']: raise ValueError('exact worker bound')
    if p['family']!='exact':
        if ('scalar','cmg_threads_requested',0,0) in data:
            requested=scalar(data,'cmg_threads_requested')
            bound=scalar(data,'cmg_threads_used')
        else:
            requested=scalar(data,'rust_execution_threads')
            bound=scalar(data,'rust_execution_workers')
        if requested!=call['threads'] or not 1<=bound<=requested:
            raise ValueError('native execution thread contract')
    if p['family']=='projection':
        for name in ['projection_results','projection_V']:
            if ('rownames',name,0,0) not in data: raise ValueError(f'missing projection output {name}')
    if p['family']=='component_inference':
        for name in ['V','V_primitive','component_spectrum','residual_moment_diagnostics']:
            if ('rownames',name,0,0) not in data: raise ValueError(f'missing inference output {name}')
        if ('macro','inference_support_status',0,0) not in data: raise ValueError('missing inference availability')
        if scalar(data,'inference_solver_max_complete')>scalar(data,'inference_solver_tolerance'):
            raise ValueError('inference original-system residual gate')
        if scalar(data,'inference_seed')!=p['inference']['seed']:
            raise ValueError('inference RNG request mismatch')
    estimator=float(value(data,'timer','estimator_boundary_seconds'))
    if not math.isfinite(estimator) or not 0<=estimator<=seconds:
        raise ValueError('invalid estimator boundary timer')
    return dict(command_seconds=seconds,estimator_boundary_seconds=estimator,
        fields=len(data),residual=scalar(data,residual))


def verify_package(package):
    package=Path(package)
    receipt=json.loads((package/'adapter.json').read_text())
    actual={p.name for p in package.iterdir()}
    if actual!=set(receipt['files'])|{'adapter.json'}: raise ValueError('package inventory mismatch')
    for name,expected in receipt['files'].items():
        p=package/name
        if p.is_symlink() or not p.is_file() or sha(p)!=expected:
            raise ValueError(f'package identity mismatch: {name}')
    return receipt


def run_call(call, package, input_path, output, stata, deadline, rss_gib, slots, deliberate_failure=False):
    output=Path(output)
    output.mkdir(parents=True,exist_ok=False)
    receipt=dict(schema='FEVC-PIPELINE-ATTEMPT-V1',call=call,status='ATTEMPTED',host=socket.gethostname(),
        slots=slots,job_id=os.environ.get('JOB_ID'),task_id=os.environ.get('SGE_TASK_ID'),
        started=time.time(),rss_guard_gib=rss_gib,deadline=deadline)
    write_json(output/'attempt.json',receipt)
    process=None
    maximum=samples=0
    try:
        if slots<call['threads']: raise ValueError('threads exceed allocation')
        package_receipt=verify_package(package)
        if call.get('adapter_sha256',sha(Path(package)/'adapter.json'))!=sha(Path(package)/'adapter.json'):
            raise ValueError('frozen adapter identity mismatch')
        if package_receipt['variant']!=call['variant']: raise ValueError('wrong package variant')
        if sha(input_path)!=call['input_sha256']: raise ValueError('input identity mismatch')
        expected=command(call['profile'],call['seed'])
        if call['command']!=expected: raise ValueError('effective command changed')
        if deadline-time.time()<5: raise TimeoutError('no task time left before launch')
        env=os.environ.copy()
        env.update(PF_PACKAGE=str(package),PF_INPUT=str(input_path),PF_OUTPUT=str(output/'result.tsv'),
            PF_COMMAND=expected,PF_ROWS=str(call['rows']),PF_THREAD_CONTRACT='FEVC-PIPELINE-THREADS-V1',
            PF_NATIVE_THREADS=str(call['threads']),PF_ASSIGNED_SLOTS=str(slots),
            PF_DELIBERATE_FAILURE=str(int(deliberate_failure)),OMP_NUM_THREADS=str(call['threads']),
            RAYON_NUM_THREADS=str(call['threads']),STATATMP=str(output/'stata-tmp'))
        (output/'stata-tmp').mkdir()
        arguments=[str(stata),'-b' if os.uname().sysname=='Darwin' else '-q','do',str(Path(__file__).with_name('screen_stata.do'))]
        with (output/'application.txt').open('w') as console:
            process=subprocess.Popen(arguments,cwd=output,env=env,stdout=console,stderr=subprocess.STDOUT,start_new_session=True)
            while process.poll() is None:
                pids,memory=rss.descendants(process.pid)
                maximum=max(maximum,sum(memory.get(pid,0) for pid in pids))
                samples+=1
                if maximum>rss_gib*1024**2: raise MemoryError('process-tree RSS guard exceeded')
                if time.time()>deadline: raise TimeoutError('total task deadline reached')
                time.sleep(.1)
            receipt.update(process_return_code=process.returncode,physical_rss_kib=maximum,rss_samples=samples,
                whole_process_seconds=time.time()-receipt['started'])
        if process.returncode!=0: raise RuntimeError('application process failed')
        transcript='\n'.join(p.read_text(errors='replace') for p in output.glob('*.log'))+(output/'application.txt').read_text(errors='replace')
        if 'FEVC_PIPELINE_SCREEN_CALL_PASS' not in transcript: raise RuntimeError('application PASS marker missing')
        if samples==0 or maximum==0: raise RuntimeError('physical RSS was not measured')
        receipt.update(validate_output(output/'result.tsv',call))
        receipt.update(status='PASS',result_sha256=sha(output/'result.tsv'),package_adapter_sha256=sha(Path(package)/'adapter.json'))
    except Exception as error:
        if process is not None and process.poll() is None:
            os.killpg(process.pid,signal.SIGTERM)
            try: process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid,signal.SIGKILL)
                process.wait()
        receipt.update(status='FAIL',error=f'{type(error).__name__}: {error}')
    receipt.update(physical_rss_kib=maximum,rss_samples=samples,
        whole_process_seconds=time.time()-receipt['started'])
    if process is not None: receipt['process_return_code']=process.returncode
    receipt['finished']=time.time()
    write_json(output/'attempt.json',receipt)
    return receipt


def run_task(manifest,task_number,stata,slots,started,expected_hash,deliberate_failure=False):
    m=json.loads(manifest.read_text())
    if m['status'] not in ('FROZEN_SUPPORTED_PATH_SCREEN','FROZEN_DEVELOPMENT_SMOKE'):
        raise ValueError('manifest is not launch-ready')
    if sha(manifest)!=expected_hash: raise ValueError('manifest identity mismatch')
    for name,digest in m['harness_hashes'].items():
        if sha(Path(__file__).parent/name)!=digest: raise ValueError(f'harness source mismatch: {name}')
    if not 1<=task_number<=len(m['tasks']): raise ValueError('task outside manifest')
    task=m['tasks'][task_number-1]
    if task['id']!=task_number or slots!=m['slots']: raise ValueError('task/allocation mismatch')
    root=manifest.parent
    output=root/'tasks'/f'task-{task_number:02d}'
    output.mkdir(parents=True,exist_ok=False)
    deadline=started+m['seconds']-30
    inventory={call['id']:'UNATTEMPTED' for call in task['calls']}
    write_json(output/'inventory.json',inventory)
    for call in task['calls']:
        receipt=run_call(call,root/'packages'/call['variant'],root/'inputs'/f"{call['profile']}.csv",
            output/call['id'],stata,deadline,m['rss_gib'],m['slots'],deliberate_failure)
        inventory[call['id']]=receipt['status']
        write_json(output/'inventory.json',inventory)
        if receipt['status']!='PASS': return False
    write_json(output/'task.json',dict(status='PASS',host=socket.gethostname(),job_id=os.environ.get('JOB_ID'),
        task_id=task_number,manifest_sha256=sha(manifest),calls=inventory))
    return True


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('manifest',type=Path)
    p.add_argument('task',type=int)
    p.add_argument('--stata',type=Path,required=True)
    a=p.parse_args()
    raise SystemExit(0 if run_task(a.manifest,a.task,a.stata,int(os.environ['NSLOTS']),
        float(os.environ['PF_TASK_STARTED_UNIX']),os.environ['PF_MANIFEST_SHA256']) else 1)
