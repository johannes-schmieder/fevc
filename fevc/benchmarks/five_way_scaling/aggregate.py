#!/usr/bin/env python3
"""Aggregate validated five-way tasks into performance and consistency tables."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path

try:
    from .common import ROLES, TARGETS, atomic_json
except ImportError:
    from common import ROLES, TARGETS, atomic_json


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    if not rows: raise ValueError("empty table")
    temporary=path.with_suffix(path.suffix+".tmp")
    with temporary.open("w",encoding="utf-8",newline="") as handle:
        writer=csv.DictWriter(handle,fieldnames=list(rows[0]),lineterminator="\n")
        writer.writeheader(); writer.writerows(rows)
    temporary.replace(path)


def elapsed_seconds(path: Path) -> float:
    for line in path.read_text(encoding="utf-8").splitlines():
        if "Elapsed (wall clock) time" in line:
            value=line.rsplit(": ",1)[-1]
            parts=[float(part) for part in value.split(":")]
            if len(parts)==2: return 60*parts[0]+parts[1]
            if len(parts)==3: return 3600*parts[0]+60*parts[1]+parts[2]
    raise ValueError(f"missing GNU time elapsed field: {path}")


def aggregate(task_root: Path, output: Path, expected: int) -> dict[str, object]:
    validations=sorted(task_root.glob("*/validation.json"))
    if len(validations)!=expected: raise ValueError(f"expected {expected} validations, found {len(validations)}")
    raw: list[dict[str,object]]=[]
    seen=set()
    for path in validations:
        value=json.loads(path.read_text(encoding="utf-8"))
        key=(value["cell_id"],int(value["repeat"]));
        if key in seen: raise ValueError("duplicate task key")
        seen.add(key)
        task_dir=path.parent
        for role in ROLES:
            item=value["roles"][role]; result=item["result"]; monitor=item["monitor"]
            row={"cell_id":value["cell_id"],"rows":int(value["rows"]),"cores":int(value["cores"]),
                 "repeat":int(value["repeat"]),"role":role,"algorithm":value["algorithm"],
                 "primary_seconds":float(result["primary_seconds"]),
                 "estimator_seconds":float(result["estimator_seconds"]),
                 "cold_seconds":elapsed_seconds(task_dir/role/"resources.txt"),
                 "phase_peak_rss_mib":float(monitor["phase_peak_rss_kib"])/1024,
                 "whole_peak_rss_mib":float(monitor["whole_peak_rss_kib"])/1024}
            for target in TARGETS: row[f"normalized_{target}"]=float(item["targets"][target])
            raw.append(row)
    output.mkdir(parents=True,exist_ok=True); write_csv(output/"raw_measurements.csv",raw)
    grouped=defaultdict(list)
    for row in raw: grouped[(row["cell_id"],row["rows"],row["cores"],row["role"])].append(row)
    performance=[]
    for (cell,n,cores,role),items in sorted(grouped.items()):
        record={"cell_id":cell,"rows":n,"cores":cores,"role":role,"repeats":len(items)}
        for metric in ("cold_seconds","primary_seconds","estimator_seconds","phase_peak_rss_mib","whole_peak_rss_mib"):
            values=[float(item[metric]) for item in items]
            record[f"{metric}_median"]=statistics.median(values); record[f"{metric}_min"]=min(values); record[f"{metric}_max"]=max(values)
        performance.append(record)
    write_csv(output/"performance_summary.csv",performance)
    consistency=[]
    by_cell_role=defaultdict(list)
    for row in raw: by_cell_role[(row["cell_id"],row["role"])].append(row)
    for cell in sorted({row["cell_id"] for row in raw}):
        reference=by_cell_role[(cell,"fevc")]
        n=int(reference[0]["rows"]); cores=int(reference[0]["cores"])
        for role in ROLES:
            items=by_cell_role[(cell,role)]
            for target in TARGETS:
                rv=[float(item[f"normalized_{target}"]) for item in reference]
                vv=[float(item[f"normalized_{target}"]) for item in items]
                gaps=[value-ref for value,ref in zip(vv,rv)]
                consistency.append({"cell_id":cell,"rows":n,"cores":cores,"role":role,"target":target,
                    "estimate_min":min(vv),"estimate_max":max(vv),"estimate_median":statistics.median(vv),
                    "fevc_min":min(rv),"fevc_max":max(rv),"median_paired_gap":statistics.median(gaps),
                    "max_absolute_paired_gap":max(map(abs,gaps)),
                    "range_overlap":max(min(vv),min(rv))<=min(max(vv),max(rv))})
    write_csv(output/"consistency_summary.csv",consistency)
    receipt={"schema":"FEVC-FIVE-WAY-AGGREGATE-V1","status":"PASS","tasks":expected,
             "estimator_calls":len(raw),"cells":len({row['cell_id'] for row in raw}),
             "roles":list(ROLES),"raw_rows":len(raw)}
    atomic_json(output/"aggregate.json",receipt); return receipt


def main() -> int:
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument("--task-root",type=Path,required=True); parser.add_argument("--output",type=Path,required=True); parser.add_argument("--expected-tasks",type=int,required=True)
    args=parser.parse_args(); value=aggregate(args.task_root,args.output,args.expected_tasks)
    print(f"FEVC_FIVE_WAY_AGGREGATE_PASS tasks={value['tasks']} calls={value['estimator_calls']}"); return 0
if __name__=="__main__": raise SystemExit(main())
