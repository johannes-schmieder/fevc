"""A small fixed-outcome sensitivity grid, separate from sampling summaries."""
from __future__ import annotations

import numpy as np


def numerical_cells(registration):
    cells=[]
    for route in registration["routes"]:
        for reference in registration["references"]:
            for share in registration["stayer_shares"]:
                base=f'{route}_{reference}_k{registration["dimension"]}_s{round(100*share)}_primary'
                for numerical_seed in registration["numerical_seeds"]:
                    for budget,numerics in registration["budgets"].items():
                        cells.append(dict(cell=f"{base}_seed{numerical_seed}_{budget}",
                            route=route,reference=reference,k=registration["dimension"],stayer_share=share,
                            diagnostic="numerical_fixed_outcome",outcome_cell=base,budget=budget,
                            numerics=dict(numerics,seed=numerical_seed)))
    return cells


def summarize_numerical(rows, manifest):
    comparisons=[];failures=[];by_seed={};by_budget={}
    fields=("point","se","lower","upper","q1_var_b","q1_cov_br","q1_var_r")
    for row in rows:
        spec=manifest["cells"][row["cell"]]["spec"]
        by_seed.setdefault((spec["outcome_cell"],row["numerical_seed"],row["arm"],row["target"]),{})[spec["budget"]]=row
        by_budget.setdefault((spec["outcome_cell"],spec["budget"],row["arm"],row["target"]),[]).append(row)
        if row["rc"] or row["target_status"]:
            failures.append(dict(cell=row["cell"],arm=row["arm"],target=row["target"],rc=row["rc"],target_status=row["target_status"],failure=row["failure"]))
    for (outcome,numerical_seed,arm,target),budgets in sorted(by_seed.items()):
        baseline=budgets.get("baseline")
        for budget,row in sorted(budgets.items()):
            comparisons.append(dict(outcome=outcome,numerical_seed=numerical_seed,arm=arm,target=target,budget=budget,
                rc=row["rc"],target_status=row["target_status"],baseline_available=baseline is not None,
                point_minus_exact=row["point"]-row["oracle_point"] if row["point"] is not None else None,
                deviations={field:row[field]-baseline[field] if baseline is not None and row[field] is not None and baseline[field] is not None else None for field in fields}))
    dispersion=[]
    for (outcome,budget,arm,target),values in sorted(by_budget.items()):
        summaries={}
        for field in fields:
            available=[r[field] for r in values if r[field] is not None]
            summaries[field]=dict(available=len(available),minimum=min(available) if available else None,
                                  maximum=max(available) if available else None,
                                  sd=float(np.std(available,ddof=1)) if len(available)>1 else None)
        dispersion.append(dict(outcome=outcome,budget=budget,arm=arm,target=target,attempts=len(values),fields=summaries))
    return dict(status="NUMERICAL_DIAGNOSTIC_COMPLETE",scope="fixed outcomes; numerical sensitivity only, no sampling coverage claim",
                profile=manifest["profile"],manifest_sha256=manifest["identity"],comparisons=comparisons,across_seed_dispersion=dispersion,
                attempt_failures=failures,scientific_screen_failures=[],total_rows=len(rows),fixed_outcomes=len({c["spec"]["outcome_cell"] for c in manifest["cells"].values()}))
