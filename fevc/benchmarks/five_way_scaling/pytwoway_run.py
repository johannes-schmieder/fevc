#!/usr/bin/env python3
"""PyTwoWay comparator driver with explicit sample and resource contracts."""

from __future__ import annotations

import json
import os
import time
from pathlib import Path

import numpy as np
import pandas as pd
import bipartitepandas as bpd
import pytwoway as tw


def need(name: str) -> str:
    value = os.environ.get(name, "")
    if not value:
        raise ValueError(f"missing {name}")
    return value


def marker(path: str, value: str) -> None:
    Path(path).write_text(value + "\n", encoding="utf-8")


def atomic(path: Path, value: dict[str, object]) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def main() -> int:
    input_path = need("FW_INPUT"); output = Path(need("FW_OUTPUT"))
    algorithm = need("FW_ALGORITHM"); n = int(need("FW_ROWS")); cores = int(need("FW_CORES"))
    probes = int(need("FW_PROBES")); seed = int(need("FW_SEED"))
    if n not in (960, 7_680, 30_720, 122_880, 491_520) or cores not in (1,2,4,8,14,28):
        raise ValueError("grid changed")
    started = time.perf_counter(); data = pd.read_csv(input_path); import_seconds = time.perf_counter()-started
    if list(data.columns) != ["observation_key","worker","firm","period","match","y"]:
        raise ValueError("columns changed")
    if (len(data) != n or data[["worker","firm"]].drop_duplicates().shape[0] != n or
            data.worker.nunique() != n//3 or data.firm.nunique() != n//120 or
            not np.isfinite(data.y.to_numpy()).all()):
        raise ValueError("input sample changed")
    marker(need("FW_PHASE_START"), "START pytwoway"); phase_clock = time.perf_counter()
    frame = pd.DataFrame({"i":data.worker.to_numpy()-1,"j":data.firm.to_numpy()-1,
                          "y":data.y.to_numpy(),"t":data.period.to_numpy(),"w":np.ones(n)})
    clean = bpd.clean_params({"connectedness":"leave_out_observation",
                              "collapse_at_connectedness_measure":True,
                              "drop_single_stayers":True,"is_sorted":True,
                              "copy":False,"verbose":False})
    bdf = bpd.BipartiteDataFrame(frame,log=False).clean(clean)
    if len(bdf) != n or bdf.n_workers() != n//3 or bdf.n_firms() != n//120:
        raise ValueError("PyTwoWay cleaning changed the common sample")
    exact = algorithm == "exact"
    params = tw.fe_params({"ho":False,"he":True,
        "Q_var":[tw.Q.VarPsi(),tw.Q.VarAlpha()],"Q_cov":[tw.Q.CovPsiAlpha()],
        "ncore":cores,"ndraw_lev_he":max(cores,probes),"ndraw_trace_he":max(1,probes),
        "exact_lev_he":exact,"exact_trace_he":exact,"preconditioner":"jacobi",
        "weighted":True,"progress_bars":False,"verbose":False})
    if params["solver_tol"] != 1e-10:
        raise ValueError("PyTwoWay default solver tolerance changed")
    estimator = tw.FEEstimator(bdf,params); estimator.fit(rng=np.random.default_rng(seed))
    primary_seconds = time.perf_counter()-phase_clock; marker(need("FW_PHASE_END"),"END pytwoway")
    result = estimator.res
    raw = {"worker":float(result["var(alpha)_he"]),"firm":float(result["var(psi)_he"]),
           "covariance":float(result["cov(psi, alpha)_he"])}
    raw["total"] = raw["worker"]+raw["firm"]+2*raw["covariance"]
    if not all(np.isfinite(list(raw.values()))):
        raise ValueError("nonfinite targets")
    record: dict[str, object] = {"schema":"FEVC-FIVE-WAY-ROLE-V1","status":"PASS",
        "role":"pytwoway","algorithm":algorithm,"rows":n,"cores":cores,"probes":probes,
        "seed":seed,"import_seconds":import_seconds,"primary_seconds":primary_seconds,
        "estimator_seconds":float(result["total_time"]),"normalization_factor":1.0,
        "retained_rows":n,"solver_tolerance":params["solver_tol"],
        "preconditioner":params["preconditioner"],
        "rng_policy":"NUMPY_DEFAULT_RNG_SEED" if not exact else "NONE_EXACT"}
    for target,value in raw.items():
        record[f"raw_{target}"] = value; record[f"normalized_{target}"] = value
    atomic(output,record)
    print(f"FEVC_FIVE_WAY_ROLE_PASS pytwoway {algorithm} rows={n} cores={cores}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
