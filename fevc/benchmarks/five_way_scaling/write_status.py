#!/usr/bin/env python3
"""Write one atomic role wrapper status receipt."""
import argparse, json, os
from pathlib import Path

parser=argparse.ArgumentParser(); parser.add_argument("--output",type=Path,required=True); parser.add_argument("--role",required=True); parser.add_argument("--app-rc",type=int,required=True); parser.add_argument("--monitor-rc",type=int,required=True); parser.add_argument("--passed",type=int,choices=(0,1),required=True)
args=parser.parse_args(); value={"schema":"FEVC-FIVE-WAY-ROLE-STATUS-V1","status":"PASS" if args.passed else "FAIL","role":args.role,"application_exit_status":args.app_rc,"monitor_exit_status":args.monitor_rc}
temporary=args.output.with_suffix(args.output.suffix+".tmp"); temporary.write_text(json.dumps(value,sort_keys=True)+"\n",encoding="utf-8"); os.replace(temporary,args.output)
