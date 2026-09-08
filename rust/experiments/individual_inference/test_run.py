"""Harness failures are independent of the numerical estimator's assertions."""
import copy
import importlib.util
from pathlib import Path
import pytest

spec=importlib.util.spec_from_file_location('individual_campaign',Path(__file__).with_name('run.py'))
run=importlib.util.module_from_spec(spec);spec.loader.exec_module(run)

def fixture():
    task=next(run.tasks('tiny'))
    rows=[{'kind':'task','schema':'individual-v1',**{k:task[k]for k in('family','profile','cell','k','start','reps','master','numseed')}},
          {'kind':'design','truth':[1,1,0,2],'n':144,'p':23,'max_h':.2,'min_variance':1,'leading_share':[.1]*4,'remainder_share':[.1]*4}]
    for rep in range(2):
        rows.append({'kind':'call','replication':rep,'seed':run.seed(task['master'],task['cell'],task['k'],rep),'seconds':.1,'status':'success',
                     'point':[1,1,0,2],'point_mcse':[.01]*4,'targets':[1,0]*4,'q1':[None]*80,'spectrum':[.1]*60,'primitive':[1.]*9,'covariance':[1.]*16,
                     'fit':2,'joint':0,'gram_probes':512,'gram_rcond':[.1],'max_residual':1e-12,'residual_gate':1e-9,'units':144,'solver_columns':512,'counter_atoms':512,'counter_words':1024,'peak':10000,'floored':0,'computed':4,'critical_draws':0})
    return task,rows

def test_complete_and_target_local_accounting():
    task,rows=fixture();assert len(run.validate(rows,task)[1])==2
    rows[2].update(joint=2,primitive=[None]*9,covariance=[None]*16,computed=3,targets=[-1,1]+[1,0]*3)
    assert run.validate(rows,task)[1][0]['computed']==3

@pytest.mark.parametrize('fault',['missing','duplicate','seed','schema','nonfinite','negative','joint','count','gram','critical','unclassified','q1'])
def test_malformed_rejected(fault):
    task,rows=fixture()
    if fault=='missing':rows.pop()
    elif fault=='duplicate':rows[-1]=copy.deepcopy(rows[-2])
    elif fault=='seed':rows[2]['seed']+=1
    elif fault=='schema':rows[0]['schema']='old'
    elif fault=='nonfinite':rows[2]['point'][0]=float('nan')
    elif fault=='negative':rows[2]['targets'][0]=-1
    elif fault=='joint':rows[2]['joint']=2
    elif fault=='count':rows[2]['computed']=3
    elif fault=='gram':rows[2]['gram_probes']=0
    elif fault=='critical':rows[2]['critical_draws']=1
    elif fault=='unclassified':rows[2].update(status='shared_failure',detail='unknown')
    elif fault=='q1':rows[2]['q1'][0]=1
    with pytest.raises(ValueError):run.validate(rows,task)

def test_scientific_failure_is_not_pipeline_failure_or_pass():
    rows=[dict(status='success',point_error=float(i%2),estimated_sd=.01,covered=False)for i in range(400)]
    summary=run.Q1._summary(rows,400)
    failures=run.Q1._gate_correct('fixture',summary)
    assert any('coverage' in f for f in failures)
    assert any('standard-error ratio' in f for f in failures)
    rows[0].update(status='target_1',point_error=100.)
    summary=run.Q1._summary(rows,400)
    assert summary['attempts']==summary['point_estimates']==400
    assert summary['coverage_denominator']==399
    assert summary['bias']>.7

def test_no_unvalidated_development(tmp_path):
    with pytest.raises(ValueError,match='validated tiny'):
        run.campaign(tmp_path/'build',tmp_path/'development','development',1)
    assert not(tmp_path/'development').exists()

def test_task_inventory_and_semantic_keys():
    tiny=list(run.tasks('tiny'));dev=list(run.tasks('development'))
    assert len(tiny)==48 and sum(t['reps']for t in tiny)==96
    assert len(dev)==192 and sum(t['reps']for t in dev)==19200
    assert len({t['id']for t in dev})==192
    assert len({(t['family'],t['cell'],t['k'],r)for t in dev for r in range(t['start'],t['start']+t['reps'])})==19200
    assert run.seed(run.Q1.MASTER_SEED,'one_mode_equal_independent',20,0)==4156217649202129127
