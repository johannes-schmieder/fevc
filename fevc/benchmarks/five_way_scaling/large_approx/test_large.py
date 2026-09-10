import importlib.util,sys,json,csv,hashlib
from pathlib import Path
import numpy as np
import pytest

HERE=Path(__file__).parent
def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
ref=module('large_reference',HERE/'reference.py')
old=module('old_noisy',HERE.parent/'noisy_comparison/prepare.py')

def test_schur_matches_dense_small_and_prior_fixture():
    worker=np.repeat(np.arange(12),3);firm=(np.arange(36)%3+worker)%5
    y=np.random.default_rng(81).normal(size=36);y-=y.mean()
    for w,f,v in [(worker,firm,y),old.fixture(3840)[:2]+(old.fixture(3840)[3],)]:
        a=ref.reference(w,f,v,block=17);b=old.dense_reference(w,f,v)
        for section in ('targets','plugin','correction'):
            for k in ref.TARGETS:assert a[section][k]==pytest.approx(b[section][k],abs=1e-10)

def test_fixture_is_same_dgp_at_original_size():
    a=ref.fixture(960);b=old.fixture(960)
    for x,y in zip(a[:4],b[:4]):assert np.array_equal(x,y)

def test_reference_rejects_invalid_design():
    with pytest.raises(ValueError):ref.fixture(1000)
    with pytest.raises(ValueError):ref.reference(np.repeat(np.arange(12),3),np.zeros(36,dtype=int),np.zeros(36))

def test_collector_accepts_disagreement_and_rejects_corruption(tmp_path):
    m=module('large_summary',HERE/'summarize_large.py');inputs=tmp_path/'input';inputs.mkdir()
    def write(p,x):p.write_text(json.dumps(x))
    def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
    (inputs/'fixture.csv').write_text('fixture\n');write(inputs/'fixture.json',dict(sha256=digest(inputs/'fixture.csv')))
    values=dict(worker=.25,firm=.07,covariance=-.003,total=.314)
    write(inputs/'oracle.json',dict(targets=values,correction={k:1. for k in values}))
    tasks=[]
    for i in range(1,4):
        task=dict(task_id=i,repeat=i,seed=100+i,algorithm='jla',rows=76800,cores=2,probes=280);tasks.append(task)
        folder=tmp_path/'output/smoke'/f'task-{i:03}';folder.mkdir(parents=True)
        (folder/'wrapper.pass').touch();write(folder/'task.json',task);write(folder/'input.json',dict(sha256=digest(inputs/'fixture.csv')))
        for role in m.ROLES:
            d=folder/role;d.mkdir();write(d/'status.json',dict(status='PASS'))
            write(d/'process_tree.json',dict(status='PASS',phase_start_observed=True,phase_end_observed=True,phase_sample_count=2,phase_peak_rss_kib=2048))
            factor=76799/76800 if role in ('matlab','julia','r') else 1.
            result=dict(task,status='PASS',role=role,retained_rows=76800,normalization_factor=factor,primary_seconds=1.)
            for k,v in values.items():result['normalized_'+k]=v*2;result['raw_'+k]=v*2/factor
            if role=='fevc':m.writecsv(d/'result.csv',[result])
            else:write(d/'result.json',result)
            (d/'application.txt').write_text(f'FEVC_FIVE_WAY_ROLE_PASS {role} jla rows=76800 cores=2')
    with (inputs/'tasks.tsv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(tasks[0]),delimiter='\t');w.writeheader();w.writerows(tasks)
    write(inputs/'identity.json',dict(rows=76800,manifest_sha256=digest(inputs/'tasks.tsv'),reference_sha256=digest(inputs/'oracle.json')))
    calls,checks,_=m.load(tmp_path);assert len(calls)==15 and len(checks)==60
    assert all(c['absolute_gap']>0 for c in checks)
    path=tmp_path/'output/smoke/task-003/r/result.json';saved=json.loads(path.read_text())
    for key,value in [('seed',0),('retained_rows',1),('normalized_worker',float('nan')),('raw_worker',1.)]:
        write(path,dict(saved,**{key:value}))
        with pytest.raises(ValueError):m.load(tmp_path)
    write(path,saved);path.unlink()
    with pytest.raises(FileNotFoundError):m.load(tmp_path)
