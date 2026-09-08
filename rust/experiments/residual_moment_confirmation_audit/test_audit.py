import importlib.util
from pathlib import Path
import numpy as np

spec=importlib.util.spec_from_file_location('audit',Path(__file__).with_name('run.py'))
audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)

def test_factorized_probe_quadratics_match_independent_dense_forms():
    rng=np.random.default_rng(41296);x=rng.normal(size=(11,3));r=rng.normal(size=(3,11))
    f=rng.normal(size=(3,6,6));f=(f+f.transpose(0,2,1))/2;g=rng.normal(size=(17,11))
    c=audit.kernels(x,r,f);q=audit.quadratic_draws(x,r,f,g)
    expected=np.array([np.einsum('ri,ij,rj->r',g,t,g) for t in c])
    np.testing.assert_allclose(q,expected,atol=1e-10,rtol=1e-12)

def test_exact_trace_matches_explicit_matrix_products():
    rng=np.random.default_rng(5316);c=rng.normal(size=(3,9,9));c=(c+c.transpose(0,2,1))/2
    y=rng.normal(size=9);s=np.exp(rng.normal(size=9));d=np.diag(s)
    actual,_=audit.exact_covariance(c,y,s)
    expected=np.array([[4*y@l@d@r@y-2*np.trace(l@d@r@d) for r in c] for l in c])
    np.testing.assert_allclose(actual,expected,atol=1e-10,rtol=1e-12)

def test_positive_primary_marginals_do_not_imply_joint_validity():
    c=np.array([[1.,2.,0.],[2.,1.,0.],[0.,0.,1.]])
    d=audit.describe(c)
    assert d['all_primary_marginals_positive']
    assert not d['dense_registered_psd_pass']
    assert not d['worker_firm_principal_psd_pass']

def test_nonpositive_diagonal_is_rejected_without_psd_clipping():
    d=audit.describe(np.diag([1.,1.,0.]))
    assert not d['dense_registered_psd_pass']
    assert d['worker_firm_principal_psd_pass']
