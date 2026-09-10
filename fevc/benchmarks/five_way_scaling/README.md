# FEVC five-way SCC benchmark

This harness compares FEVC Rust, maintained KSS Matlab, Julia
VarianceComponentsHDFE.jl, R LeaveOutKSS, and PyTwoWay on the frozen protocol
in [PROTOCOL.md](PROTOCOL.md).

The normal sequence is:

1. Run the Python tests and generate all four manifests.
2. Deploy a new run with `deploy_scc.sh`; deployment refuses an existing run.
3. Submit `prepare`, then `smoke`, `exact`, and `pilot`, validating scheduler,
   application, and task receipts after each stage.
4. Submit the 45-task `confirmation` array only after those gates pass.
5. Aggregate with `aggregate.py`, generate figures/report with
   `report/analyze_report.py`, and compile the report PDF.

Comparator archives and environments live only in the isolated SCC run; no
licensed MATLAB source or third-party source tree is copied into this repo.
The R source remains unchanged. `r_run.R` applies the documented benchmark-only
adapter to the package's imported `detectCores` binding so its internal
`detectCores()-1` registration uses exactly the requested worker count, then
verifies that count and the worker PIDs.
