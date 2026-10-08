"""Validate every attempted case and compare sampling/sketch errors to truth."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

os.environ.setdefault("MPLCONFIGDIR", "/tmp/fevc-centering-hetero-matplotlib")
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

from exact_oracle import TARGETS, oracle
from run import digest, keys, validate, validate_fixture


def collect(root, profile):
    manifest = json.loads((root/"manifest.json").read_text())
    source_failures=[]
    for relative,expected in manifest["files"].items():
        path=root/relative
        actual=digest(path) if path.is_file() else None
        if actual != expected:
            source_failures.append(dict(category="input",path=relative,expected_sha256=expected,actual_sha256=actual))
    if source_failures:
        audit_path=root/"analysis"/profile/"validation.json"
        audit_path.parent.mkdir(parents=True,exist_ok=True)
        audit_path.write_text(json.dumps(dict(status="FAIL",failures=source_failures),indent=2)+"\n")
        raise ValueError("frozen input integrity failure: inspect validation.json")
    units = manifest["profiles"][profile]
    cases = {c["case"]:c for c in manifest["cases"]}
    seen, frames, receipts, fixtures = set(), [], [], {}
    for td in sorted((root/"output"/profile).glob("*/task-*")):
        receipt = json.loads((td/"receipt.json").read_text())
        if receipt["manifest_sha256"] != digest(root/"manifest.json"):
            raise ValueError("manifest binding mismatch")
        if not receipt["stata_success"] or receipt["process_rc"]:
            raise ValueError(f"failed application {td}")
        d = pd.read_csv(td/"results.csv", na_values=["."])
        unique = {(r.case, r.phase, int(r.dataset), int(r.rep)) for r in d.itertuples()}
        taskunits = [u for u in units if (u["case"],u["phase"],u["dataset"],u["rep"]) in unique]
        case = td.parent.name
        if (receipt["expected_units"] != taskunits or receipt["expected_rows"] != len(keys(taskunits))
                or receipt["profile"] != profile or receipt["case"] != case
                or receipt["output_path"] != str(td.relative_to(root))
                or receipt["case_parameters"] != cases[case]):
            raise ValueError("task receipt does not match manifest inventory and case")
        validation = validate(td/"results.csv", taskunits)
        if digest(td/"results.csv") != receipt["results_sha256"]:
            raise ValueError("result hash mismatch")
        newkeys = keys(taskunits)
        if seen & newkeys:
            raise ValueError("duplicate task units")
        seen |= newkeys
        if set(d.case) != {case}:
            raise ValueError("case does not match task directory")
        if validate_fixture(td,receipt["case_parameters"]) != receipt["fixture"]:
            raise ValueError("fixture receipt no longer matches the generated files")
        fixture = pd.read_csv(td/"fixture.csv",na_values=["."])
        if case in fixtures:
            pd.testing.assert_frame_equal(fixtures[case][1], fixture, check_exact=False, rtol=0, atol=1e-12)
        else:
            fixtures[case] = (td/"fixture.csv", fixture)
        generated = pd.read_csv(td/"fixture.generated.csv",na_values=["."])
        if abs(generated.sigma2_true.dropna().mean()-2.25) > 1e-10:
            raise ValueError("generated employed conditional variance is not normalized")
        frames.append(d)
        receipts.append(dict(case=case, task=td.name, receipt_status=receipt["status"], **validation))
    if seen != keys(units):
        raise ValueError("incomplete task inventory")
    d = pd.concat(frames, ignore_index=True)
    audit = dict(rows=len(d),tasks=receipts,
        failures=[dict(case=r["case"],task=r["task"],**failure)
                  for r in receipts for failure in r["failures"]],
        withheld=[dict(case=r["case"],task=r["task"],**withheld)
                  for r in receipts for withheld in r["withheld"]])
    audit_path=root/"analysis"/profile/"validation.json"
    audit_path.parent.mkdir(parents=True,exist_ok=True)
    audit_path.write_text(json.dumps(audit,indent=2)+"\n")
    if audit["failures"] or any(r["status"] != "PASS" or r["receipt_status"] != "PASS" for r in receipts):
        raise ValueError("scientific/application failure: every validated failure is recorded in validation.json")
    stable = ["workerid","time","firmid","alpha_true","psi_true","signal"]
    reference = next(iter(fixtures.values()))[1][stable]
    for _, f in fixtures.values():
        pd.testing.assert_frame_equal(reference,f[stable], check_exact=False,rtol=0,atol=1e-12)
    oracles = {}
    for case,(path,f) in fixtures.items():
        reps = len([u for u in units if u["case"] == case and u["phase"] == "sampling"])
        point_paths=list((root/"output"/profile/case).glob("task-*/oracle-y.csv"))
        if len(point_paths) != 1:
            raise ValueError("expected one fixture-aligned point-parity outcome per case")
        outcome=pd.read_csv(point_paths[0])
        pd.testing.assert_frame_equal(f[["workerid","time"]],outcome[["workerid","time"]])
        result = oracle(path, outcome[["y"]].to_numpy(), sampling_reps=reps)
        if not result["diagnostics"]["positive_true_covariance"]:
            raise ValueError("fixture has not realized positive covariance")
        for row in result["moments"]:
            if row["mode"] in ("none","corrected") and abs(row["bias"]) > 1e-9:
                raise ValueError("independent exact oracle violates unbiasedness")
        errors=[]
        fitted=d[(d.case==case)&(d.phase=="sampling")&(d.rep==1)&(d.algorithm=="exact")]
        for row in result["moments"]:
            mode="none" if row["mode"]=="plugin" else row["mode"]
            col=f"{'plugin' if row['mode']=='plugin' else 'point'}{row['target']}"
            got=fitted[fitted.centering==mode][col]
            if len(got) != 1 or not np.isfinite(got.iloc[0]):
                raise ValueError("missing exact point for oracle parity")
            errors.append(abs(got.iloc[0]-row["points"][0]))
        result["diagnostics"]["maximum_exact_point_parity_error"]=max(errors)
        if max(errors)>1e-9:
            raise ValueError("exact point estimates disagree with independent dense oracle")
        oracles[case] = result
    successful = d[d.rc==0]
    for _, group in successful.groupby(["case","phase","dataset","outcome_seed"]):
        for j in range(1,5):
            if np.ptp(group[f"plugin{j}"].to_numpy()) > 1e-9:
                raise ValueError("plugin changed across paired requests")
    return d, oracles, receipts, manifest


def summaries(d, oracles):
    if (d.rc != 0).any():
        raise ValueError("cannot summarize failed fits as an accepted experiment; inspect validation.json")
    sampling, paired, center, calibration, increment = [], [], [], [], []
    for case, data in d.groupby("case"):
        truth = np.array(oracles[case]["diagnostics"]["true_targets"])
        analytic = {(r["mode"],r["target"]):r for r in oracles[case]["moments"]}
        contrast = {r["target"]:r for r in oracles[case]["paired_centering"]}
        s = data[(data.phase=="sampling") & (data.rc==0)]
        for (alg,mode), group in s.groupby(["algorithm","centering"]):
            for j,name in enumerate(TARGETS,1):
                vals = group[f"point{j}"].to_numpy()
                error = vals-truth[j-1]
                attempted = len(data[(data.phase=="sampling") & (data.algorithm==alg) & (data.centering==mode)])
                a = analytic[mode,j]
                sampling.append(dict(case=case,algorithm=alg,centering=mode,target=name,n=len(vals),attempted=attempted,
                    truth=truth[j-1],mean=vals.mean(),bias=error.mean(),simulation_se=vals.std(ddof=1)/np.sqrt(len(vals)),
                    sd=vals.std(ddof=1),rmse=np.sqrt(np.mean(error**2)),
                    exact_analytic_mean=a["expectation"],exact_analytic_bias=a["bias"],exact_analytic_sd=a["sd"]))
        plugin = s[(s.algorithm=="exact") & (s.centering=="none")]
        for j,name in enumerate(TARGETS,1):
            vals = plugin[f"plugin{j}"].to_numpy(); a=analytic["plugin",j]
            sampling.append(dict(case=case,algorithm="plugin",centering="unchanged",target=name,n=len(vals),
                attempted=len(data[(data.phase=="sampling") & (data.algorithm=="exact") & (data.centering=="none")]),
                truth=truth[j-1],mean=vals.mean(),bias=vals.mean()-truth[j-1],
                simulation_se=vals.std(ddof=1)/np.sqrt(len(vals)),sd=vals.std(ddof=1),
                rmse=np.sqrt(np.mean((vals-truth[j-1])**2)),exact_analytic_mean=a["expectation"],
                exact_analytic_bias=a["bias"],exact_analytic_sd=a["sd"]))
        for mode in ("none","mean","corrected"):
            pivot=s[s.centering==mode].pivot(index="rep",columns="algorithm",values=[f"point{i}" for i in range(1,5)])
            for j,name in enumerate(TARGETS,1):
                v=(pivot[f"point{j}","jla"]-pivot[f"point{j}","exact"]).to_numpy()
                paired.append(dict(case=case,centering=mode,target=name,n=len(v),
                    mean_jla_minus_exact=v.mean(),simulation_se=v.std(ddof=1)/np.sqrt(len(v)),
                    rmse_jla_minus_exact=np.sqrt(np.mean(v**2))))
        for alg,group in s.groupby("algorithm"):
            for j,name in enumerate(TARGETS,1):
                p=group.pivot(index="rep",columns="centering",values=f"point{j}")
                delta=(p["mean"]-p.corrected).to_numpy(); a=contrast[j]
                se=delta.std(ddof=1)/np.sqrt(len(delta))
                center.append(dict(case=case,algorithm=alg,target=name,n=len(delta),
                    mean_minus_corrected=delta.mean(),sd=delta.std(ddof=1),simulation_se=se,
                    exact_analytic_contrast=a["expected_mean_minus_corrected"],
                    exact_analytic_sd=a["sd_mean_minus_corrected"],
                    z_vs_exact_analytic=(delta.mean()-a["expected_mean_minus_corrected"])/se if se else np.nan))
        cal=data[(data.phase=="calibration") & (data.rc==0)]
        for (dataset,probes,mode),group in cal[cal.algorithm=="jla"].groupby(["dataset","probes","centering"]):
            exact=cal[(cal.dataset==dataset) & (cal.algorithm=="exact") & (cal.centering==mode)].iloc[0]
            for j,name in enumerate(TARGETS,1):
                v=group[f"point{j}"].to_numpy(); se=group[f"se{j}"].to_numpy()
                valid=np.isfinite(se); error=v-float(exact[f"point{j}"])
                rms=np.sqrt(np.mean(se[valid]**2)) if valid.any() else np.nan
                sd=v.std(ddof=1)
                attempted=len(data[(data.phase=="calibration") & (data.dataset==dataset) &
                    (data.probes==probes) & (data.centering==mode)])
                calibration.append(dict(case=case,dataset=int(dataset),probes=int(probes),centering=mode,target=name,
                    n=len(v),attempted=attempted,available=int(valid.sum()),exact=float(exact[f"point{j}"]),
                    numerical_bias=error.mean(),bias_simulation_se=sd/np.sqrt(len(v)),empirical_sd=sd,rms_mcse=rms,
                    mcse_sd_ratio=rms/sd,coverage_exact=float(np.mean(np.abs(error[valid])<=1.96*se[valid])),
                    coverage_centered=float(np.mean(np.abs(v[valid]-v.mean())<=1.96*se[valid])),
                    coverage_all_attempts=float(np.sum(np.abs(error[valid])<=1.96*se[valid])/attempted)))
        for (dataset,probes),group in cal[cal.algorithm=="jla"].groupby(["dataset","probes"]):
            for j,name in enumerate(TARGETS,1):
                p=group.pivot(index="rep",columns="centering",values=f"point{j}")
                delta=p.corrected-p["mean"]
                increment.append(dict(case=case,dataset=int(dataset),probes=int(probes),target=name,
                    mean_increment=delta.mean(),sd_increment=delta.std(ddof=1),sd_mean=p["mean"].std(ddof=1),
                    sd_corrected=p.corrected.std(ddof=1),increment_mean_cov=np.cov(delta,p["mean"],ddof=1)[0,1]))
    return {"sampling":pd.DataFrame(sampling),"paired":pd.DataFrame(paired),
            "centering-contrast":pd.DataFrame(center),"calibration":pd.DataFrame(calibration),
            "increment":pd.DataFrame(increment)}


def pooled_calibration(calibration):
    pooled=[]
    for (case,probes,mode,target),g in calibration.groupby(["case","probes","centering","target"]):
        numerical_sd=np.sqrt(np.average(g.empirical_sd**2,weights=g.n-1))
        rms_mcse=np.sqrt(np.average(g.rms_mcse**2,weights=g.available))
        pooled.append(dict(case=case,probes=probes,centering=mode,target=target,
            numerical_sd=numerical_sd,rms_mcse=rms_mcse,ratio=rms_mcse/numerical_sd,
            coverage_exact=np.average(g.coverage_exact,weights=g.available),
            coverage_all_attempts=np.average(g.coverage_all_attempts,weights=g.attempted)))
    return pd.DataFrame(pooled)


def verify_baseline(current_fixture, baseline_fixture):
    current=pd.read_csv(current_fixture); previous=pd.read_csv(baseline_fixture)
    columns=["workerid","time","firmid","lnwage","alpha_true","psi_true","lnwage_true","epsilon_true"]
    pd.testing.assert_frame_equal(current[columns],previous[columns],check_exact=True)
    signal_difference=float(np.max(np.abs(current.signal-previous.signal)))
    if signal_difference>1e-12:
        raise ValueError("homoskedastic conditional mean differs from original campaign")
    return dict(current_fixture_sha256=digest(current_fixture),baseline_fixture_sha256=digest(baseline_fixture),
        unchanged_columns=columns,exact_equality=True,maximum_signal_difference=signal_difference)


def figures(out,frames,cases):
    plt.rcParams.update({"font.size":10,"axes.spines.top":False,"axes.spines.right":False})
    colors={"none":"#a14a3b","mean":"#176b91","corrected":"#35845b"}
    for case in cases:
        fig,axes=plt.subplots(2,4,figsize=(15,8))
        fig.subplots_adjust(left=.07,right=.985,bottom=.085,top=.86,wspace=.35,hspace=.43)
        for j,target in enumerate(TARGETS):
            ax=axes[0,j]; g=frames["sampling"]
            g=g[(g.case==case)&(g.target==target)]
            for x,(mode,color) in enumerate(colors.items()):
                row=g[(g.algorithm=="exact")&(g.centering==mode)].iloc[0]
                ax.errorbar(x,row["mean"],yerr=1.96*row.simulation_se,fmt="o",color=color,capsize=4)
                jr=g[(g.algorithm=="jla")&(g.centering==mode)].iloc[0]
                ax.plot(x+.12,jr["mean"],"x",color=color)
            ax.axhline(g.truth.iloc[0],color="#252525",linestyle="--",label="Truth")
            ax.axhline(g[g.algorithm=="plugin"]["mean"].iloc[0],color="#777777",linestyle=":",label="Plug-in mean")
            ax.set_xticks(range(3),["None","Mean","Corrected"],rotation=20); ax.set_title(target)
            if j==0: ax.set_ylabel("Mean ± 1.96 simulation SE"); ax.legend(fontsize=8)
            ax=axes[1,j]
            for mode,color in colors.items():
                g=frames["calibration-pooled"]
                g=g[(g.case==case)&(g.target==target)&(g.centering==mode)].sort_values("probes")
                ax.plot(np.log2(g.probes)+(list(colors).index(mode)-1)*.045,g.ratio,"o-",color=color,label=mode.title())
            ax.axhline(1,color="#252525",linestyle="--"); ax.set_xticks([6,8],["64","256"]); ax.set_xlabel("JLA probes")
            if j==0: ax.set_ylabel("RMS MCSE / pooled sketch SD"); ax.legend(fontsize=8)
        fig.suptitle(f"{case.title()} variance profile: sampling and numerical calibration\nDots: exact; crosses: JLA(256). MCSE pooled within fixed outcomes.",y=.975,fontsize=14)
        fig.savefig(out/f"centering-{case}.png",dpi=180); plt.close(fig)
    fig,axes=plt.subplots(1,4,figsize=(15,4.8))
    fig.subplots_adjust(left=.075,right=.985,bottom=.16,top=.80,wspace=.4)
    for j,target in enumerate(TARGETS):
        ax=axes[j]
        for x,case in enumerate(cases):
            g=frames["centering-contrast"]
            g=g[(g.case==case)&(g.target==target)]
            for alg,offset,color,marker in [("exact",-.08,"#176b91","o"),("jla",.08,"#a14a3b","x")]:
                r=g[g.algorithm==alg].iloc[0]
                ax.errorbar(x+offset,r.mean_minus_corrected,yerr=1.96*r.simulation_se,fmt=marker,color=color,capsize=4,label=alg if x==0 else None)
            ax.plot([x-.23,x+.23],[g.exact_analytic_contrast.iloc[0]]*2,"k-",label="Exact expectation" if x==0 else None)
        ax.axhline(0,color="#aaa",linestyle="--"); ax.set_xticks(range(len(cases)),[c.title() for c in cases]); ax.set_title(target)
        ax.ticklabel_format(axis="y",style="sci",scilimits=(0,0))
        if j==0: ax.set_ylabel("Paired Mean − Corrected\n± 1.96 simulation SE"); ax.legend(fontsize=8)
    fig.suptitle("The estimated-mean correction: paired outcome contrasts",y=.97,fontsize=14)
    fig.savefig(out/"centering-contrast.png",dpi=180); plt.close(fig)


def main():
    p=argparse.ArgumentParser(); p.add_argument("root",type=Path)
    p.add_argument("--profile",default="main",choices=["main","smoke"])
    p.add_argument("--baseline-fixture",type=Path,help="Original campaign's retained homoskedastic fixture")
    a=p.parse_args()
    d,oracles,receipts,manifest=collect(a.root,a.profile)
    out=a.root/"analysis"/a.profile; out.mkdir(parents=True,exist_ok=True)
    baseline_note=""
    if a.baseline_fixture:
        current=next((a.root/"output"/a.profile/"homo").glob("task-*/fixture.csv"))
        preservation=verify_baseline(current,a.baseline_fixture)
        (out/"baseline-preservation.json").write_text(json.dumps(preservation,indent=2)+"\n")
        baseline_note=("<p><b>Preserved baseline.</b> The homoskedastic fixture exactly reproduces the prior campaign's worker, year, firm, observed wages, latent effects and realized shocks. "
            f"The direct conditional mean differs from the prior subtraction-based signal by at most {preservation['maximum_signal_difference']:.2g}, consistent with floating-point rounding.</p>")
    frames=summaries(d,oracles); frames["calibration-pooled"]=pooled_calibration(frames["calibration"])
    for name,frame in frames.items(): frame.to_csv(out/f"{name}.csv",index=False)
    (out/"design-oracle.json").write_text(json.dumps(oracles,indent=2)+"\n")
    cases=[c["case"] for c in manifest["cases"]]; figures(out,frames,cases)
    diagnostics=pd.DataFrame([dict(case=c,**o["diagnostics"]) for c,o in oracles.items()]).drop(columns="true_targets")
    analytic=pd.DataFrame([dict(case=c,**r) for c,o in oracles.items() for r in o["moments"]])
    analytic.to_csv(out/"analytic-moments.csv",index=False)
    tables=[]
    for title,frame in [("Realized design and conditional variances",diagnostics),("Exact Gaussian moments",analytic),
        ("Paired Mean minus Corrected",frames["centering-contrast"]),("Sampling estimates",frames["sampling"]),
        ("Paired JLA(256) minus exact",frames["paired"]),("Pooled within-outcome MCSE calibration",frames["calibration-pooled"]),
        ("Individual fixed-outcome MCSE calibration",frames["calibration"]),("Corrected JLA increment",frames["increment"])]:
        tables.append(f"<h2>{title}</h2>"+frame.to_html(index=False,float_format=lambda x:f"{x:.7g}"))
    pooled=frames["calibration-pooled"]; increment=frames["increment"]
    correction_effect=float(np.max(np.abs(increment.sd_corrected/increment.sd_mean-1)))
    page=f'''<!doctype html><meta charset="utf-8"><title>Heteroskedastic centering Monte Carlo</title>
<style>body{{font:16px system-ui;color:#20313c;margin:40px;max-width:1550px}}h1,h2{{color:#163b50}}table{{border-collapse:collapse;font-size:12px;margin-bottom:30px}}td,th{{padding:7px;border-bottom:1px solid #ddd;text-align:right}}th{{background:#edf3f6}}img{{width:100%}}code{{overflow-wrap:anywhere}}.note{{background:#edf3f6;padding:18px;line-height:1.5}}</style>
<h1>Heteroskedastic centering Monte Carlo · October 7, 2026</h1>
{'<p><b>DEVELOPMENT SMOKE: two outcome draws and two sketches per calibration cell. These values validate the pipeline and are not Monte Carlo findings.</b></p>' if a.profile=='smoke' else ''}
<p>Same small fesim network and positive worker–firm covariance across three conditional-variance profiles: homoskedastic, firm-only, and worker–firm interaction. Mean generated employed variance is 2.25 in each profile; retained-sample means may differ after pruning. Gaussian shocks are conditionally independent. Effects, mobility, and underlying standardized shocks are paired across profiles. Exact and JLA use None, Mean and Corrected, match deletion with eligible stayers, Rust and tolerance 1e-10.</p>
<div class="note"><b>Interpretation.</b> Sampling simulation SE describes an average over independent outcomes. JLA MCSE describes numerical randomness for a fixed outcome and is not an econometric standard error. Corrected reports exactly Mean's MCSE, excluding randomness in its additional increment. Numerical coverage is measured around each mode's exact estimate, separately from sampling bias.</div>
<p><b>Why the profiles differ.</b> Firm-only conditional variance lies in the worker–firm design span, so residualized variance is zero and Mean remains unbiased. The interaction creates residualized variance and a small estimated-mean bias. The dense independent oracle computes Gaussian moments directly from each estimator's quadratic matrix and exported conditional variances; None and Corrected should have zero bias up to roundoff. Paired Mean-minus-Corrected outcome contrasts reveal the small adjustment much more precisely than separately comparing either estimator's mean with truth.</p>
<p>All {len(d):,} attempted fits enter the inventory audit. Failures and withheld diagnostics are listed in validation.json. Profiles, seeds, repetition counts and source inputs were frozen before execution; this is exploratory benchmark evidence, not native qualification.</p>
{baseline_note}
<p>Across all cases, targets and probe budgets, pooled RMS MCSE / empirical sketch SD ranges from {pooled.ratio.min():.3f} to {pooled.ratio.max():.3f}, with numerical 95% coverage from {pooled.coverage_exact.min():.1%} to {pooled.coverage_exact.max():.1%}. Corrected's increment changes empirical sketch SD by up to {correction_effect:.2%}; its reported MCSE omits that increment's uncertainty.</p>
<img src="centering-contrast.png" alt="Paired centering contrasts against independent analytic expectations">
{''.join(f'<h2>{c.title()} profile</h2><img src="centering-{c}.png" alt="{c} sampling and MCSE calibration">' for c in cases)}
{''.join(tables)}
<h2>Reproducibility</h2><p>fevc: <code>{manifest['fevc_commit']}</code><br>fesim base: <code>{manifest['fesim_commit']}</code><br>Manifest SHA-256: <code>{digest(a.root/'manifest.json')}</code></p>
<p>The manifest binds the immutable fesim working-tree source bundle. Raw task results, fixtures, receipts and logs are preserved. CSV tables accompany this report. The prior homoskedastic campaign remains unchanged.</p>'''
    (out/"report.html").write_text(page)
    print(frames["centering-contrast"].to_string(index=False))
    print(out/"report.html")


if __name__ == "__main__": main()
