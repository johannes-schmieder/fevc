"""Production-independent identities and hostile-result harness tests."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
from types import SimpleNamespace

import numpy as np
import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("pooled_component_assessment", HERE / "run.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
from pooled_oracle import build_design, evaluate, gaussian_covariance, mean_kernel, moments, seed


@pytest.fixture(scope="module")
def design():
    return build_design(12, "rust", "q1")


def test_kernel_point_and_whole_match_oracle(design):
    d=design
    y=d.draw("unit", "fixture", 1)
    c=d.direction@y/(d.direction@d.direction)
    z=y-c*d.direction
    inverse=np.linalg.inv(d.x.T@d.x)
    coefficients=inverse@d.x.T@y
    fit=d.x@coefficients
    h=np.diag(d.x@inverse@d.x.T)
    deleted=(y-fit)/(1-h)
    direct=np.einsum("i,tij,j->t",y,d.plugin,y)-np.diagonal(d.plugin,axis1=1,axis2=2)@(z*deleted)
    np.testing.assert_allclose(evaluate(d,y)["point"],direct,atol=2e-12)
    for unit in (0,8,20):
        keep=np.arange(len(y))!=unit
        beta=np.linalg.lstsq(d.x[keep],y[keep],rcond=None)[0]
        assert deleted[unit]==pytest.approx(y[unit]-d.x[unit]@beta,abs=1e-12)
    shift=np.eye(len(y))-np.outer(d.direction,d.direction)/(d.direction@d.direction)
    for target in range(4):
        np.testing.assert_allclose(mean_kernel(d.kernels,d.direction)[target],shift@d.kernels[target]@shift,atol=1e-14)


def test_shift_scale_q1_and_accounting(design):
    d=design;y=d.draw("unit","fixture",3)
    a=evaluate(d,y);b=evaluate(d,y+200*d.direction);scaled=evaluate(d,.01*y)
    np.testing.assert_allclose(a["point"],b["point"],atol=2e-12)
    np.testing.assert_allclose(a["q1_score"],b["q1_score"],atol=2e-12)
    np.testing.assert_allclose(a["point"],np.asarray(scaled["point"])/.01**2,atol=2e-12)
    assert a["point"][3]==pytest.approx(a["point"][0]+a["point"][1]+2*a["point"][2],abs=1e-12)
    inverse=np.linalg.inv(d.x.T@d.x)
    projection=d.x@inverse@d.x.T
    z=y-a["mean"]*d.direction
    residual=(np.eye(len(y))-projection)@y/(1-np.diag(projection))
    raw=(d.modes*d.modes)@(z*residual)
    decomposition=np.asarray(a["q1_remainder"])+d.eigenvalues*(np.asarray(a["q1_score"])**2-raw)
    np.testing.assert_allclose(a["point"],decomposition,atol=3e-12)


def test_gaussian_moments_against_independent_draws():
    rng=np.random.default_rng(728319)
    n=5
    raw=rng.normal(size=(2,n,n)); kernels=(raw+raw.transpose(0,2,1))/2
    mu=rng.normal(size=n);variance=np.arange(1,n+1)/3
    expectation,covariance=gaussian_covariance(kernels,mu,variance)
    y=mu[:,None]+np.sqrt(variance)[:,None]*rng.normal(size=(n,100000))
    values=np.einsum("ir,tij,jr->tr",y,kernels,y)
    np.testing.assert_allclose(values.mean(axis=1),expectation,atol=.04)
    # Cross-covariances near zero need sampling-error bounds, not a relative
    # error criterion. Fourth moments of Gaussian quadratic forms are finite.
    products=(values-expectation[:,None])[:,None,:]*(values-expectation[:,None])[None,:,:]
    covariance_mcse=np.std(products,axis=2,ddof=1)/np.sqrt(values.shape[1])
    assert np.all(np.abs(products.mean(axis=2)-covariance) < 6*covariance_mcse)


def test_actual_mean_population_bias_and_difference(design):
    d=design;m=moments(d)
    np.testing.assert_allclose(m["fixed_c0"]["expectation"],d.truth,atol=1e-12)
    actual=mean_kernel(d.kernels,d.direction)
    expected_bias=np.diagonal(actual,axis1=1,axis2=2)@d.variance
    np.testing.assert_allclose(m["actual_mean"]["bias"],expected_bias,atol=2e-12)
    np.testing.assert_allclose(m["difference"]["expectation"],expected_bias,atol=2e-12)


def test_semantic_rng_order_and_sharding(design):
    expected={r:design.draw("development","same_cell",r) for r in range(1,9)}
    split={r:design.draw("development","same_cell",r) for shard in ([8,6,4,2],[7,5,3,1]) for r in shard}
    for r in expected: np.testing.assert_array_equal(expected[r],split[r])
    assert seed("pipeline","cell",1)!=seed("assessment","cell",1)
    assert len(runner.cells("assessment"))==8
    assert len({c["cell"] for c in runner.cells("assessment")})==8
    assert runner.primary_targets(dict(reference="q1"))==("worker","firm","total")


def fixture_rows(design):
    cell=dict(cell="fixture",route="mata",reference="highrank",k=12,diagnostic="primary",stayer_share=.25)
    task=dict(cell=cell,replications=[1,2],c0=design.c0)
    manifest=dict(identity="source_hash",profile="smoke",domain="pipeline",
                  registration=json.loads(runner.REGISTRATION.read_text()),
                  cells={"fixture":dict(spec=cell,oracle=moments(design))})
    rows=[]
    for rep in task["replications"]:
        for arm in runner.ARMS:
            for i,target in enumerate(runner.TARGETS):
                truth=design.truth[i]
                rows.append(dict(cell="fixture",replication=rep,arm=arm,target=target,rc=0,target_status=0,sample_ok=1,
                    n=design.diagnostics["stored_rows"],physical_n=design.diagnostics["physical_rows"],point=truth,
                    se=.1,lower=truth-.2,upper=truth+.2,centering="mean" if arm=="mean" else "none",
                    inference_centering="fixed observed mean" if arm=="mean" else "uncentered",failure="",identity_error=0.,
                    point_mcse=.01,mcse_available=1,mcse_mode="all",mcse_centering="fixed observed mean" if arm=="mean" else "uncentered",failure_phase="",
                    q1_var_b=1.,q1_cov_br=.2,q1_var_r=2.,q1_b=.5,q1_remainder=truth-.25,
                    point_probes=200,gram_probes=2048,covariance_simulations=1000,
                    schema=runner.SCHEMA,profile="smoke",manifest_sha256="source_hash",
                    outcome_seed=seed("pooled-component-v1","pipeline","fixture",rep),
                    numerical_seed=runner.numerical_seed("fixture"),oracle_point=truth))
    return rows,task,manifest


@pytest.mark.parametrize("damage",["missing","duplicate","partial","source","seed","profile","count","sample","point","identity","interval","failure","schema","centering_metadata","mcse_centering","mixed_atomic","unknown_status","interpreter"])
def test_bad_results_rejected(design,damage):
    rows,task,manifest=fixture_rows(design)
    if damage=="missing": rows.pop()
    if damage=="duplicate": rows[-1]=copy.deepcopy(rows[0])
    if damage=="partial": rows=rows[:4]
    if damage=="source": rows[0]["manifest_sha256"]="other"
    if damage=="seed": rows[0]["outcome_seed"]+=1
    if damage=="profile": rows[0]["profile"]="assessment"
    if damage=="count": rows[0]["physical_n"]+=1
    if damage=="sample": rows[0]["sample_ok"]=0
    if damage=="point": rows[0]["point"]+=1
    if damage=="identity": rows[0]["identity_error"]=1e-4
    if damage=="interval": rows[0]["lower"]=None
    if damage=="failure": rows[0]["rc"]=498
    if damage=="schema": rows[0]["schema"]="wrong"
    if damage=="centering_metadata": rows[0]["inference_centering"]="wrong"
    if damage=="mcse_centering": rows[0]["mcse_centering"]="wrong"
    if damage=="mixed_atomic": rows[0].update(rc=498,failure="NEGATIVE_INFERENCE_VARIANCE")
    if damage=="unknown_status": rows[0]["target_status"]=99
    if damage=="interpreter":
        for row in rows: row.update(rc=199,failure="stata_rc_199")
    with pytest.raises(ValueError): runner.validate_rows(rows,task,manifest)


def test_valid_reordering_failure_accounting_and_screens(design):
    rows,task,manifest=fixture_rows(design)
    runner.validate_rows(rows[::-1],task,manifest)
    for row in rows:
        row.update(rc=498,target_status=-1,failure="NEGATIVE_INFERENCE_VARIANCE",point=None,se=None,lower=None,upper=None)
    runner.validate_rows(rows,task,manifest)
    result=runner.summarize(rows,manifest)
    assert result["status"]=="SCREEN_FAILURE"
    assert len(result["attempt_failures"])==len(rows)
    assert all(r["success_rate"]==0 and r["coverage_all"]==0 for r in result["summaries"])
    assert any("success_rate" in r["screens"] for r in result["scientific_screen_failures"])


def test_native_point_uses_registered_mcse_gate(design):
    rows,task,manifest=fixture_rows(design)
    task["cell"]["route"]="rust"
    rows[0]["point"]+=.04
    runner.validate_rows(rows,task,manifest)
    rows[0]["point"]+=1e6
    with pytest.raises(ValueError,match="dense point"): runner.validate_rows(rows,task,manifest)


def test_q1_does_not_require_highrank_se_but_requires_joint_covariance(design):
    rows,task,manifest=fixture_rows(design)
    task["cell"]["reference"]="q1"
    for row in rows: row["se"]=None
    runner.validate_rows(rows,task,manifest)
    result=runner.summarize(rows,manifest)
    assert all(r["highrank_se_available"]==0 for r in result["summaries"])
    rows[0]["identity_error"]=None
    with pytest.raises(ValueError,match="lacks remainder"): runner.validate_rows(rows,task,manifest)
    rows[0]["identity_error"]=0
    rows[0]["q1_var_b"]=None
    with pytest.raises(ValueError,match="nonfinite"): runner.validate_rows(rows,task,manifest)


def test_mean_approximation_screen_is_enforced(design):
    rows,_,manifest=fixture_rows(design)
    manifest["cells"]["fixture"]["oracle"]["difference"]["rms_over_actual_sd"][0]=.5
    result=runner.summarize(rows,manifest)
    assert any("mean_approximation" in row["screens"] for row in result["scientific_screen_failures"])


def test_fixed_outcome_numerical_grid_and_key_invariance():
    grid=runner.cells("numerical")
    assert len(grid)==48 and len({c["cell"] for c in grid})==48
    assert len({runner.outcome_cell(c) for c in grid})==4
    for key in {runner.outcome_cell(c) for c in grid}:
        selected=[c for c in grid if runner.outcome_cell(c)==key]
        assert len(selected)==12
        d=build_design(12,selected[0]["route"],selected[0]["reference"])
        expected=d.draw("numerical_fixed_outcome",key,1)
        for c in reversed(selected):
            renamed=dict(c,cell="arbitrary_task_label")
            np.testing.assert_array_equal(expected,d.draw("numerical_fixed_outcome",runner.outcome_cell(renamed),1))
            assert runner.task_numerics(dict(cell=renamed))==c["numerics"]
        for numeric_seed in (410081,410089,410099):
            same=[c for c in selected if c["numerics"]["seed"]==numeric_seed]
            baseline=next(c["numerics"] for c in same if c["budget"]=="baseline")
            for c in same:
                assert sum(c["numerics"][k]!=baseline[k] for k in baseline)==(0 if c["budget"]=="baseline" else 1)


def test_numerical_profile_exports_requested_budgets(tmp_path):
    cell=next(c for c in runner.cells("numerical") if c["budget"]=="gram8192")
    text=runner.make_do(dict(cell=cell,c0=3.,replications=[1]),tmp_path,tmp_path/"data.csv",tmp_path/"rows.csv")
    assert "probes(200)" in text and "inferencegramprobes(8192)" in text
    assert "inferencesimulations(1000)" in text and "inferenceseed(410081)" in text


def test_bad_numerical_count_rejected(design):
    rows,task,manifest=fixture_rows(design)
    task["cell"]["route"]="rust"
    rows[0]["gram_probes"]=1024
    with pytest.raises(ValueError,match="numerical budget"): runner.validate_rows(rows,task,manifest)


def test_runner_centers_outcome_once_and_preserves_error_probes(design,tmp_path):
    _,task,_=fixture_rows(design)
    text=runner.make_do(task,tmp_path,tmp_path/"data.csv",tmp_path/"rows.csv")
    assert "centering(`mode')" in text
    assert "inferencesimulations(1000)" in text
    assert "asdouble" in text
    assert "local n=e(N_stored)" in text
    assert "e(native_error_phase)" in text
    assert f"y1-{design.c0:.17g}" in text
    assert "POOLED_COMPONENT_ASSESSMENT_STATA_PASS" in text
    assert "error 459" in runner.make_do(task,tmp_path,tmp_path/"data.csv",tmp_path/"rows.csv",fail=True)


def test_postprocess_failure_leaves_failed_receipt(design,tmp_path,monkeypatch):
    _,task,manifest=fixture_rows(design)
    task["id"]="fixture_task"
    manifest["tasks"]=[task]
    monkeypatch.setattr(runner,"load_manifest",lambda root:manifest)
    monkeypatch.setattr(runner,"build_design",lambda *args:design)
    monkeypatch.setattr(runner.subprocess,"run",lambda *args,**kwargs:SimpleNamespace(returncode=0,stdout="POOLED_COMPONENT_ASSESSMENT_STATA_PASS"))
    executable=tmp_path/"fake-stata";executable.write_text("test executable identity")
    with pytest.raises(FileNotFoundError): runner.run_task(tmp_path,task["id"],executable)
    receipt=json.loads((tmp_path/"tasks/fixture_task/receipt.json").read_text())
    assert receipt["status"]=="FAILED"
    assert "FileNotFoundError" in receipt["failure_reason"]
