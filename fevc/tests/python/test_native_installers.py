import importlib.util
from pathlib import Path

import pytest

SCRIPT = Path(__file__).resolve().parents[2] / "tools/verify_native_installers.py"
SPEC = importlib.util.spec_from_file_location("native_installers", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_stata_lowercase_notice_names_are_resolved(tmp_path):
    folder = tmp_path / "l"
    folder.mkdir()
    notice = folder / "license"
    notice.write_bytes(b"exact notice bytes")
    assert MODULE.installed_file(tmp_path, "LICENSE") == notice
    assert MODULE.installed_file(tmp_path, "LICENSE").read_bytes() == b"exact notice bytes"


def test_missing_and_duplicate_installed_files_are_rejected(tmp_path):
    with pytest.raises(ValueError, match="found 0"):
        MODULE.installed_file(tmp_path, "LICENSE")
    for dirname in ("l", "duplicate"):
        folder = tmp_path / dirname
        folder.mkdir()
        (folder / "license").write_bytes(b"notice")
    with pytest.raises(ValueError, match="found 2"):
        MODULE.installed_file(tmp_path, "LICENSE")


def test_repository_catalog_installs_all_platform_plugins():
    root = SCRIPT.parents[2]
    entries = [line[2:] for line in (root / "fevc.pkg").read_text().splitlines()
               if line.startswith(("f ", "F "))]
    plugins = [Path(entry).name for entry in entries if entry.endswith(".plugin")]
    assert len(plugins) == len(set(plugins))
    assert set(plugins) == MODULE.REPOSITORY_PLUGINS
    # Git source archives deliberately omit the binaries; direct repository
    # installs download them. The portable manifest must remain binary-free.
    portable = (root / "fevc/fevc.pkg").read_text()
    assert ".plugin" not in portable


def test_pooled_markers_ignore_stata_echoes_and_require_both_backends():
    echoed = '. display "FEVC POOLED COMPONENT INFERENCE PASS: `backend\'"\n'
    mata = 'FEVC POOLED COMPONENT INFERENCE PASS: mata\n'
    rust = 'FEVC POOLED COMPONENT INFERENCE PASS: rust\n'
    assert MODULE.pooled_checks_passed((echoed + mata) * 2 + (echoed + rust) * 2)
    assert not MODULE.pooled_checks_passed(echoed * 4)
    assert not MODULE.pooled_checks_passed(mata * 4)
    assert not MODULE.pooled_checks_passed(mata * 2 + rust)
    assert not MODULE.pooled_checks_passed(mata * 2 + rust * 3)
