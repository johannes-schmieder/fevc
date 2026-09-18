#!/usr/bin/env python3
"""Build a source-bound benchmark Ado with decoupled full-CMG threads."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

SCHEMA = "FEVC-BENCHMARK-ADO-ADAPTER-V1"
THREAD_CONTRACT = "FEVC-BENCHMARK-THREADS-V1"
INSERT_AFTER = (
    "    local implicit_match = (`fullcmg' == 1 & \"`deletionmode'\"==\"match\" & ///\n"
    "        \"`stayersmode'\"==\"movers\" & \"`probeorder'\"!=\"\" & ///\n"
    "        !`frequencyused' & !`targetweightsupplied' & !`deletionidsupplied' & ///\n"
    "        strtrim(`\"`controls'\"')==\"\" & \"`nuisance'\"==\"joint\")\n"
)
THREAD_CALL = "        fullcmg(`fullcmg') threads(`=c(processors)')                 ///\n"
THREAD_CALL_ADAPTED = (
    "        fullcmg(`fullcmg') threads(`full_cmg_threads')                 ///\n"
)
REQUEST_CHECK = "            `cmg_threads_requested'==c(processors) &              ///\n"
REQUEST_CHECK_ADAPTED = (
    "            `cmg_threads_requested'==`full_cmg_threads' &              ///\n"
)
USED_CHECK = "            `cmg_threads_used'==c(processors) &                    ///\n"
USED_CHECK_ADAPTED = (
    "            `cmg_threads_used'==`full_cmg_threads' &                    ///\n"
)
ADAPTER = r'''    // Benchmark artifacts may decouple the native full-CMG pool from
    // Stata/MP's licensed processor ceiling.  Ordinary installed commands never
    // set this source-bound environment contract and retain c(processors).
    local full_cmg_threads = c(processors)
    local benchmark_thread_contract : environment VCKSS_BENCHMARK_THREAD_CONTRACT
    local benchmark_thread_value : environment VCKSS_BENCHMARK_RUST_THREADS
    local benchmark_active_value : environment VCKSS_BENCHMARK_ACTIVE_CORES
    local benchmark_slots_value : environment VCKSS_BENCHMARK_ASSIGNED_SLOTS
    local benchmark_thread_env =                                      ///
        strtrim(`"`benchmark_thread_contract'"')!="" |               ///
        strtrim(`"`benchmark_thread_value'"')!="" |                  ///
        strtrim(`"`benchmark_active_value'"')!="" |                  ///
        strtrim(`"`benchmark_slots_value'"')!=""
    if `benchmark_thread_env' {
        local benchmark_threads = real(`"`benchmark_thread_value'"')
        local benchmark_active = real(`"`benchmark_active_value'"')
        local benchmark_slots = real(`"`benchmark_slots_value'"')
        local benchmark_contract_ok =                                 ///
            `"`benchmark_thread_contract'"'=="FEVC-BENCHMARK-THREADS-V1"
        local benchmark_counts_ok =                                   ///
            !missing(`benchmark_threads',`benchmark_active',`benchmark_slots') & ///
            `benchmark_threads'==floor(`benchmark_threads') &          ///
            `benchmark_active'==floor(`benchmark_active') &            ///
            `benchmark_slots'==floor(`benchmark_slots') &              ///
            inlist(`benchmark_threads',1,2,4,7,8,14,28) &              ///
            `benchmark_active'==`benchmark_threads' &                  ///
            `benchmark_slots'>=`benchmark_active' &                    ///
            c(processors)==min(4,`benchmark_active')
        if !`benchmark_contract_ok' | !`benchmark_counts_ok' | `fullcmg'!=1 {
            quietly _vckss_post_failure "INVALID_TUNING"              ///
                "The source-bound benchmark thread contract is invalid."
            di as error "invalid source-bound benchmark thread contract"
            exit 198
        }
        local full_cmg_threads = `benchmark_threads'
    }
'''


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def replace_once(source: str, old: str, new: str, label: str) -> str:
    if source.count(old) != 1:
        raise ValueError(f"expected exactly one {label} anchor")
    return source.replace(old, new, 1)


def build(source_path: Path, output_path: Path, receipt_path: Path) -> dict[str, object]:
    if not source_path.is_file() or source_path.is_symlink():
        raise ValueError("invalid source Ado")
    if output_path.exists() or receipt_path.exists():
        raise ValueError("benchmark Ado output already exists")
    source = source_path.read_text(encoding="utf-8")
    adapted = replace_once(source, INSERT_AFTER, INSERT_AFTER + ADAPTER,
                           "adapter insertion")
    adapted = replace_once(adapted, THREAD_CALL, THREAD_CALL_ADAPTED,
                           "thread request")
    adapted = replace_once(adapted, REQUEST_CHECK, REQUEST_CHECK_ADAPTED,
                           "requested-thread receipt")
    adapted = replace_once(adapted, USED_CHECK, USED_CHECK_ADAPTED,
                           "used-thread receipt")
    output_path.write_text(adapted, encoding="utf-8")
    receipt = {
        "schema": SCHEMA,
        "status": "PASS",
        "thread_contract": THREAD_CONTRACT,
        "allowed_native_threads": [1, 2, 4, 7, 8, 14, 28],
        "maximum_stata_processors": 4,
        "source_path": source_path.name,
        "source_sha256": sha256(source_path),
        "output_path": output_path.name,
        "output_sha256": sha256(output_path),
        "transformations": 4,
    }
    receipt_path.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    receipt = build(args.source, args.output, args.receipt)
    print(
        "VCKSS_BENCHMARK_ADO_ADAPTER_PASS "
        f"source={receipt['source_sha256']} output={receipt['output_sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
