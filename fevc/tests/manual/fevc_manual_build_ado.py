#!/usr/bin/env python3
"""Build a temporary fevc Ado with independent native benchmark threads."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


SCHEMA = "FEVC-MANUAL-BENCHMARK-ADO-V1"
THREAD_CONTRACT = "FEVC-MANUAL-BENCHMARK-THREADS-V1"
INSERT_AFTER = (
    "    local implicit_match = (`fullcmg' == 1 & \"`deletionmode'\"==\"match\" & ///\n"
    "        \"`stayersmode'\"==\"movers\" & \"`probeorder'\"!=\"\" & ///\n"
    "        !`frequencyused' & !`targetweightsupplied' & !`deletionidsupplied' & ///\n"
    "        strtrim(`\"`controls'\"')==\"\" & \"`nuisance'\"==\"joint\")\n"
)
THREAD_CALL = "        fullcmg(`fullcmg') threads(`=c(processors)')                 ///\n"
THREAD_CALL_ADAPTED = (
    "        fullcmg(`fullcmg') threads(`manual_rust_threads')                ///\n"
)
REQUEST_CHECK = "            `cmg_threads_requested'==c(processors) &              ///\n"
REQUEST_CHECK_ADAPTED = (
    "            `cmg_threads_requested'==`manual_rust_threads' &             ///\n"
)
USED_CHECK = "            `cmg_threads_used'==c(processors) &                    ///\n"
USED_CHECK_ADAPTED = (
    "            `cmg_threads_used'==`manual_rust_threads' &                   ///\n"
)
ADAPTER = r'''    // The manual benchmark may decouple the native Rust pool from
    // Stata/MP's licensed processor ceiling.  This temporary Ado alone reads
    // the manual contract; the installed public command remains unchanged.
    local manual_rust_threads = c(processors)
    local manual_thread_contract `"${FEVC_MANUAL_THREAD_CONTRACT}"'
    local manual_thread_value `"${FEVC_MANUAL_RUST_THREADS}"'
    local manual_stata_value `"${FEVC_MANUAL_STATA_THREADS}"'
    local manual_thread_env =                                      ///
        strtrim(`"`manual_thread_contract'"')!="" |               ///
        strtrim(`"`manual_thread_value'"')!="" |                  ///
        strtrim(`"`manual_stata_value'"')!=""
    if `manual_thread_env' {
        local manual_threads = real(`"`manual_thread_value'"')
        local manual_stata_threads = real(`"`manual_stata_value'"')
        local manual_contract_ok =                                 ///
            `"`manual_thread_contract'"'==                         ///
                "FEVC-MANUAL-BENCHMARK-THREADS-V1"
        local manual_counts_ok =                                   ///
            !missing(`manual_threads',`manual_stata_threads') &    ///
            `manual_threads'==floor(`manual_threads') &            ///
            `manual_stata_threads'==floor(`manual_stata_threads') & ///
            `manual_threads'>=1 & `manual_threads'<=64 &           ///
            `manual_stata_threads'==c(processors)
        if !`manual_contract_ok' | !`manual_counts_ok' | `fullcmg'!=1 {
            quietly _vckss_post_failure "INVALID_TUNING"          ///
                "The manual benchmark thread contract is invalid."
            di as error "invalid manual benchmark thread contract"
            exit 198
        }
        local manual_rust_threads = `manual_threads'
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
        raise ValueError("temporary benchmark output already exists")
    source = source_path.read_text(encoding="utf-8")
    # Public dispatch now carries an explicit native context. Accept that
    # source shape or the historical one, but never silently guess anchors.
    explicit_context = source.count("    local native_threads = c(processors)\n") == 2
    thread_call = THREAD_CALL.replace("`=c(processors)'", "`native_threads'") if explicit_context else THREAD_CALL
    request_check = REQUEST_CHECK.replace("c(processors)", "`native_threads'") if explicit_context else REQUEST_CHECK
    used_check = USED_CHECK.replace("c(processors)", "`native_threads'") if explicit_context else USED_CHECK
    adapted = replace_once(source, INSERT_AFTER, INSERT_AFTER + ADAPTER, "insertion")
    adapted = replace_once(adapted, thread_call, THREAD_CALL_ADAPTED, "thread call")
    adapted = replace_once(
        adapted, request_check, REQUEST_CHECK_ADAPTED, "requested-thread receipt"
    )
    adapted = replace_once(adapted, used_check, USED_CHECK_ADAPTED, "used-thread receipt")
    output_path.write_text(adapted, encoding="utf-8")
    receipt = {
        "schema": SCHEMA,
        "status": "PASS",
        "thread_contract": THREAD_CONTRACT,
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
        "FEVC_MANUAL_BENCHMARK_ADAPTER|PASS|"
        f"source={receipt['source_sha256']}|output={receipt['output_sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
