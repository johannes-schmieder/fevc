"""Adversarial schema and independent scalar algebra tests (no native fixture)."""
from copy import deepcopy
from pathlib import Path
import importlib.util
import numpy as np
import pytest

spec = importlib.util.spec_from_file_location('match_interval_audit', Path(__file__).with_name('audit.py'))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


def fixture():
    c = dict(q='q0',cell='null_signal',replication=0,group='rejected',dense=False,
             baseline={'worker':{'semantic_seed':123}})
    rows = [dict(kind='start',cell='null_signal',replication=0,seed=123,dense=False),
            dict(kind='joint',primitive=[1.,0,0,0,1,0,0,0,-1],trace=[0.]*9)]
    rows += [dict(kind='spectrum',target=t,certified=True,values=[0.]*7) for t in range(4)]
    rows += [dict(kind='end',status='failed',replication=0,phase='component_inference_psd',code='JLA_CONSTRAINT_FAILED')]
    return rows,c


def test_complete_rejection_preserved_with_partial_marginals():
    rows,c=fixture()
    result=audit.inspect(audit.validate(rows,c),c)
    assert [r['q0_computable'] for r in result] == [True,True,False,False]


@pytest.mark.parametrize('mutation', ['missing','duplicate','target','seed','phase','success','nonfinite','dimension','partial_dense'])
def test_bad_capture_rejected(mutation):
    rows,c=fixture()
    if mutation=='missing': rows.pop()
    elif mutation=='duplicate': rows.append(deepcopy(rows[1]))
    elif mutation=='target': rows[2]['target']=3
    elif mutation=='seed': rows[0]['seed']=124
    elif mutation=='phase': rows[-1]['phase']='structured_variance_positivity'
    elif mutation=='success': rows[-1]['status']='success'
    elif mutation=='nonfinite': rows[1]['primitive'][0]=float('nan')
    elif mutation=='dimension': rows[1]['trace'].pop()
    else: c['dense']=True
    with pytest.raises(ValueError): audit.validate(rows,c)


def test_unique_capture_anchor_required():
    marker=audit.run.MARKER
    assert audit.run.instrument('a'+marker+'b','CAPTURE') == 'aCAPTURE\n'+marker+'b'
    for text in ('none',marker+marker):
        with pytest.raises(ValueError): audit.run.instrument(text,'CAPTURE')


def test_q0_baseline_allows_only_matching_inapplicable_q1_fields():
    rows,c=fixture(); c['group']='comparison'; rows[-1]['status']='success'
    c['baseline']={t:dict(target=t,semantic_seed=123,status='success',leading_variance=None) for t in audit.run.TARGETS}
    rows.extend(deepcopy(list(c['baseline'].values())))
    audit.validate(rows,c)
    rows[-1]['leading_variance']=1.
    with pytest.raises(ValueError): audit.validate(rows,c)


def test_original_partial_and_duplicate_inventory_fail_closed():
    row=dict(cell='null_signal',replication=0,k=20,target='worker',status='backend_failure',error_phase='component_inference_psd')
    with pytest.raises(ValueError): audit.run.select([row],'q0')
    with pytest.raises(ValueError): audit.run.select([row,row],'q0')


@pytest.mark.parametrize('vb,cross,vr,certified,status',[(1,0,1,True,0),(1,0,-1,True,1),(1,2,1,True,2),(1,0,1,False,6)])
def test_q1_pair_does_not_use_full_joint_covariance(vb,cross,vr,certified,status):
    assert audit.oracle.pair_status(vb,cross,vr,certified)==status


def test_collapsed_design_from_physical_rows_and_target_map():
    # 400 independently declared matches with three unequal-frequency stored rows.
    rng=np.random.default_rng(16)
    group=np.repeat(np.arange(400),3)
    worker=10000+group//20; firm=20000+group%20
    frequency=rng.integers(1,5,len(group)); y=rng.normal(size=len(group)); target=rng.uniform(.1,2,len(group))
    reps=np.arange(0,len(group),3)[::-1]
    inp=dict(worker=worker.tolist(),firm=firm.tolist(),deletion=(group+100000).tolist(),
             frequency=frequency.tolist(),outcome=y.tolist(),target_weight=target.tolist(),true_variance=[.26]*400)
    geom=dict(physical_representative=reps.tolist(),ratio=np.zeros((3,400)).tolist())
    yc,s,maker,plugins,kernels,ratios=audit.matrices(inp,geom)
    assert np.allclose(maker@maker,maker,atol=1e-12)
    assert np.allclose(plugins[3],plugins[0]+plugins[1]+2*plugins[2],atol=1e-12)
    assert np.allclose(yc,[(frequency[3*g:3*g+3]@y[3*g:3*g+3])/np.sqrt(frequency[3*g:3*g+3].sum()) for g in range(399,-1,-1)])
    inv=1/np.diag(maker)
    c=plugins[0]-(np.diag(plugins[0])*inv)[:,None]*maker/2-maker*(np.diag(plugins[0])*inv)[None,:]/2
    assert np.max(np.abs(np.diag(c)))<1e-14
    assert audit.oracle.exact_trace(c,s)>0


def test_ellipse_quartic_against_dense_angle_grid():
    q=[.4,.08,.7]; radius=2.1; lam=.6; center=[.7,-.2]
    endpoints=audit.oracle.ellipse(center,q,radius,lam)
    angle=np.linspace(0,2*np.pi,200001)
    score=center[0]+radius*np.sqrt(q[0])*np.cos(angle)
    remainder=center[1]+radius*(q[1]/np.sqrt(q[0])*np.cos(angle)+np.sqrt(q[2]-q[1]**2/q[0])*np.sin(angle))
    values=lam*score**2+remainder
    assert np.allclose(endpoints,[values.min(),values.max()],atol=1e-9)


def test_quadrature_inversion():
    for curvature in (0.,.1,1.,10.):
        value=audit.oracle.critical_value(curvature)
        assert abs(audit.oracle.critical_cdf(value,curvature)-.95)<1e-10
        assert abs(audit.oracle.critical_cdf(value,curvature,nodes=64)-.95)<1e-7
