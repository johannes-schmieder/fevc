#!/usr/bin/env python3
"""Build a source-bound FEVC Ado that admits the registered benchmark core grid."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

try:
    from .common import atomic_json, sha256
except ImportError:
    from common import atomic_json, sha256

INSERT_AFTER = "    local implicit_match = (`fullcmg' == 1)\n"
THREAD_CALL = "        fullcmg(`fullcmg') threads(`=c(processors)')                 ///\n"
REQUEST_CHECK = "            `cmg_threads_requested'==c(processors) &              ///\n"
USED_CHECK = "            `cmg_threads_used'==c(processors) &                    ///\n"
ADAPTER = r'''    // Source-bound benchmark only: decouple the native CMG pool from
    // Stata/MP's licensed processor ceiling under a complete environment contract.
    local full_cmg_threads = c(processors)
    local b_contract : environment FEVC_FIVE_WAY_THREAD_CONTRACT
    local b_threads : environment FEVC_FIVE_WAY_RUST_THREADS
    local b_cores : environment FEVC_FIVE_WAY_ACTIVE_CORES
    local b_slots : environment FEVC_FIVE_WAY_ASSIGNED_SLOTS
    local b_any = strtrim(`"`b_contract'"')!="" | strtrim(`"`b_threads'"')!="" | ///
        strtrim(`"`b_cores'"')!="" | strtrim(`"`b_slots'"')!=""
    if `b_any' {
        local bt = real(`"`b_threads'"')
        local bc = real(`"`b_cores'"')
        local bs = real(`"`b_slots'"')
        local b_ok = `"`b_contract'"'=="FEVC-FIVE-WAY-THREADS-V1" & ///
            !missing(`bt',`bc',`bs') & `bt'==floor(`bt') & `bc'==floor(`bc') & ///
            `bs'==floor(`bs') & inlist(`bt',1,2,4,8,14,28) & `bc'==`bt' & ///
            `bs'>=`bc' & c(processors)==min(4,`bc') & `fullcmg'==1
        if !`b_ok' {
            quietly _vckss_post_failure "INVALID_TUNING" ///
                "The source-bound five-way benchmark thread contract is invalid."
            exit 198
        }
        local full_cmg_threads = `bt'
    }
'''


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if text.count(old) != 1:
        raise ValueError(f"expected one {label} anchor")
    return text.replace(old, new, 1)


def build(source: Path, output: Path, receipt: Path) -> dict[str, object]:
    text = source.read_text(encoding="utf-8")
    text = replace_once(text, INSERT_AFTER, INSERT_AFTER + ADAPTER, "insertion")
    text = replace_once(text, THREAD_CALL,
                        "        fullcmg(`fullcmg') threads(`full_cmg_threads')                 ///\n",
                        "thread call")
    text = replace_once(text, REQUEST_CHECK,
                        "            `cmg_threads_requested'==`full_cmg_threads' &              ///\n",
                        "request check")
    text = replace_once(text, USED_CHECK,
                        "            `cmg_threads_used'==`full_cmg_threads' &                    ///\n",
                        "used check")
    output.write_text(text, encoding="utf-8")
    value = {
        "schema": "FEVC-FIVE-WAY-ADO-ADAPTER-V1", "status": "PASS",
        "source_sha256": sha256(source), "output_sha256": sha256(output),
        "allowed_native_threads": [1, 2, 4, 8, 14, 28],
        "maximum_stata_processors": 4, "transformations": 4,
    }
    atomic_json(receipt, value)
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    value = build(args.source, args.output, args.receipt)
    print(f"FEVC_FIVE_WAY_ADAPTER_PASS {value['output_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
