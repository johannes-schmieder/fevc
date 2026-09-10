from __future__ import annotations

import importlib.util
import json
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
        ".local/diagnostics/run/test.log",
        ".local/tools/compiler/build",
        "output/five_way_scaling/result.log",
        "fevc/fevc_macos_arm64.plugin",
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


def test_cleanup_preserves_evidence_and_requires_build_cache_opt_in(tmp_path, monkeypatch, capsys) -> None:
    root = tmp_path / "repo"
    root.mkdir()
    subprocess.run(["git", "init", "-q", str(root)], check=True)
    (root / ".gitignore").write_text("*.log\n.local/\noutput/\n*.plugin\n"
                                     "target/\nbuild/\n__pycache__/\n")
    keep = (
        "tracked.log", ".local/diagnostics/run.log", "output/result.log",
        "fevc/fevc_macos_arm64.plugin", "build/evidence/run.log",
        "new-source.py", "reviews/audit.log",
    )
    scratch = ("scratch.log", "pkg/__pycache__/module.pyc", "build/temp.log")
    cache = "rust/target/debug/build.log"
    for name in (*keep, *scratch, cache):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(name)
    subprocess.run(["git", "-C", str(root), "add", "-f", "tracked.log"], check=True)
    outside = tmp_path / "outside"
    outside.mkdir()
    (outside / "external.log").write_text("retain external file")
    (root / "linked").symlink_to(outside, target_is_directory=True)
    monkeypatch.setattr(MODULE, "REPO_ROOT", root)
    monkeypatch.setattr(sys, "argv", [str(SCRIPT), "--json"])
    # Exercise the actual CLI default, not a source-string approximation.
    assert MODULE.main() == 0
    assert json.loads(capsys.readouterr().out)["applied"] is False
    assert all((root / name).exists() for name in (*keep, *scratch, cache))
    report = MODULE.clean(apply=True)
    assert report.applied
    assert all(not (root / name).exists() for name in scratch)
    assert (root / cache).exists()
    MODULE.clean(apply=True, build_caches=True)
    assert not (root / cache).exists()
    assert all((root / name).read_text() == name for name in keep)
    assert (outside / "external.log").read_text() == "retain external file"


def test_unignored_child_of_disposable_directory_survives(tmp_path, monkeypatch) -> None:
    subprocess.run(["git", "init", "-q", str(tmp_path)], check=True)
    (tmp_path / ".gitignore").write_text("build/*\n!build/keep.py\n")
    (tmp_path / "build").mkdir()
    (tmp_path / "build/keep.py").write_text("source")
    (tmp_path / "build/remove.log").write_text("scratch")
    monkeypatch.setattr(MODULE, "REPO_ROOT", tmp_path)
    MODULE.clean(apply=True)
    assert (tmp_path / "build/keep.py").read_text() == "source"
    assert not (tmp_path / "build/remove.log").exists()


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
        "output/five_way_scaling/chart.pdf",
        "tmp/scratch.txt",
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
