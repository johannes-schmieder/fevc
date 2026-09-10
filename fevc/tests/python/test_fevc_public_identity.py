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


def test_obsolete_vckss_working_tree_is_rejected() -> None:
    assert MODULE.audit(["vckss/README.md"]) == [
        "vckss/README.md: obsolete predecessor working-tree path"
    ]


def test_internal_vckss_protocol_names_are_not_public_identity() -> None:
    for value in (
        "vckss__version()",
        "__vckss_rust_state",
        "VCKSS_STATA_CASE_CWD",
        "vckss-plugin",
    ):
        assert not any(pattern.search(value) for pattern in MODULE.PUBLIC_PATTERNS)


def test_legacy_distributed_runtime_filenames_are_rejected() -> None:
    for value in (
        "vckss.mata",
        "vckss_lifecycle.ado",
        "_vckss_rust_public_call.ado",
        "vckss_rust_macos.plugin",
    ):
        assert MODULE.LEGACY_DISTRIBUTED_BASENAME.fullmatch(value)
    for value in (
        "fevc.mata",
        "_fevc_display.ado",
        "_fevc_lifecycle.ado",
        "fevc_estat.ado",
        "_fevc_rust_public_call.ado",
        "fevc_rust_macos.plugin",
    ):
        assert not MODULE.LEGACY_DISTRIBUTED_BASENAME.fullmatch(value)


def test_old_public_invocations_are_rejected() -> None:
    for value in (
        "program define vckss, eclass",
        "quietly vckss y, worker(i) firm(j)",
        "help vckss",
        'ereturn local cmd "vckss"',
    ):
        assert any(pattern.search(value) for pattern in MODULE.PUBLIC_PATTERNS)


def test_private_vckss_build_ids_cannot_be_rebranded() -> None:
    for value in (
        "fevc-api21-stayer-hybrid",
        "fevc-inference-api1-block-projection",
    ):
        assert MODULE.RENAMED_PRIVATE_BUILD_ID.search(value)
    for value in (
        "vckss-api24-control-lanes256",
        "vckss-inference-api1-block-projection",
    ):
        assert not MODULE.RENAMED_PRIVATE_BUILD_ID.search(value)
