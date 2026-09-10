"""Freeze a larger approximate-only comparison using hash-verified adapters."""
import argparse,csv,hashlib,json,shutil
from pathlib import Path
from reference import fixture

ROLES=('fevc','matlab','julia','r','pytwoway')
SEEDS=(202609091,202609193,202609299)
HERE=Path(__file__).parent

def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def write(p,x): p.write_text(json.dumps(x,indent=2,sort_keys=True)+'\n')
def replace(text,old,new):
    if text.count(old)!=1: raise ValueError(f'adapter anchor changed: {old}')
    return text.replace(old,new,1)

def prepare(base,out,n):
    if out.exists(): raise ValueError('run already exists')
    worker,firm,period,y,ref=fixture(n)
    if ref['correction_fraction_of_plugin']['worker']<.25 or ref['correction_fraction_of_plugin']['firm']<.2:
        raise ValueError('fixed draw lacks substantial bias correction')
    code=out/'code';inputs=out/'input';code.mkdir(parents=True);inputs.mkdir()
    for line in (base/'bundle.sha256').read_text().splitlines():
        digest,rel=line.split(maxsplit=1)
        if sha(base/rel)!=digest: raise ValueError(f'base hash mismatch: {rel}')
        if rel.startswith('code/'):
            target=out/rel;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(base/rel,target)
    transforms={
      'common.py':('(*SIZE_ROWS, 960, 3840)',f'(*SIZE_ROWS, 960, 3840, {n})'),
      'fevc_run.do':("inlist(`rows',960,3840,7680",f"inlist(`rows',960,3840,{n},7680"),
      'matlab_run.m':('[960 3840 7680',f'[960 3840 {n} 7680'),
      'julia_run.jl':('(960,3840,7680',f'(960,3840,{n},7680'),
      'r_run.R':('c(960L,3840L,7680L',f'c(960L,3840L,{n}L,7680L'),
      'pytwoway_run.py':('(960, 3_840, 7_680',f'(960, 3_840, {n}, 7_680'),
      'generate_input.py':('== 3840 and',f'== {n} and')}
    for name,(old,new) in transforms.items():
        p=code/name;p.write_text(replace(p.read_text(),old,new))
    for name in ('reference.py','prepare.py','summarize_large.py','run_large.sge','PROTOCOL.md'):
        shutil.copyfile(HERE/name,code/('prepare_large.py' if name=='prepare.py' else name))
    with (inputs/'fixture.csv').open('w',newline='') as f:
        wr=csv.writer(f,lineterminator='\n');wr.writerow(('observation_key','worker','firm','period','match','y'))
        for i in range(n):wr.writerow((i+1,int(worker[i])+1,int(firm[i])+1,int(period[i]),i+1,format(y[i],'.17g')))
    receipt=dict(schema='FEVC-LARGE-NOISY-INPUT-V1',status='PASS',rows=n,workers=n//3,firms=n//120,
      sha256=sha(inputs/'fixture.csv'),mean_centered=True,unique_worker_firm=True,all_workers_move=True,
      seed_worker=20260909201,seed_firm=20260909202,seed_noise=20260909203)
    write(inputs/'fixture.json',receipt);write(inputs/'oracle.json',ref)
    tasks=[dict(task_id=i+1,cell_id='large-noisy-jla',sweep='diagnostic',rows=n,workers=n//3,firms=n//120,
       cores=2,repeat=i+1,seed=seed,probes=280,algorithm='jla',role_order=','.join(ROLES[i:]+ROLES[:i]))
       for i,seed in enumerate(SEEDS)]
    with (inputs/'tasks.tsv').open('w',newline='') as f:
        wr=csv.DictWriter(f,fieldnames=list(tasks[0]),delimiter='\t',lineterminator='\n');wr.writeheader();wr.writerows(tasks)
    write(inputs/'identity.json',dict(schema='FEVC-LARGE-APPROX-V1',status='FROZEN',rows=n,tasks=3,calls=15,
      cores=2,probes=280,prior_run=str(base),comparator_estimation_code_changed=False,
      manifest_sha256=sha(inputs/'tasks.tsv'),reference_sha256=sha(inputs/'oracle.json'),data=receipt,
      julia_rng='Fixed internal thread streams; caller seeds not independent projection draws'))
    paths=sorted(p for folder in (code,inputs) for p in folder.rglob('*') if p.is_file())
    (out/'bundle.sha256').write_text(''.join(f'{sha(p)}  {p.relative_to(out)}\n' for p in paths))
    print(json.dumps(ref,indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--base',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--rows',type=int,default=76800)
    a=p.parse_args();prepare(a.base,a.output,a.rows)
