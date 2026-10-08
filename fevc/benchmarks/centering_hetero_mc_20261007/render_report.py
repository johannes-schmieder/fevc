#!/usr/bin/env python3
"""Academic presentation of immutable centering MC evidence; no estimator reruns."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import sys

# Optional pure-Python PDF dependencies supplied by the desktop runtime.
if os.environ.get("FEVC_REPORT_PYTHONPATH"):
    sys.path.append(os.environ["FEVC_REPORT_PYTHONPATH"])
os.environ.setdefault("MPLCONFIGDIR", "/tmp/fevc-centering-report-matplotlib")

import pandas as pd
import matplotlib
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_JUSTIFY, TA_LEFT
from reportlab.lib.styles import ParagraphStyle
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (
    Image, PageBreak, Paragraph, SimpleDocTemplate, Spacer, Table, TableStyle,
)

from report_figures import make_figures

CASES = ["homo", "firm", "interaction"]
CASE_NAMES = {
    "homo": "Homoskedastic noise", "firm": "Firm heteroskedasticity",
    "interaction": "Worker-firm interaction heteroskedasticity",
}
TARGETS = ["Worker variance", "Firm variance", "Worker–firm covariance", "Variance of sum"]
SHORT = ["Worker variance", "Firm variance", "Covariance", "Variance of sum"]
MODES = ["none", "mean", "corrected"]
WIDTH = 520


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def verify_evidence(root):
    accepted = json.loads((root / "main.accepted.json").read_text())
    assert accepted["status"] == "PASS" and accepted["rows"] == 52227
    assert accepted["tasks"] == 48 and accepted["failures"] == accepted["withheld"] == 0
    assert digest(root / "manifest.json") == accepted["manifest_sha256"]
    assert digest(root / accepted["artifact_hash_inventory"]) == accepted["artifact_hash_inventory_sha256"]
    manifest = json.loads((root / "manifest.json").read_text())
    inventory = json.loads((root / accepted["artifact_hash_inventory"]).read_text())
    for rel, expected in {**manifest["files"], **inventory["files"]}.items():
        assert digest(root / rel) == expected, rel
    return accepted


def build(root, output):
    accepted = verify_evidence(root)
    font_dir = Path(matplotlib.get_data_path()) / "fonts/ttf"
    for name, filename in [("Times-Roman", "STIXGeneral.ttf"),
                           ("Times-Bold", "STIXGeneralBol.ttf"),
                           ("Times-Italic", "STIXGeneralItalic.ttf"),
                           ("Times-BoldItalic", "STIXGeneralBolIta.ttf")]:
        pdfmetrics.registerFont(TTFont(name, str(font_dir / filename)))
    pdfmetrics.registerFontFamily("Times-Roman", normal="Times-Roman", bold="Times-Bold",
                                  italic="Times-Italic", boldItalic="Times-BoldItalic")
    source = root / "analysis/main"
    data = {name: pd.read_csv(source / f"{name}.csv") for name in
            ["sampling", "analytic-moments", "centering-contrast", "paired", "calibration-pooled", "increment"]}
    oracle = json.loads((source / "design-oracle.json").read_text())
    assets = output.parent / (output.stem + "_assets")
    assets.mkdir(parents=True, exist_ok=True)
    figures = make_figures(root, assets)
    story = []
    styles = {
        "body": ParagraphStyle("body", fontName="Times-Roman", fontSize=11,
                               leading=14, alignment=TA_JUSTIFY, spaceAfter=9),
        "small": ParagraphStyle("small", fontName="Times-Roman", fontSize=9.4,
                                leading=11.6, spaceAfter=7),
        "caption": ParagraphStyle("caption", fontName="Times-Roman", fontSize=10,
                                  leading=12, spaceAfter=7),
        "heading": ParagraphStyle("heading", fontName="Times-Bold", fontSize=14,
                                  leading=17, spaceBefore=5, spaceAfter=11),
        "subheading": ParagraphStyle("subheading", fontName="Times-Bold", fontSize=11.5,
                                     leading=14, spaceBefore=8, spaceAfter=7),
        "title": ParagraphStyle("title", fontName="Times-Roman", fontSize=18,
                                leading=22, alignment=TA_CENTER, spaceAfter=20),
        "subtitle": ParagraphStyle("subtitle", fontName="Times-Roman", fontSize=12,
                                   leading=15, alignment=TA_CENTER, spaceAfter=20),
        "table": ParagraphStyle("table", fontName="Times-Roman", fontSize=9,
                                leading=10.8, alignment=TA_LEFT),
        "head": ParagraphStyle("head", fontName="Times-Roman", fontSize=9,
                               leading=10.8, alignment=TA_CENTER),
    }

    def para(text, kind="body"):
        story.append(Paragraph(text, styles[kind]))

    def heading(text):
        para(text, "heading")

    def table(headers, rows, widths, breaks=(), fontsize=9, padding=3):
        rows = [[Paragraph(str(x), styles["head"]) for x in headers]] + rows
        t = Table(rows, colWidths=widths, repeatRows=1, hAlign="CENTER")
        commands = [
            ("FONTNAME", (0, 0), (-1, -1), "Times-Roman"),
            ("FONTSIZE", (0, 1), (-1, -1), fontsize),
            ("LEADING", (0, 1), (-1, -1), fontsize + 2),
            ("TOPPADDING", (0, 0), (-1, -1), padding),
            ("BOTTOMPADDING", (0, 0), (-1, -1), padding),
            ("LEFTPADDING", (0, 0), (-1, -1), 4),
            ("RIGHTPADDING", (0, 0), (-1, -1), 4),
            ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
            ("ALIGN", (2, 1), (-1, -1), "RIGHT"),
            ("LINEABOVE", (0, 0), (-1, 0), .6, colors.black),
            ("LINEBELOW", (0, 0), (-1, 0), .4, colors.black),
            ("LINEBELOW", (0, -1), (-1, -1), .6, colors.black),
        ]
        for row in breaks:
            commands.append(("LINEABOVE", (0, row), (-1, row), .25, colors.HexColor("#aaaaaa")))
            commands.append(("TOPPADDING", (0, row), (-1, row), padding + 3))
        t.setStyle(TableStyle(commands))
        story.append(t)
        story.append(Spacer(1, 9))

    def pic(key, height):
        story.append(Image(str(figures[key]), width=WIDTH, height=height, kind="proportional"))

    def page():
        story.append(PageBreak())

    def select(df, **kwargs):
        for k, v in kwargs.items():
            df = df[df[k] == v]
        assert len(df) == 1, kwargs
        return df.iloc[0]

    analytic = data["analytic-moments"]
    practical = []
    for name in TARGETS:
        mean = select(analytic, case="interaction", target_name=name, mode="mean")
        corr = select(analytic, case="interaction", target_name=name, mode="corrected")
        rmse_mean = math.hypot(mean["sd"], mean["bias"])
        rmse_corr = math.hypot(corr["sd"], corr["bias"])
        practical.append({"target": name, "mean_bias": mean["bias"], "mean_sd": mean["sd"],
                          "bias_percent_of_sd": 100 * abs(mean["bias"]) / mean["sd"],
                          "corrected_rmse_percent_change": 100 * (rmse_corr / rmse_mean - 1)})

    para("Centering in fixed-effect variance estimation", "title")
    para("Exact and JLA Monte Carlo evidence under heteroskedasticity", "subtitle")
    para("This study compares plug-in estimates with fevc using None, Mean, and Corrected centering. "
         "It holds one small worker-firm network fixed and varies independent wage shocks across three "
         "conditional-variance profiles. The main result is a large precision gain from centering, "
         "with an additional bias correction that is very small in this design.")
    heading("1  Why Mean and Corrected look alike")
    para("Their near-overlap is a substantive result. In the interaction profile, Mean's analytic "
         "worker-variance bias is 0.000151, compared with a sampling standard deviation of 0.121882. "
         "The bias is only 0.124% of that dispersion. Plotting the two marginal distributions on "
         "their natural scale should therefore produce almost identical curves.")
    para("Table 1: Size of the correction in the interaction profile. Exact Gaussian-oracle quantities.", "caption")
    table(["Component", "Mean bias<br/>(x 10<super>-6</super>)", "Mean SD", "|Bias| / SD<br/>(%)", "Corrected RMSE<br/>change (%)"],
          [[SHORT[i], f"{r['mean_bias'] * 1e6:.3f}", f"{r['mean_sd']:.6f}",
            f"{r['bias_percent_of_sd']:.3f}", f"+{r['corrected_rmse_percent_change']:.3f}"]
           for i, r in enumerate(practical)], [128, 82, 82, 105, 123])
    para("Corrected removes the analytic bias but adds a tiny amount of variance. Its exact RMSE is "
         "slightly higher here, by 0.004-0.106%. Unbiasedness and lower mean squared error are distinct "
         "criteria; this experiment does not demonstrate a practically meaningful RMSE benefit from "
         "the additional correction.")
    para("1.1  What each comparison measures", "subheading")
    para("The plug-in estimate takes moments of fitted worker and firm effects. None applies the "
         "leave-out noise correction without outcome centering. Mean centers its outcome factor "
         "using the sample mean. Corrected adds the adjustment for estimating that mean. Exact and "
         "JLA target the corresponding same-mode quantities; JLA adds numerical randomization.")
    para("The paired difference, Mean minus Corrected on the same outcome and probes, removes most "
         "shared fluctuation. It can reveal a small systematic correction even when the marginal "
         "densities are indistinguishable. The next page displays that contrast directly.")

    page()
    heading("2  Paired Mean minus Corrected")
    para("Every point uses all 2,000 paired outcome draws. Intervals are Monte Carlo uncertainty "
         "about the average difference, not confidence intervals for an individual variance estimate. "
         "Axes are explicitly magnified to millionths; the visual separation should not be read as "
         "a large improvement in estimator accuracy.")
    pic("contrast", 366)
    para("Figure 1: Mean-minus-Corrected contrasts. Bars are the simulated mean plus or minus 1.96 "
         "paired simulation SEs. Diamonds mark the exact analytic expectation; the vertical line "
         "marks zero. JLA uses 256 probes. Profile comparisons share standardized outcome draws.", "caption")
    para("Table 2: Interaction-profile contrasts, all in units of 10<super>-6</super>. Parentheses give paired simulation SEs.", "caption")
    contrast_rows = []
    for i, name in enumerate(TARGETS):
        ex = select(data["centering-contrast"], case="interaction", algorithm="exact", target=name)
        jl = select(data["centering-contrast"], case="interaction", algorithm="jla", target=name)
        contrast_rows.append([SHORT[i], f"{ex.exact_analytic_contrast * 1e6:.3f}",
                              f"{ex.mean_minus_corrected * 1e6:.3f} ({ex.simulation_se * 1e6:.3f})",
                              f"{jl.mean_minus_corrected * 1e6:.3f} ({jl.simulation_se * 1e6:.3f})"])
    table(["Component", "Analytic", "Exact MC (SE)", "JLA MC (SE)"], contrast_rows, [140, 80, 150, 150])
    para("All 24 exact/JLA contrasts lie within 1.15 simulation SEs of the analytic expectation. "
         "The interaction creates a small nonzero expectation; the homoskedastic and firm-only "
         "profiles have zero expectation, even though their realized corrections need not be zero.", "small")

    page()
    heading("3  Data generating processes and simulation design")
    para("fesim's AKM stylized preset generates 100 workers and 15 firms over six annual periods after a five-year burn-in. "
         "The seed is 7102026. Structural wage parameters are mu = 3, worker-effect SD = 0.45, "
         "firm-effect SD = 0.30, and noise scale = 1.50. Positive sorting produces a retained true "
         "worker-firm covariance of 0.0202503. The generated panel has 600 rows; 574 are employed, "
         "and estimator pruning retains 573 rows, 99 workers, and 15 firms. Additional parameters "
         "are theta_sort = 1.5, firm_size_sd = 0.25, kappa_ee = -0.69314718056, and kappa_eu = -4.")
    para("For generated population midrank scores a and b, each strictly between -1 and 1, the "
         "log-variance index is eta = h_worker a + h_firm b + h_interaction ab. Conditional variance "
         "is 2.25 exp(eta) divided by the mean of exp(eta) over generated employed output rows. "
         "Thus each profile redistributes the same total conditional noise budget. No renormalization "
         "occurs after connectivity filtering or estimator pruning.")
    para("Table 3: Variance profiles on the common retained design.", "caption")
    table(["Profile", "Nonzero coefficient", "Mean variance", "Max/min", "Residualized<br/>variance RMS"],
          [["Homoskedastic", "None", "2.2500", "1.00", "0 (roundoff)"],
           ["Firm", "h_firm = 2", "2.2478", "41.82", "0 (roundoff)"],
           ["Interaction", "h_interaction = 2", "2.2510", "40.29", "1.0175"]],
          [103, 127, 95, 75, 120])
    para("Firm-only variance lies in the fixed-effect design span. Its residual projection is zero, "
         "so Mean remains unbiased despite substantial heteroskedasticity. The interaction profile "
         "has a nonzero residual projection and a nonzero Mean bias. Variance dispersion alone is "
         "therefore not a measure of the strength of the centering-bias experiment.")
    para("Table 4: Fixed truth on the retained sample. Targets use the registered N = 573 denominator.", "caption")
    truths = oracle["homo"]["diagnostics"]["true_targets"]
    table(["Component", "Truth"], [[SHORT[i], f"{v:.7f}"] for i, v in enumerate(truths)], [310, 150])
    para("3.1  Replications, pairing, and uncertainty", "subheading")
    para("For each profile, 2,000 independent Gaussian outcome vectors use its exported conditional "
         "mean and variance. All methods share the outcome draws, and profiles share underlying "
         "standard normal draws. Exact and JLA(256) are compared for all three centerings. Separately, "
         "three fixed outcomes per profile each receive 300 independent sketches at 64 and 256 "
         "probes. This numerical experiment holds the observed outcome fixed.")
    para("All 52,227 attempted fits succeeded, with no withheld MCSEs. Match deletion, eligible "
         "stayers, strict Rust routing, and tolerance 1e-10 are common throughout. The design has "
         "294 deletion units and 56 eligible-stayer rows. Bias SEs in performance tables equal "
         "sampling SD divided by sqrt(2,000). JLA MCSE instead measures numerical randomness "
         "conditional on an outcome. Neither is an econometric standard error for the target.")

    case_text = {
        "homo": ("This baseline reproduces the original fixture's existing columns exactly. "
                 "Exact None, Mean, and Corrected are all analytically unbiased. Mean lowers "
                 "worker-variance sampling SD by 39.5% and sum-variance SD by 43.8% relative to None. "
                 "Corrected changes those dispersions only slightly."),
        "firm": ("Conditional variances differ by a factor of 41.82 across retained rows. "
                 "Because this variance vector is in the firm-effect span, Mean's analytic bias "
                 "remains zero. Mean lowers worker-variance sampling SD by 37.1% and sum-variance "
                 "SD by 41.7%. Large heteroskedasticity does not by itself separate Mean and Corrected."),
        "interaction": ("Conditional variances differ by a factor of 40.29 and have a nonzero "
                        "residual projection. Corrected removes Mean's small analytic bias, shown "
                        "on pages 1-2. Mean lowers worker-variance sampling SD by 36.5% and "
                        "sum-variance SD by 35.1%; the additional correction has little practical "
                        "effect on precision in this design."),
    }
    for c, case in enumerate(CASES):
        page()
        heading(f"{c + 4}  {CASE_NAMES[case]}")
        para(case_text[case])
        para("Table %d: Estimator performance over all 2,000 outcomes. Parentheses give the "
             "simulation SE of bias. JLA uses 256 probes." % (c + 5), "caption")
        rows = []
        for i, target in enumerate(TARGETS):
            specs = [("plugin", "unchanged", "Plug-in")] + [
                (alg, mode, ("Exact " if alg == "exact" else "JLA ") + mode.title())
                for alg in ["exact", "jla"] for mode in MODES]
            for j, (alg, mode, label) in enumerate(specs):
                row = select(data["sampling"], case=case, target=target, algorithm=alg, centering=mode)
                rows.append([SHORT[i] if j == 0 else "", label, f"{row.truth:.5f}",
                             f"{row['mean']:.5f}", f"{row.bias:.5f} ({row.simulation_se:.5f})",
                             f"{row.sd:.5f}", f"{row.rmse:.5f}"])
        table(["Component", "Estimator", "Truth", "Mean", "Bias (simulation SE)", "SD", "RMSE"],
              rows, [100, 84, 48, 48, 132, 54, 54], breaks=[8, 15, 22], fontsize=8.5, padding=2.5)
        if case == "firm":
            para("The exact Corrected covariance mean, 0.023177, is 3.36 simulation SEs above "
                 "truth; its worker-variance mean is 2.34 SEs below truth. These finite-MC deviations "
                 "are retained. The independent analytic calculation verifies zero exact bias.", "small")
        else:
            para("Observed bias includes finite-MC fluctuation. Analytic expectations and paired "
                 "contrasts distinguish that fluctuation from centering bias. Negative estimates "
                 "are retained; no fits or components were selected on significance.", "small")
        page()
        heading(f"{c + 4}.1  {CASE_NAMES[case]}: distributions")
        pic(case, 580)
        para(f"Figure {c + 2}: All four components and all 2,000 outcomes. Upper panels show exact "
             "estimates; lower panels show JLA(256) estimates minus truth. Black vertical lines mark "
             "truth or zero. Colored dashed vertical lines mark estimator means or biases. A common "
             "Gaussian bandwidth is used within each panel, and every draw enters each density. "
             "Mean and Corrected overlap because their difference is small; Figure 1 isolates it.", "caption")

    page()
    heading("7  Calibration of JLA numerical MCSE")
    para("Each entry below pools numerical dispersion within the three fixed outcomes, using "
         "300 sketches per outcome. The ratio compares RMS reported MCSE with empirical sketch "
         "SD. Coverage is the share of estimate +/- 1.96 MCSE intervals containing the corresponding "
         "exact estimate. Ideal values are one and 95%, respectively. These are numerical intervals, "
         "not sampling confidence intervals.")
    para("Table 8: RMS MCSE / numerical SD and numerical 95% coverage (%). Each cell shows ratio / coverage.", "caption")
    cal_rows = []
    for case in CASES:
        for p in [64, 256]:
            for i, target in enumerate(TARGETS):
                values = []
                for mode in MODES:
                    r = select(data["calibration-pooled"], case=case, probes=p, target=target, centering=mode)
                    values.append(f"{r.ratio:.3f} / {100 * r.coverage_exact:.1f}")
                cal_rows.append([{"homo":"Homo", "firm":"Firm", "interaction":"Interaction"}[case]
                                 if p == 64 and i == 0 else "", str(p) if i == 0 else "",
                                 SHORT[i], *values])
    table(["Profile", "Probes", "Component", "None", "Mean", "Corrected"], cal_rows,
          [68, 45, 116, 97, 97, 97], breaks=[5, 9, 13, 17, 21], fontsize=9, padding=3)
    para("Pooled ratios span 0.951-1.066 and coverage spans 92.9-96.6%. The weakest coverage "
         "occurs for interaction-profile firm variance with 64 probes. Individual fixed-outcome "
         "cell ratios are wider, 0.912-1.152. Increasing probes from 64 to 256 approximately halves "
         "MCSE (ratios 1.990-2.033). The three fixed outcomes give a limited calibration assessment.")
    para("Corrected returns exactly Mean's reported MCSE and omits uncertainty in its additional "
         "increment. Empirical full-Corrected dispersion is used for the Corrected ratios above. "
         "That increment changes sketch SD by at most 0.645% here; this is not a general guarantee.")

    page()
    heading("7.1  Numerical precision and the exact comparison")
    pic("jla", 330)
    para("Figure 5: Interaction profile, 256 probes. RMS reported MCSE and empirical numerical SD "
         "pool within three fixed outcomes. Both start at zero on a common scale within each "
         "component. Mean and Corrected report the same MCSE; the empirical Corrected SD includes "
         "randomness in the added correction.", "caption")
    para("Table 9: RMSE of JLA(256) minus its same-mode exact estimate, across 2,000 outcomes.", "caption")
    pairs = []
    for case in CASES:
        for i, target in enumerate(TARGETS):
            pairs.append([{"homo":"Homo", "firm":"Firm", "interaction":"Interaction"}[case] if i == 0 else "",
                          SHORT[i], *[f"{select(data['paired'], case=case, target=target, centering=m).rmse_jla_minus_exact:.5f}"
                                      for m in MODES]])
    table(["Profile", "Component", "None", "Mean", "Corrected"], pairs,
          [75, 151, 98, 98, 98], breaks=[5, 9], fontsize=9, padding=2.5)
    para("The maximum absolute average JLA-minus-exact difference is 0.000345; all such means "
         "are within 1.12 paired simulation SEs of zero. Centering roughly halves worker-variance "
         "JLA approximation RMSE. This gain is much larger than the Mean-Corrected distinction.", "small")

    page()
    heading("8  Interpretation and limits")
    para("The experiment cleanly separates three questions. First, estimation noise creates "
         "substantial plug-in bias, including a negative estimated covariance when truth is positive. "
         "Second, centering substantially reduces sampling and JLA numerical dispersion. Third, "
         "the additional Corrected adjustment eliminates the small Mean bias in the interaction "
         "profile, while making almost no visible difference to marginal distributions.")
    para("The difference is statistically detectable through pairing, but the correction is "
         "too small to deliver a material RMSE gain in this network. Indeed, the exact oracle "
         "finds slightly higher Corrected RMSE for all four interaction targets. Presenting the "
         "paired contrast on its own scale makes the correction visible without claiming that "
         "it is practically large.")
    para("8.1  What a stronger stress test would require", "subheading")
    para("A new experiment should first screen networks and variance profiles using the exact "
         "oracle. The useful diagnostics are absolute Mean bias divided by sampling SD, the "
         "Mean-Corrected RMSE difference, and leave-out identification. A large maximum/minimum "
         "variance ratio is insufficient. The variance pattern must align with the correction's "
         "bias weights, which depend on network and deletion geometry. More replications make "
         "small differences easier to detect but cannot make them economically larger.")
    para("Any such design search would be exploratory. A selected stress design would require "
         "a separately frozen campaign and fresh outcome draws. This report reuses the completed "
         "campaign and does not alter its design, outcomes, acceptance rules, or results.")
    para("8.2  Scope and reproducibility", "subheading")
    para("Evidence is conditional on one small network with independent Gaussian errors and "
         "no controls or weights. It does not establish performance under correlated errors or "
         "across a population of networks. Exact Corrected covariance sample means are 2.20-3.36 simulation SEs "
         "above truth across profiles. These deviations are correlated because the profiles share "
         "standardized outcome draws. They remain reported without retuning.")
    para("The main SCC job used 16 slots and completed in 2,353 seconds with scheduler failure "
         "and exit status zero. All 48 task receipts, 52,227 fit keys, and 116 frozen source/input "
         "files passed validation. Independent exact point parity is within 1.42e-13. The original "
         "homoskedastic fixture's existing columns are bitwise unchanged. Accepted evidence and "
         "the original HTML report remain immutable; this PDF is a new presentation of those results.")
    para("Campaign: 20261007-centering-hetero-mc-v1; SCC job 7922488.<br/>"
         "Manifest SHA-256:<br/><font size='8'>" + accepted["manifest_sha256"] + "</font>", "small")
    para("The PDF builder records its input identities and derived display quantities in an "
         "adjacent presentation receipt. Tables use the complete accepted summary files; density "
         "panels use raw per-replication results. Every numerical scale change is labeled. "
         "This is exploratory estimator evidence, not release or platform qualification.", "small")

    def footer(canvas, doc):
        canvas.setFont("Times-Roman", 10)
        canvas.drawCentredString(306, 24, str(doc.page))

    output.parent.mkdir(parents=True, exist_ok=True)
    doc = SimpleDocTemplate(str(output), pagesize=(612, 792), rightMargin=46, leftMargin=46,
                            topMargin=42, bottomMargin=40, title="Centering in fixed-effect variance estimation",
                            author="fevc Monte Carlo analysis")
    doc.build(story, onFirstPage=footer, onLaterPages=footer)
    # Reading evidence again also certifies that presentation did not mutate it.
    verify_evidence(root)
    receipt = {
        "kind": "presentation overlay", "status": "BUILT; visual review recorded separately",
        "evidence_root": str(root.resolve()), "manifest_sha256": accepted["manifest_sha256"],
        "accepted_receipt_sha256": digest(root / "main.accepted.json"),
        "builder_sha256": digest(__file__),
        "figure_builder_sha256": digest(Path(__file__).with_name("report_figures.py")),
        "pdf_sha256": digest(output), "pdf": str(output.resolve()),
        "rows": 52227, "sampling_reps_per_profile": 2000,
        "derived_interaction_practical_size": practical,
        "frozen_evidence_verified_before_and_after": True,
        "new_estimator_runs": 0,
    }
    output.with_suffix(".receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"pdf": str(output), "bytes": output.stat().st_size,
                      "manifest_sha256": accepted["manifest_sha256"]}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--output", type=Path, default=Path("output/pdf/fevc_centering_mc.pdf"))
    args = parser.parse_args()
    build(args.root, args.output)
