"""Deterministic moment oracle and input/transport guard regressions."""
import copy
import importlib.util
import itertools
from pathlib import Path

import numpy as np
import pytest

spec = importlib.util.spec_from_file_location('population_audit', Path(__file__).with_name('audit.py'))
audit = importlib.util.module_from_spec(spec); spec.loader.exec_module(audit)


def test_gaussian_moments_against_deterministic_quadrature():
    # Six Hermite nodes integrate every polynomial here exactly; no outcome RNG.
    k = np.array([[.8,-.2,.1],[-.2,-.4,.3],[.1,.3,.6]])
    mu = np.array([.3,-.7,1.2]); s=np.array([.6,1.3,.4])
    v = np.array([.5,-.3,.2]); v/=np.linalg.norm(v)
    x,w=np.polynomial.hermite.hermgauss(6)
    indices=np.array(list(itertools.product(range(6),repeat=3)))
    y=mu+np.sqrt(2*s)*x[indices]
    weights=np.prod(w[indices]/np.sqrt(np.pi),axis=1)
    values=np.einsum('ni,ij,nj->n',y,k,y)
    mean=weights@values; centered=values-mean
    result=audit.gaussian_quadratic(k,mu,s)
    assert result['mean']==pytest.approx(mean,abs=1e-12)
    assert result['variance']==pytest.approx(weights@centered**2,rel=1e-12)
    assert result['fourth_central_moment']==pytest.approx(weights@centered**4,rel=1e-12)
    score=y@v; score-=weights@score
    assert weights@(score*centered)==pytest.approx(2*np.dot(v*s,k@mu),abs=1e-12)
    second=np.diag([.1,.7,-.3])
    q2=np.einsum('ni,ij,nj->n',y,second,y)
    cov=audit.covariance_of_quadratics([k,second],mu,s)
    assert cov[0,1]==pytest.approx(weights@(centered*(q2-weights@q2)),abs=1e-12)


def test_normal_sample_variance_mcse():
    assert audit.sample_variance_mcse(3.,27.,400)==pytest.approx(3*np.sqrt(2/399))
    with pytest.raises(ValueError):audit.sample_variance_mcse(3.,1.,400)


@pytest.mark.parametrize('kind',['nonsymmetric','nan','nonpositive','shape'])
def test_moment_input_rejections(kind):
    k=np.eye(2);mu=np.ones(2);s=np.ones(2)
    if kind=='nonsymmetric':k[0,1]=.3
    if kind=='nan':mu[0]=np.nan
    if kind=='nonpositive':s[0]=0
    if kind=='shape':k=np.eye(3)
    with pytest.raises(ValueError):audit.gaussian_quadratic(k,mu,s)


def fake_inventory():
    entry={'truth':[1,2,3,4],'master':7,'calls':{i:dict(kind='call',replication=i,seed=i+9,seconds=.1) for i in range(400)}}
    rows=[dict(kind='task',family='observation',profile='development',start=0,reps=400,master=7,numseed=8675309),
          dict(kind='design',truth=entry['truth']),dict(kind='population',mean=[0],variance=[1])]
    for rep in range(400):
        rows.append(dict(kind='input',replication=rep,seed=rep+9,worker=[1],firm=[1],deletion=[1],frequency=[1],target_weight=[1],controls=[],outcome=[rep]))
        if rep in (0,399):
            rows.append(dict(kind='geometry',members=[[0]],maker_inverse=[1],ratio=[[1]]))
            rows.extend(dict(kind='vectors',target=t,mode=[1],ratio=[1],eigenvalue=1) for t in range(4))
            rows.append(entry['calls'][rep])
    return rows,entry


def test_complete_inventory():
    rows,entry=fake_inventory()
    assert len(audit.validate(rows,'observation',entry)['input'])==400


def test_within_unit_order_is_not_unit_membership():
    rows,entry=fake_inventory()
    geometry=[r for r in rows if r['kind']=='geometry']
    geometry[0]['members']=[[0,1]];geometry[1]['members']=[[1,0]]
    audit.validate(rows,'observation',entry)
    geometry[1]['members']=[[1,2]]
    with pytest.raises(ValueError):audit.validate(rows,'observation',entry)


@pytest.mark.parametrize('kind',['missing','duplicate','seed','geometry','native','target','misassociated'])
def test_inventory_rejections(kind):
    rows,entry=fake_inventory(); rows=copy.deepcopy(rows)
    inputs=[r for r in rows if r['kind']=='input']
    if kind=='missing':rows.remove(inputs[7])
    if kind=='duplicate':inputs[7]['replication']=6
    if kind=='seed':inputs[7]['seed']+=1
    if kind=='geometry':next(r for r in rows if r['kind']=='geometry')['maker_inverse']=[2]
    if kind=='native':next(r for r in rows if r['kind']=='call')['seed']+=1
    if kind=='target':next(r for r in rows if r['kind']=='vectors')['target']=9
    if kind=='misassociated':
        g=next(r for r in rows if r['kind']=='geometry');rows.remove(g);rows.append(g)
    with pytest.raises(ValueError):audit.validate(rows,'observation',entry)


def test_target_mass_and_collapse_units():
    # Unequal physical frequencies: target mass must NOT be frequency weighted.
    inp=dict(worker=[0,0,0,0,1,1,1,1],firm=[0,0,1,1,0,0,1,1],deletion=[0,0,1,1,2,2,3,3],
             frequency=[1,2,2,3,3,4,4,5],target_weight=[1,3,1,1,2,1,4,2],controls=[],outcome=list(range(8)))
    pop=dict(mean=[.3,.3,.7,.7,1.3,1.3,1.7,1.7],variance=[.2,.4,.6,.8])
    geom=dict(members=[[6,7],[2,3],[0,1],[4,5]],ratio=np.zeros((3,4)).tolist())
    collapse,mu,s,maker,plugins,*_=audit.matrices(inp,pop,geom,'match_q1')
    np.testing.assert_allclose(s,[.8,.4,.2,.6])
    np.testing.assert_allclose(mu,np.sqrt([9,5,3,7])*[1.7,.7,.3,1.3])
    raw=np.array([0,0,0,0,1,1,1,1],dtype=float)
    tw=np.array(inp['target_weight']);tw=tw/tw.sum()
    assert mu@plugins[0]@mu==pytest.approx(tw@(raw-(tw@raw))**2)
    bad=copy.deepcopy(geom);bad['members'][0]=[0,7]
    with pytest.raises(ValueError):audit.matrices(inp,pop,bad,'match_q1')


def test_instrumentation_rejects_nonunique_anchor():
    with pytest.raises(ValueError):audit.run.replace_once('aa','a','b')
    with pytest.raises(ValueError):audit.run.replace_once('aa','c','b')
