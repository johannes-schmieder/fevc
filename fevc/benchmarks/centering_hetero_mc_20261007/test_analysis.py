import numpy as np
import pandas as pd
import pytest
import json

import analyze
from run import cells, digest, keys

from analyze import TARGETS, pooled_calibration, summaries


def sample_results():
    truth=np.array([.2,.1,.03,.36]); rows=[]
    for phase in ("sampling","calibration"):
        for rep,shock in enumerate([-.3,-.1,.1,.3],1):
            for alg in ("exact","jla"):
                if phase=="calibration" and alg=="exact" and rep>1:
                    continue
                for mode in ("none","mean","corrected"):
                    if phase=="calibration":
                        value=truth+(shock if alg=="jla" else 0)
                    else:
                        value=truth+shock+(0.01+shock*.02 if mode=="mean" else 0)
                        if alg=="jla": value=value+.001*rep
                    row=dict(case="interaction",phase=phase,rep=rep,dataset=int(phase=="calibration"),
                        algorithm=alg,centering=mode,rc=0,probes=256 if alg=="jla" else 0)
                    for j in range(1,5):
                        row[f"point{j}"]=value[j-1]
                        row[f"plugin{j}"]=truth[j-1]+.4+shock
                        row[f"se{j}"]=.25 if alg=="jla" else 0.
                    rows.append(row)
    moments=[dict(target=j,mode=mode,expectation=t,bias=0.,sd=.5)
             for j,t in enumerate(truth,1) for mode in ("plugin","none","mean","corrected")]
    paired=[dict(target=j,expected_mean_minus_corrected=.01,sd_mean_minus_corrected=.003)
            for j in range(1,5)]
    oracle={"interaction":dict(diagnostics=dict(true_targets=truth.tolist()),moments=moments,paired_centering=paired)}
    return pd.DataFrame(rows),oracle


def test_paired_contrasts_and_numerical_error_keep_outcomes_paired():
    data,oracle=sample_results(); tables=summaries(data,oracle)
    contrasts=tables["centering-contrast"]
    np.testing.assert_allclose(contrasts.mean_minus_corrected,.01,atol=1e-15)
    expected_se=np.std(np.array([-.3,-.1,.1,.3])*.02,ddof=1)/2
    np.testing.assert_allclose(contrasts.simulation_se,expected_se,atol=1e-15)
    assert set(contrasts.algorithm)=={"exact","jla"}
    np.testing.assert_allclose(tables["paired"].mean_jla_minus_exact,.0025,atol=1e-15)
    assert len(tables["sampling"])==28
    assert (tables["sampling"].n==tables["sampling"].attempted).all()


def test_failed_fits_cannot_be_silently_dropped():
    data,oracle=sample_results(); data.loc[0,"rc"]=498
    with pytest.raises(ValueError,match="failed fits"):
        summaries(data,oracle)


def test_mcse_pools_within_outcomes_and_counts_withheld_attempts():
    records=[]
    for ds,n,available,sd,rms,coverage in [(1,10,10,2.,2.2,.9),(2,20,15,3.,2.8,.8)]:
        records.append(dict(case="homo",probes=256,centering="none",target=TARGETS[0],dataset=ds,
            n=n,available=available,attempted=n,empirical_sd=sd,rms_mcse=rms,coverage_exact=coverage,
            coverage_all_attempts=coverage*available/n,exact=1000.*ds))
    p=pooled_calibration(pd.DataFrame(records)).iloc[0]
    assert p.numerical_sd==pytest.approx(np.sqrt((9*4+19*9)/28))
    assert p.rms_mcse==pytest.approx(np.sqrt((10*2.2**2+15*2.8**2)/25))
    assert p.coverage_exact==pytest.approx((9+12)/25)
    assert p.coverage_all_attempts==pytest.approx((9+12)/30)


def test_collector_audits_every_scientific_failure_before_summary(tmp_path, monkeypatch):
    case=dict(case="homo",het_worker=0,het_firm=0,het_interaction=0)
    units=cells("smoke",[case])[:1]
    manifest=dict(cases=[case],profiles=dict(smoke=units),files={})
    (tmp_path/"manifest.json").write_text(json.dumps(manifest))
    td=tmp_path/"output/smoke/homo/task-01"; td.mkdir(parents=True)
    rows=[dict(case=k[0],phase=k[1],dataset=k[2],rep=k[3],algorithm=k[4],centering=k[5],probes=k[6],rc=0)
          for k in keys(units)]
    pd.DataFrame(rows).to_csv(td/"results.csv",index=False)
    pd.DataFrame(dict(sigma2_true=[2.25])).to_csv(td/"fixture.generated.csv",index=False)
    pd.DataFrame(dict(workerid=[1],time=[2000],firmid=[1],alpha_true=[.1],psi_true=[.1],signal=[3.2])).to_csv(td/"fixture.csv",index=False)
    receipt=dict(manifest_sha256=digest(tmp_path/"manifest.json"),stata_success=True,process_rc=0,
        expected_units=units,expected_rows=len(keys(units)),profile="smoke",case="homo",
        output_path="output/smoke/homo/task-01",case_parameters=case,fixture={},status="FAIL",
        results_sha256=digest(td/"results.csv"))
    (td/"receipt.json").write_text(json.dumps(receipt))
    failures=[dict(key=list(k),category="scientific",error="bad target identity") for k in list(keys(units))[:2]]
    monkeypatch.setattr(analyze,"validate",lambda *_:dict(rows=6,failures=failures,withheld=[],status="FAIL"))
    monkeypatch.setattr(analyze,"validate_fixture",lambda *_:{})
    with pytest.raises(ValueError,match="scientific/application failure"):
        analyze.collect(tmp_path,"smoke")
    audit=json.loads((tmp_path/"analysis/smoke/validation.json").read_text())
    assert len(audit["failures"])==2
    assert all(f["category"]=="scientific" for f in audit["failures"])


def test_collector_enumerates_changed_and_missing_inputs(tmp_path):
    (tmp_path/"source.py").write_text("changed")
    (tmp_path/"manifest.json").write_text(json.dumps(dict(files={"source.py":"wronghash","missing.py":"alsoincorrect"})))
    with pytest.raises(ValueError,match="input integrity"):
        analyze.collect(tmp_path,"main")
    audit=json.loads((tmp_path/"analysis/main/validation.json").read_text())
    assert {row["path"] for row in audit["failures"]}=={"source.py","missing.py"}
    assert audit["status"]=="FAIL"
