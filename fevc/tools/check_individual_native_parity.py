"""Compare the public Stata command to frozen native campaign inputs/results."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import subprocess

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('individual_parity_campaign',ROOT/'rust/experiments/individual_inference/run.py')
run=importlib.util.module_from_spec(spec);spec.loader.exec_module(run)

def check(build,plugin,out):
    out.mkdir(parents=True,exist_ok=False)
    cases=[('observation','diffuse_common',12),('observation','dominant_common',12),('observation','dominant_common_controls',16),
           ('match_q0','diffuse_equal_independent',20),('match_q1','one_mode_equal_independent',20),('match_q1','controls_varying_fixedoffset',20)]
    receipts=[]
    for family,cell,k in cases:
        task=next(t for t in run.tasks('tiny')if(t['family'],t['cell'],t['k'])==(family,cell,k))
        task=dict(task,reps=1)
        folder=out/f'{family}-{cell}-{k}';folder.mkdir()
        fixture=folder/'fixture.csv'
        command=[str(build/family),'tiny',cell,str(k),'0','1',str(task['master']),'individual-v1']
        native=subprocess.run(command,capture_output=True,text=True,env=dict(os.environ,FEVC_INDIVIDUAL_FIXTURE=str(fixture)),check=True)
        (folder/'native.jsonl').write_text(native.stdout)
        d,calls=run.validate([json.loads(x)for x in native.stdout.splitlines()],task)
        row=calls[0];assert row['status']=='success',row
        c=run.spec_for(task);q1=run.is_q1(c)
        controls='control*'if c['controls']else''
        deletion='deletion(observation)'if family=='observation'else'deletion(match) deletionid(deletion) nuisance(fixedoffset)'
        weights=''if family=='observation'else'[fw=frequency]'
        model=c['variance_model'];reference='q1'if q1 else'highrank'
        options=f'worker(worker) firm(firm) {deletion} stayers(movers) backend(rust) engine(generic) rng(counter_v1) algorithm(jla) preconditioner(diagonal) batch(16) targetweight(target) inferencemodel({model}) inference({reference})'
        lines=['version 18.0','clear all','set more off','set type double','set processors 1',f'adopath ++ "{ROOT}/fevc"',f'adopath ++ "{plugin}"',
               f'quietly run "{ROOT}/fevc/fevc.ado"',f'import delimited using "{fixture}", clear asdouble',f'quietly fevc outcome {controls} {weights}, {options}',
               'assert e(probes)==200','assert e(inference_simulations)==1000',
               f'assert e(inference_joint_status)=={row["joint"]}',f'assert e(inference_computed_targets)=={row["computed"]}',
               f'assert e(inference_critical_draws)=={row["critical_draws"]}']
        def close(expr,value):
            if value is None:lines.append(f'assert missing({expr})')
            else:lines.append(f'assert abs({expr}-({value:.17e}))<=1e-7*max(1,abs({value:.17e}))')
        for i in range(4):
            close(f'e(b)[1,{i+1}]',row['point'][i])
            close(f'e(q0_status)[{i+1},1]',row['targets'][2*i+1])
            if q1:
                for j in range(20):close(f'e(component_q1_diagnostics)[{i+1},{j+1}]',row['q1'][20*i+j])
            elif row['targets'][2*i+1]==0:close(f'e(component_inference)[{i+1},2]^2',row['targets'][2*i])
        lines+=['quietly fevc_rust snapshot','assert r(state)==0','display "INDIVIDUAL NATIVE PARITY PASS"','exit, clear']
        (folder/'check.do').write_text('\n'.join(lines)+'\n')
        subprocess.run(['/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp','-b','do','check.do'],cwd=folder,timeout=180,check=True)
        log=folder/'check.log'
        if not log.exists()or'INDIVIDUAL NATIVE PARITY PASS\n'not in log.read_text():raise ValueError(f'Stata parity failed: {folder}')
        receipts.append({'family':family,'cell':cell,'k':k,'status':'PASS','fixture_sha256':run.sha(fixture),'native_sha256':run.sha(folder/'native.jsonl'),'log_sha256':run.sha(log)})
    receipt={'status':'PASS','tolerance':'1e-7 * max(1,abs(native))','build_receipt_sha256':run.sha(build/'receipt.json'),
             'script_sha256':run.sha(Path(__file__)),'plugin_sha256':run.sha(plugin/'fevc_rust_macos_arm64.plugin'),'cases':receipts}
    run.write(out/'receipt.json',receipt)
    return receipt

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('build',type=Path);p.add_argument('plugin',type=Path);p.add_argument('output',type=Path)
    a=p.parse_args();print(json.dumps(check(a.build.resolve(),a.plugin.resolve(),a.output.resolve()),indent=2))
