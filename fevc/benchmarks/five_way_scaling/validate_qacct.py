#!/usr/bin/env python3
"""Validate complete scalar or array SGE accounting."""
from __future__ import annotations
import argparse, json, re
from pathlib import Path

def main() -> int:
    parser=argparse.ArgumentParser(); parser.add_argument("--input",type=Path,required=True); parser.add_argument("--expected",type=int,required=True); parser.add_argument("--output",type=Path,required=True); args=parser.parse_args()
    records=[]; current={}
    for line in args.input.read_text(encoding="utf-8").splitlines():
        if line.startswith("===="):
            if current: records.append(current); current={}
            continue
        match=re.match(r"^(\S+)\s+(.+?)\s*$",line)
        if match: current[match.group(1)]=match.group(2)
    if current: records.append(current)
    records=[r for r in records if "jobnumber" in r]
    if len(records)!=args.expected: raise ValueError(f"expected {args.expected} qacct records, found {len(records)}")
    taskids=[]
    for record in records:
        if record.get("failed")!="0" or record.get("exit_status")!="0": raise ValueError("nonzero SGE accounting")
        task_value=record.get("taskid", "1").strip().lower()
        taskids.append(1 if task_value in {"", "none", "undefined"} else int(task_value))
    if sorted(taskids)!=list(range(1,args.expected+1)): raise ValueError("qacct task ids incomplete")
    value={"schema":"FEVC-FIVE-WAY-QACCT-V1","status":"PASS","records":len(records),"task_ids":sorted(taskids)}
    temporary=args.output.with_suffix(args.output.suffix+".tmp"); temporary.write_text(json.dumps(value,indent=2,sort_keys=True)+"\n",encoding="utf-8"); temporary.replace(args.output)
    print(f"FEVC_FIVE_WAY_QACCT_PASS records={len(records)}"); return 0
if __name__=="__main__": raise SystemExit(main())
