#!/usr/bin/env python3
"""Create paper-style performance figures and a standalone Markdown report."""

from __future__ import annotations

import argparse, csv, json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

COLORS={"fevc":"#0072B2","matlab":"#D55E00","julia":"#009E73","r":"#CC79A7","pytwoway":"#E69F00"}
LABELS={"fevc":"FEVC (Rust)","matlab":"KSS MATLAB","julia":"Julia","r":"R","pytwoway":"PyTwoWay"}

def rows(path):
    with path.open(encoding="utf-8",newline="") as handle: return list(csv.DictReader(handle))

def figure(data, selector, xname, metrics, output):
    fig,axes=plt.subplots(1,2,figsize=(10.5,4.1))
    for role in COLORS:
        chosen=sorted((row for row in data if row["role"]==role and selector(row)),key=lambda row:float(row[xname]))
        x=[float(row[xname]) for row in chosen]
        for ax,(metric,ylabel) in zip(axes,metrics):
            y=[float(row[f"{metric}_median"]) for row in chosen]
            lo=[float(row[f"{metric}_min"]) for row in chosen]; hi=[float(row[f"{metric}_max"]) for row in chosen]
            ax.plot(x,y,marker="o",label=LABELS[role],color=COLORS[role],linewidth=1.8)
            ax.fill_between(x,lo,hi,color=COLORS[role],alpha=.12); ax.set_ylabel(ylabel); ax.grid(alpha=.25)
    for ax in axes: ax.set_xscale("log",base=2); ax.set_yscale("log"); ax.set_xlabel("Rows" if xname=="rows" else "CPU cores")
    axes[0].legend(frameon=False,ncol=2,fontsize=8); fig.tight_layout()
    for suffix in ("pdf","svg","png"): fig.savefig(output.with_suffix("."+suffix),dpi=220,bbox_inches="tight")
    plt.close(fig)

def main():
    parser=argparse.ArgumentParser(); parser.add_argument("--aggregate",type=Path,required=True); parser.add_argument("--exact",type=Path,required=True); parser.add_argument("--output",type=Path,required=True); args=parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=True); data=rows(args.aggregate/"performance_summary.csv")
    metrics=(("primary_seconds","Primary runtime (seconds)"),("phase_peak_rss_mib","Primary peak RSS (MiB)"))
    figure(data,lambda row:int(row["cores"])==28,"rows",metrics,args.output/"scaling_by_rows")
    figure(data,lambda row:int(row["rows"])==122880,"cores",metrics,args.output/"scaling_by_cores")
    consistency=rows(args.aggregate/"consistency_summary.csv"); exact=json.loads(args.exact.read_text(encoding="utf-8"))
    nonref=[r for r in consistency if r["role"]!="fevc"]
    overlap=sum(r["range_overlap"]=="True" for r in nonref)
    text=f"""# Five-way FE variance-component benchmark

This report compares FEVC's Rust backend, maintained KSS MATLAB, VarianceComponentsHDFE.jl, LeaveOutKSS for R, and PyTwoWay on the same deterministic `strong_d3` data. Every worker has three distinct firm matches; all observations are movers and every match is one row. Results use match deletion, no controls, and 280 randomized projections. The timing phase starts after generic CSV import and validation and includes implementation-specific cleaning, worker-pool setup, estimation, and target extraction. Memory is the 100 ms process-tree RSS peak over that same phase.

## Performance

![Runtime and memory by dataset size](scaling_by_rows.png)

![Runtime and memory by CPU cores](scaling_by_cores.png)

Curves show medians across five fixed seeds and shaded full ranges. The five roles rotate through a five-position Latin order on each benchmark cell. BLAS is capped at one thread; the requested implementation worker/thread count is the x-axis core count.

## Consistency

MATLAB, Julia, and R report sample-covariance targets, so their raw estimates are multiplied by `(N-1)/N` before comparison with FEVC and PyTwoWay's population-`N` targets. Raw and normalized estimates are both retained. Across comparator/cell/target combinations, {overlap} of {len(nonref)} five-run ranges overlap FEVC's range. This randomized comparison is descriptive because the implementations use different probe distributions, RNG partitioning, nonlinear JLA corrections, and—in PyTwoWay—separate leverage and trace sketches.

The N=960 one-core exact audit status is **{exact['status']}**. Its gate compares every normalized implementation target with an independent dense KSS oracle using `abs(gap) <= max(1e-8, 1e-5*max(1,abs(oracle)))`.

## Reproducibility boundary

The aggregate directory contains raw call-level measurements, median/full-range performance summaries, target consistency summaries, task validations, source identities, environment receipts, application logs, process-tree traces, and SGE accounting. These results are benchmark evidence only; they do not modify the FEVC paper or promote a package release.
"""
    (args.output/"report.md").write_text(text,encoding="utf-8")
    print("FEVC_FIVE_WAY_REPORT_PASS")
if __name__=="__main__": main()
