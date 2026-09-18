#!/usr/bin/env python3
"""Guarded SCC qualification, first-stage submission and accounting boundary."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import subprocess
import sys
import time
from campaign import PROFILES
from screen_input import build as generate_input
from screen_manifest import freeze
from screen_runner import rss, run_task, sha, write_json, verify_package
from screen_validate import validate
from thread_adapter import build as adapt

HERE=Path(__file__).resolve().parent


def scc_granted_slots(requested):
    # Verified SCC JSV rule: non-GPU omp requests of 7 become omp8/8.
    # Four-slot smokes, 14-slot screens and one-slot aggregation are unchanged.
    grants={1:1,4:4,7:8,14:14}
    if requested not in grants: raise ValueError('unregistered SCC resource request')
    return grants[requested]


def checked(arguments,output,cwd=None):
    with Path(output).open('x') as stream:
        subprocess.run([str(a) for a in arguments],cwd=cwd,stdout=stream,stderr=subprocess.STDOUT,check=True)


def accounting(text,job,tasks,slots):
    records=[]
    for block in re.split(r'^=+\s*$',text,flags=re.M):
        row={}
        for line in block.splitlines():
            parts=line.split(None,1)
            if len(parts)==2:
                if parts[0] in row: raise ValueError('duplicate accounting field')
                row[parts[0]]=parts[1].strip()
        if row: records.append(row)
    actual=[]
    for row in records:
        if row.get('jobnumber')!=str(job): raise ValueError('accounting job identity differs')
        task=0 if row.get('taskid') in ('undefined','0') else int(row['taskid'])
        actual.append(task)
        if row.get('failed','').split()[0]!='0' or row.get('exit_status')!='0':
            raise ValueError(f'scheduler failure for task {task}')
        if int(row['slots'])!=slots or not row.get('hostname') or not row.get('qname'):
            raise ValueError('incomplete scheduler allocation record')
        for key in ('ru_wallclock','cpu'):
            if float(row[key])<0: raise ValueError('invalid scheduler duration')
        if not row.get('maxvmem'): raise ValueError('missing virtual-memory accounting')
    if sorted(actual)!=sorted(tasks): raise ValueError('incomplete or duplicate scheduler inventory')
    return records


def collect_accounting(root,job,tasks,slots):
    destination=root/'qacct'/f'{job}.txt'
    last=''
    for _ in range(20):
        result=subprocess.run(['qacct','-j',str(job)],capture_output=True,text=True)
        last=result.stdout
        if result.returncode==0:
            try: rows=accounting(last,job,tasks,slots)
            except ValueError as error:
                if 'incomplete' not in str(error):
                    destination.write_text(last)
                    raise
            else:
                destination.write_text(last)
                return rows
        time.sleep(15)
    destination.write_text(last)
    raise ValueError('scheduler accounting incomplete after bounded wait')


def verify_deployment(root):
    snapshot=root/'snapshot.json'
    expected=(root/'source/SOURCE_SNAPSHOT_SHA256.txt').read_text().strip()
    if sha(snapshot)!=expected: raise ValueError('deployed snapshot mismatch')
    for name,digest in json.loads(snapshot.read_text())['source'].items():
        if sha(root/'source'/name)!=digest: raise ValueError(f'deployed source drift: {name}')
    qualification=json.loads((root/'local-qualification.json').read_text())
    if qualification['status']!='PASS' or qualification['candidate_snapshot_sha256']!=expected:
        raise ValueError('local qualification is not source-bound')
    return expected


def native_qualification(root,stata):
    source=root/'source'
    work=Path(os.environ['TMPDIR'])/f'fevc-screen-native-{os.environ["JOB_ID"]}'
    work.mkdir()
    logs=root/'artifacts/linux-native'
    logs.mkdir()
    if not subprocess.check_output(['rustc','--version'],text=True).startswith('rustc 1.85.1 '):
        raise ValueError('unapproved Rust toolchain')
    os.environ.update(CARGO_TARGET_DIR=str(work/'target'),CARGO_BUILD_JOBS='4',
        RUST_TEST_THREADS='4',VCKSS_STATA_SPI_DIR=str(root/'spi'),
        RUSTC_WRAPPER='',RUSTC_WORKSPACE_WRAPPER='')
    manifest=source/'rust/stata_backend/Cargo.toml'
    checked(['cargo','test','--manifest-path',manifest,'--locked','--all-targets','--','--test-threads=4'],logs/'standalone-tests.txt')
    checked(['cargo','clippy','--manifest-path',manifest,'--locked','--all-targets','--','-D','warnings'],logs/'clippy.txt')
    checked(['cargo','build','--manifest-path',manifest,'--locked','--release'],logs/'build.txt')
    plugin=root/'artifacts/fevc_rust_linux_x64.plugin'
    shutil.copy2(work/'target/release/libvckss_stata.so',plugin)
    checked(['file',plugin],logs/'file.txt')
    checked(['ldd',plugin],logs/'ldd.txt')
    for name in ('cshim_interrupt_test','cshim_error_transport_test','abi_header_compat_test'):
        output=work/name
        args=['cc','-std=c11','-Wall','-Wextra','-Werror','-DSYSTEM=OPUNIX',
            '-I',root/'spi','-I',source/'rust/stata_backend/cshim','-I',source/'rust/stata_backend/include']
        if name=='abi_header_compat_test': args+=['-c']
        else: args+=['-ffunction-sections','-Wl,--gc-sections']
        command=args+[source/f'rust/stata_backend/tests/{name}.c','-o',output]
        if name!='abi_header_compat_test': command+=['-lm']
        checked(command,logs/f'{name}-build.txt')
        if name!='abi_header_compat_test': checked([output],logs/f'{name}-run.txt')
    plain=root/'native-package'
    plain.mkdir()
    for p in (source/'fevc').iterdir():
        if p.is_file() and p.suffix in ('.ado','.mata','.sthlp','.pkg','.toc'): shutil.copyfile(p,plain/p.name)
    shutil.copy2(plugin,plain/plugin.name)
    # The ordinary source package deliberately does not ship a binary. Add
    # only this tested Linux artifact to this isolated installation catalog.
    pkg=plain/'fevc.pkg'
    pkg.write_text(pkg.read_text()+'\nf fevc_rust_linux_x64.plugin\n')
    install=root/'isolated-install'
    install.mkdir()
    cases=[('install',source/'fevc/tests/stata/test_rust_public_install.do',
        [plain,install,'qualified',source/'fevc/tests/stata'],'PASS test_rust_public_install.do'),
        ('routing',source/'fevc/tests/stata/test_backend_routing.do',[plain],'PASS test_backend_routing.do'),
        ('public-exact',HERE/'public_exact_smoke.do',[plain,'0','1 4'],'PASS public_exact_smoke.do'),
        ('exact-failure',HERE/'public_exact_failure_smoke.do',[plain],'PASS public_exact_failure_smoke.do')]
    for name,driver,args,marker in cases:
        directory=logs/name
        directory.mkdir()
        checked([stata,'-b','do',driver,*args],directory/'console.txt',directory)
        transcript='\n'.join(p.read_text(errors='replace') for p in directory.glob('*.log'))
        if marker not in transcript: raise ValueError(f'native application qualification failed: {name}')
    receipt=dict(status='PASS',snapshot_sha256=sha(root/'snapshot.json'),binary_sha256=sha(plugin),
        job_id=os.environ['JOB_ID'],host=socket.gethostname(),slots=4,
        scope='affected standalone ABI, C transport, complete isolated install routes and explicit/auto exact; local high-thread tests not repeated')
    write_json(root/'receipts/linux-native.json',receipt)
    return plugin


def smoke(root,threads,stata):
    snapshot_hash=verify_deployment(root)
    slots=int(os.environ['NSLOTS'])
    if slots!=scc_granted_slots(threads): raise ValueError('smoke allocation mismatch')
    if threads==4:
        plugin=native_qualification(root,stata)
        packages=root/'packages'
        packages.mkdir()
        adapt(root/'snapshot.json',snapshot_hash,plugin,sha(plugin),packages/'candidate','candidate')
        baseline=root/'baseline'
        bp=baseline/'fevc_rust_linux_x64.plugin'
        adapt(baseline/'snapshot.json',sha(baseline/'snapshot.json'),bp,sha(bp),packages/'baseline','baseline')
        inputs=root/'smoke-inputs'
        for profile in PROFILES: generate_input(inputs/f'{profile}.csv',profile,True)
    else:
        submission=json.loads((root/'submissions/smoke4.json').read_text())
        collect_accounting(root,submission['job_id'],[0],4)
        prior=json.loads((root/'receipts/smoke4.json').read_text())
        guard=json.loads((root/'receipts/smoke4-guard.json').read_text())
        if prior['status']!='PASS' or guard['status']!='PASS': raise ValueError('four-slot smoke did not pass')
    stage=root/f'smoke{threads}'
    m=freeze(stage,root/'smoke-inputs',root/'packages',threads,slots)
    for task in m['tasks']:
        if not run_task(stage/'manifest.json',task['id'],stata,slots,float(os.environ['PF_TASK_STARTED_UNIX']),sha(stage/'manifest.json')):
            raise ValueError('smoke application failure')
    report=validate(m,stage)
    write_json(stage/'report.json',report)
    if report['status']!='SMOKE_PASS': raise ValueError('smoke scientific or output validation failure')
    # The historical twelve profiles do not select compressed execution.
    # Cover that adapter boundary separately without changing their requests.
    os.environ.update(PF_THREAD_CONTRACT='FEVC-PIPELINE-THREADS-V1',
        PF_NATIVE_THREADS=str(threads),PF_ASSIGNED_SLOTS=str(slots))
    for variant in ('baseline','candidate'):
        directory=root/f'smoke{threads}-compressed-{variant}'
        directory.mkdir()
        checked([stata,'-b','do',HERE/'compressed_boundary_smoke.do',root/'packages'/variant,
            root/'smoke-inputs/mover_match_both.csv'],directory/'console.txt',directory)
        transcript='\n'.join(p.read_text(errors='replace') for p in directory.glob('*.log'))
        if 'PASS compressed_boundary_smoke.do' not in transcript:
            raise ValueError('compressed thread-boundary application failure')
    # Same manifest/runner, isolated outputs: a real Stata failure must stop
    # the task and preserve its second call as unattempted.
    failure=root/f'smoke{threads}-deliberate-failure'
    freeze(failure,root/'smoke-inputs',root/'packages',threads,slots)
    if run_task(failure/'manifest.json',1,stata,slots,float(os.environ['PF_TASK_STARTED_UNIX']),sha(failure/'manifest.json'),True):
        raise ValueError('deliberate failure was incorrectly accepted')
    states=json.loads((failure/'tasks/task-01/inventory.json').read_text())
    if list(states.values())!=['FAIL','UNATTEMPTED']: raise ValueError('failure inventory did not propagate')
    write_json(root/f'receipts/smoke{threads}.json',dict(status='PASS',job_id=os.environ['JOB_ID'],
        slots=slots,requested_slots=threads,native_threads=threads,host=socket.gethostname(),snapshot_sha256=snapshot_hash,
        manifest_sha256=sha(stage/'manifest.json'),report_sha256=sha(stage/'report.json'),
        deliberate_failure_propagated=True,performance_claim=None))


def guarded(root,mode,stata):
    output=root/f'receipts/{mode}-guard.json'
    if output.exists(): raise ValueError('guard receipt exists')
    child=subprocess.Popen([sys.executable,str(Path(__file__)),root,mode+'-inner','--stata',stata],start_new_session=True)
    maximum=samples=0
    status='FAIL'
    error=None
    deadline=float(os.environ['PF_TASK_STARTED_UNIX'])+1470
    try:
        while child.poll() is None:
            pids,memory=rss.descendants(child.pid)
            maximum=max(maximum,sum(memory.get(pid,0) for pid in pids))
            samples+=1
            if maximum>10*1024**2: raise MemoryError('whole-smoke process-tree RSS exceeded 10 GiB')
            if time.time()>deadline: raise TimeoutError('whole-smoke task window exhausted')
            time.sleep(.2)
        if child.returncode!=0: raise RuntimeError('smoke subprocess failed')
        status='PASS'
    except Exception as exc:
        error=str(exc)
        if child.poll() is None:
            os.killpg(child.pid,signal.SIGTERM)
            try: child.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid,signal.SIGKILL)
                child.wait()
    write_json(output,dict(status=status,error=error,process_return_code=child.returncode,
        maximum_physical_rss_kib=maximum,samples=samples,rss_guard_gib=10,job_id=os.environ['JOB_ID']))
    if status!='PASS': raise ValueError(error)


def submit(root,stage):
    verify_deployment(root)
    destination=root/'submissions'/f'{stage}.json'
    if destination.exists(): raise ValueError('stage already submitted; do not duplicate it')
    if stage=='smoke4': slots,seconds,tasks=4,1500,[]
    elif stage=='smoke7':
        prior=json.loads((root/'submissions/smoke4.json').read_text())
        collect_accounting(root,prior['job_id'],[0],4)
        for name in ('smoke4','smoke4-guard','linux-native'):
            if json.loads((root/f'receipts/{name}.json').read_text())['status']!='PASS':
                raise ValueError('first smoke prerequisite failed')
        slots,seconds,tasks=7,1500,[]
    elif stage=='screen':
        for mode,slots in [('smoke4',4),('smoke7',7)]:
            prior=json.loads((root/f'submissions/{mode}.json').read_text())
            collect_accounting(root,prior['job_id'],[0],scc_granted_slots(slots))
            for name in (mode,mode+'-guard'):
                if json.loads((root/f'receipts/{name}.json').read_text())['status']!='PASS':
                    raise ValueError('smoke prerequisite failed')
        passed=json.loads((root/'smoke7/manifest.json').read_text())
        for variant in ('baseline','candidate'):
            if verify_package(root/'packages'/variant)!=passed['packages'][variant]:
                raise ValueError('package changed after thread-boundary qualification')
        freeze(root/'screen',root/'full-inputs',root/'packages')
        slots,seconds,tasks=14,2700,['-t','1-36']
    else: raise ValueError('only the two smokes and first screen may be submitted')
    environment=f'PF_RUN_ROOT={root},PF_STAGE={stage}'
    args=['qsub','-terse','-P','welfgr','-pe','omp',str(slots),'-l','mem_per_core=3G',
        '-l',f'h_rt=00:{seconds//60:02d}:00','-j','y','-m','n','-N',f'fevc_pf_{stage}',
        '-o',str(root/'logs'),'-v',environment,*tasks,str(HERE/'screen_job.sge')]
    raw=subprocess.check_output(args,text=True).strip()
    if not re.fullmatch(r'\d+(?:\.[\d:-]+)?',raw): raise ValueError('unrecognized submission receipt')
    job=raw.split('.')[0]
    receipt=dict(status='SUBMITTED',job_id=job,stage=stage,slots=scc_granted_slots(slots),requested_slots=slots,seconds=seconds,command=args,
        snapshot_sha256=sha(root/'snapshot.json'),submitted=time.time(),later_stages_submitted=False)
    write_json(destination,receipt)
    if stage=='screen':
        receipt['manifest_sha256']=sha(root/'screen/manifest.json')
        aggregate=['qsub','-terse','-P','welfgr','-pe','omp','1','-l','mem_per_core=3G',
            '-l','h_rt=00:15:00','-j','y','-m','n','-N','fevc_pf_aggregate','-o',str(root/'logs'),
            '-hold_jid',job,'-v',f'PF_RUN_ROOT={root},PF_STAGE=aggregate',str(HERE/'screen_job.sge')]
        aggregate_id=subprocess.check_output(aggregate,text=True).strip()
        if not aggregate_id.isdigit(): raise ValueError('invalid aggregation job ID')
        receipt.update(aggregate_job_id=aggregate_id,aggregate_command=aggregate)
        write_json(destination,receipt)
    print(json.dumps(receipt))


def aggregate(root):
    submission=json.loads((root/'submissions/screen.json').read_text())
    records=[]
    scheduler_error=None
    try: records=collect_accounting(root,submission['job_id'],range(1,37),14)
    except ValueError as error: scheduler_error=str(error)
    stage=root/'screen'
    if sha(stage/'manifest.json')!=submission['manifest_sha256']: raise ValueError('submitted manifest changed')
    report=validate(json.loads((stage/'manifest.json').read_text()),stage)
    report['scheduler']=records
    if scheduler_error:
        report['errors'].append(scheduler_error)
        report['status']='FAIL'
    for row in records:
        receipt=json.loads((stage/f'tasks/task-{int(row["taskid"]):02d}/task.json').read_text())
        if receipt['job_id']!=submission['job_id'] or receipt['host'].split('.')[0]!=row['hostname'].split('.')[0]:
            report['errors'].append('scheduler and application host/job disagree')
            report['status']='FAIL'
    write_json(stage/'report.json',report)
    print('FEVC_PIPELINE_AGGREGATE_'+report['status'])
    if report['status']!='PASS': raise ValueError('first-stage assessment did not pass; no expansion authorized')


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('root',type=Path)
    p.add_argument('mode',choices=['smoke4','smoke7','smoke4-inner','smoke7-inner','screen','aggregate','submit'])
    p.add_argument('--stage',choices=['smoke4','smoke7','screen'])
    p.add_argument('--stata',default=os.environ.get('PF_STATA','stata-mp'))
    a=p.parse_args()
    if not str(a.root).startswith('/projectnb/welfgr/vckss/runs/') or a.root.is_symlink():
        raise SystemExit('invalid established SCC run path')
    if a.mode=='submit': submit(a.root,a.stage)
    elif a.mode in ('smoke4','smoke7'): guarded(a.root,a.mode,a.stata)
    elif a.mode.endswith('-inner'): smoke(a.root,int(a.mode[5]),a.stata)
    elif a.mode=='aggregate': aggregate(a.root)
    else:
        manifest=a.root/'screen/manifest.json'
        if not run_task(manifest,int(os.environ['SGE_TASK_ID']),a.stata,int(os.environ['NSLOTS']),
            float(os.environ['PF_TASK_STARTED_UNIX']),json.loads((a.root/'submissions/screen.json').read_text())['manifest_sha256']):
            raise SystemExit(1)
