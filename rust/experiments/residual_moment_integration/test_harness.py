"""Adversarial inventory and scientific-failure accounting checks."""
import copy
import importlib.util
from pathlib import Path
import pytest

spec=importlib.util.spec_from_file_location('native_run',Path(__file__).with_name('run.py'))
run=importlib.util.module_from_spec(spec)
spec.loader.exec_module(run)

@pytest.fixture
def sample():
    task={'cell':'diffuse_common','k':16,'start':0,'reps':1,'master':run.PLAN['outcome_master'],'numseed':570239}
    n=100
    rows=[{'kind':'design','cell':task['cell'],'k':16,'n':n,'p':31,'truth':[1]*4,'leading_share':[.1]*4,'remainder_share':[.1]*4,'max_h':.2,'min_variance':1},
          {'kind':'call','replication':0,'status':'success','seconds':1,'projection_count':512,'gram_atoms':512*n,'gram_words':1024*n,'projection_residual':1e-12,'projection_gate':1e-9,'full_residual':1e-12,'full_gate':1e-9,'h':[.2]*n,'b':[[.01]*n]*3,'gram':[1.0]*225,'atoms':1642*n,'words':3284*n,'rcond':.1,'memory':10000,'floored':0}]
    for arm in run.ARMS:
        for target in run.TARGETS:
            rows.append({'kind':'target','cell':task['cell'],'k':16,'replication':0,'seed':run.old.semantic_seed(task['master'],task['cell'],16,0),'numseed':task['numseed'],'arm':arm,'target':target,'status':'success','point_error':0.0,'variance':1.0,'estimated_sd':1.0,'width':3.92,'covered':True,'lower_miss':False,'upper_miss':False,'point_kernel_identity':0.0})
    return rows,task

def test_complete_inventory(sample):
    rows,t=sample
    assert len(run.validate(rows,t)[2])==8

@pytest.mark.parametrize('change',[
    lambda r:r.pop(), lambda r:r.append(copy.deepcopy(r[-1])),
    lambda r:r[-1].update(seed=1), lambda r:r[-1].update(numseed=1),
    lambda r:r[-1].update(k=24),lambda r:r[-1].update(arm='unknown'),
    lambda r:r[-1].update(variance=float('nan')),
    lambda r:r[-1].pop('width'),lambda r:r[-1].update(covered=1),
    lambda r:r[-1].update(variance=-1),lambda r:r[-1].update(estimated_sd=2),
    lambda r:r[1].update(projection_count=511),lambda r:r[1].update(projection_residual=1),
    lambda r:r[1].update(gram_words=1),lambda r:r[1].update(words=1),
    lambda r:r[1].update(h=[]),lambda r:r[0].update(leading_share=[.9]*4),
    lambda r:r[2].update(point_kernel_identity=.1),
    lambda r:r.append({'kind':'garbage'}),
])
def test_corruption_rejected(sample,change):
    rows,t=sample;change(rows)
    with pytest.raises((ValueError,KeyError)):run.validate(rows,t)

def test_scientific_call_failure_is_counted(sample):
    rows,t=sample
    rows[1]={'kind':'call','replication':0,'status':'SINGULAR_INFORMATION','phase':'test','detail':'deliberate rank failure','seconds':0}
    for r in rows[2:]:r['status']='native_call_failed'
    targets=run.validate(rows,t)[2]
    for s in run.summary(targets):
        assert s['attempted']==1 and s['success']==0 and s['coverage_all']==0
        assert s['failures']=={'native_call_failed':1}

def test_inconsistent_failure_rejected(sample):
    rows,t=sample;rows[-1]['status']='native_call_failed'
    with pytest.raises(ValueError):run.validate(rows,t)

def test_all_misses_are_not_silently_dropped(sample):
    rows,t=sample
    for r in rows[2:]:r.update(covered=False,upper_miss=True)
    for s in run.summary(run.validate(rows,t)[2]):assert s['success']==1 and s['coverage_all']==0

def test_geometry_outcome_dependence_rejected(sample):
    rows,t=sample;t['reps']=2
    additional=copy.deepcopy(rows[1:]);additional[0]['h'][0]=.3
    for r in additional:
        r['replication']=1
        if r['kind']=='target':r['seed']=run.old.semantic_seed(t['master'],t['cell'],16,1)
    with pytest.raises(ValueError,match='outcome-dependent'):run.validate(rows+additional,t)
