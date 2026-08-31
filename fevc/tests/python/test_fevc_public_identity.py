from __future__ import annotations

import importlib.util
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "fevc/tools/check_fevc_public_identity.py"
SPEC = importlib.util.spec_from_file_location("fevc_public_identity", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_current_tree_has_no_former_public_identity() -> None:
    assert MODULE.audit(MODULE.candidates()) == []


def test_private_vckss_names_are_not_public_identity() -> None:
    for value in (
        "vckss.mata",
        "_vckss_rust_public_call",
        "vckss__version()",
        "VCKSS_STATA_CASE_CWD",
        "vckss-plugin",
    ):
        assert not any(pattern.search(value) for pattern in MODULE.PUBLIC_PATTERNS)


def test_old_public_invocations_are_rejected() -> None:
    for value in (
        "program define vckss, eclass",
        "quietly vckss y, worker(i) firm(j)",
        "help vckss",
        'ereturn local cmd "vckss"',
    ):
        assert any(pattern.search(value) for pattern in MODULE.PUBLIC_PATTERNS)
