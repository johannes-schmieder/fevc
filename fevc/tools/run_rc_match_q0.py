#!/usr/bin/env python3
"""Fresh match-q0 confirmation; immutable repair validators, original q0 gates."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path


_PATH = Path(__file__).with_name("run_inference_repair_campaign.py")
_SPEC = importlib.util.spec_from_file_location("fevc_rc_match_q0_adapter", _PATH)
assert _SPEC is not None and _SPEC.loader is not None
CAMPAIGN = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = CAMPAIGN
_SPEC.loader.exec_module(CAMPAIGN)

# A private module instance avoids changing the accepted repair registration or
# the independently importable q1 validator. No production estimator is patched.
CAMPAIGN.MANIFEST_SCHEMA = "fevc-rc-match-q0-manifest-v1"
CAMPAIGN.PREFLIGHT_RECEIPT_SCHEMA = "fevc-rc-match-q0-preflight-receipt-v1"
CAMPAIGN.ROW_SCHEMA = "fevc-rc-match-q0-row-v1"
CAMPAIGN.TASK_RECEIPT_SCHEMA = "fevc-rc-match-q0-task-receipt-v1"
CAMPAIGN.BUILD_RECEIPT_SCHEMA = "fevc-rc-match-q0-scc-build-v1"
CAMPAIGN.SUMMARY_SCHEMA = "fevc-rc-match-q0-summary-v1"
CAMPAIGN.REGISTRATION_SCHEMA = "FEVC_RC_MATCH_Q0_V1"
CAMPAIGN.REGISTRATION_PATH = Path("fevc/docs/rc_match_q0_v1.json")
CAMPAIGN.MASTER_SEED = 0x73BD_6A80_912F_C4E5
CAMPAIGN.CONFIRMATION_SEED = 0xECA1_8F43_7D60_B295
CAMPAIGN.FIXED_FOLD_SEED = 0x593F_7D82_A106_CE4B
CAMPAIGN.CELLS = tuple(
    {
        **{key: value for key, value in cell.items() if key != "coverage_eligible"},
        "reference_distribution": "q0",
        "coverage_targets": list(CAMPAIGN.TARGETS)
        if cell["gate"] in {"correct", "mild", "descriptive"} else [],
    }
    for cell in CAMPAIGN.BASE.CELLS
)
CAMPAIGN.CELL_BY_NAME = {cell["cell"]: cell for cell in CAMPAIGN.CELLS}
CAMPAIGN.PROFILE_DEFAULTS = {
    "tiny": {
        "cells": CAMPAIGN.CELLS, "k": 20, "replications": 2, "shard_size": 2,
        "settings": CAMPAIGN.TASK_SETTINGS, "preflight_spectrum_probes": 4096,
    },
    "smoke": {
        "cells": (CAMPAIGN.CELL_BY_NAME["controls_varying_fixedoffset"],),
        "k": 20, "replications": 2, "shard_size": 2,
        "settings": CAMPAIGN.TASK_SETTINGS, "preflight_spectrum_probes": 4096,
    },
    "confirmation": {
        "cells": CAMPAIGN.CELLS, "k": 20, "replications": 2500, "shard_size": 500,
        "settings": CAMPAIGN.TASK_SETTINGS, "preflight_spectrum_probes": 4096,
    },
}


def _registration_identity(root: Path) -> dict:
    path = root / CAMPAIGN.REGISTRATION_PATH
    payload = path.read_bytes()
    registration = json.loads(payload)
    if (registration.get("schema") != CAMPAIGN.REGISTRATION_SCHEMA
            or registration.get("status") != "CONFIRMATION_REGISTERED"
            or registration.get("cells") != list(CAMPAIGN.CELLS)
            or registration.get("thresholds") != CAMPAIGN.THRESHOLDS):
        raise CAMPAIGN.CampaignError("RC q0 registration contract mismatch")
    frozen = registration.get("source_binding", {}).get("frozen_file_sha256")
    if not isinstance(frozen, dict) or not frozen:
        raise CAMPAIGN.CampaignError("RC q0 registration has no frozen source inventory")
    for relative, expected in frozen.items():
        candidate = root / relative
        if not candidate.is_file() or CAMPAIGN._sha256(candidate.read_bytes()) != expected:
            raise CAMPAIGN.CampaignError(f"RC q0 frozen hash mismatch: {relative}")
    return {"path": CAMPAIGN.REGISTRATION_PATH.as_posix(),
            "schema": CAMPAIGN.REGISTRATION_SCHEMA, "sha256": CAMPAIGN._sha256(payload)}


_repair_preflight = CAMPAIGN._preflight_failures


def _preflight_failures(rows, cells, k):
    # The original q0 checks also enforce the equal/unequal effective-count pair.
    return sorted(set(_repair_preflight(rows, cells, k)
                      + CAMPAIGN.BASE._preflight_failures(rows, cells, k)))


def _scientific_failures(summaries, profile):
    return CAMPAIGN.BASE._scientific_failures(
        summaries, "development" if profile == "confirmation" else profile
    )


CAMPAIGN._registration_identity = _registration_identity
CAMPAIGN._preflight_failures = _preflight_failures
CAMPAIGN._scientific_failures = _scientific_failures


def main(argv=None):
    args = CAMPAIGN._parser().parse_args(argv)
    try:
        if args.command == "run-preflight":
            result = CAMPAIGN.run_preflight(args.root.resolve(), args.profile, args.output_dir, args.binary)
        elif args.command == "create-manifest":
            result = CAMPAIGN.create_manifest(args.root.resolve(), args.profile, args.output, args.preflight_receipt, args.shard_size)
        elif args.command == "run-task":
            result = CAMPAIGN.run_task(args.manifest, args.task_id, args.output_dir, args.binary)
        else:
            result = CAMPAIGN.aggregate(args.manifest, args.task_dir, args.output_dir, args.build_receipt)
    except (CAMPAIGN.CampaignError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(json.dumps({"status": "failure", "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 1 if result["status"] == "FAIL" else 0


if __name__ == "__main__":
    raise SystemExit(main())
