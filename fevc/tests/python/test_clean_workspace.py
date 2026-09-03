from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "fevc/tools/clean_workspace.py"
SPEC = importlib.util.spec_from_file_location("clean_workspace", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def test_source_bound_evidence_is_protected() -> None:
    protected = (
        ".ci/stata/results/source.json",
        ".git/objects/.DS_Store",
        ".venv/lib/python3.13/site-packages/__pycache__",
        "docs/history/old.log",
        "reviews/audit.log",
        "rust/progress/checkpoint.log",
        "rust/qualification/evidence/run.log",
        "qualification/exact-source/run.log",
        "fevc/benchmarks/reports/result.log",
        "fevc/benchmarks/projection/evidence/run.log",
        "fevc/qualification/run/test.log",
    )
    assert all(MODULE.is_protected(path) for path in protected)


def test_disposable_paths_are_not_protected() -> None:
    disposable = (
        ".ci/stata/run/current/output.log",
        ".pytest_cache",
        ".mypy_cache/state.json",
        "build/lib/module.py",
        "dist/fevc.whl",
        "fevc.egg-info/PKG-INFO",
        "rust/fuzz/target/debug/build.log",
        "rust/target/debug/build.log",
        "fevc/tests/stata/test.smcl",
        "fevc/tests/stata/test.log",
    )
    assert not any(MODULE.is_protected(path) for path in disposable)


def test_cleanup_default_is_dry_run() -> None:
    parser_source = SCRIPT.read_text(encoding="utf-8")
    assert "report = clean(apply=args.apply)" in parser_source
    assert "shutil.rmtree" in parser_source
    assert "path.unlink" in parser_source


def test_gitignore_matches_cleanup_policy() -> None:
    ignored = (
        ".mypy_cache/state.json",
        ".tox/state.json",
        ".nox/session/state.json",
        ".ipynb_checkpoints/notebook.ipynb",
        "fevc.egg-info/PKG-INFO",
        "build/lib/module.py",
        "dist/fevc.whl",
        ".coverage.worker",
        "coverage.xml",
        "test.smcl",
        "profile.asv",
    )
    for relative in ignored:
        result = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "check-ignore", "-q", "--no-index", relative],
            check=False,
        )
        assert result.returncode == 0, relative

    result = subprocess.run(
        [
            "git",
            "-C",
            str(REPO_ROOT),
            "check-ignore",
            "-q",
            "--no-index",
            "fevc/stata.toc",
        ],
        check=False,
    )
    assert result.returncode == 1
