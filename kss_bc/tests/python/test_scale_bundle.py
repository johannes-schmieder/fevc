from __future__ import annotations

import gzip
import hashlib
import importlib.util
import io
import os
import re
import subprocess
import tarfile
from pathlib import Path, PurePosixPath

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
KSS_ROOT = REPO_ROOT / "kss_bc"
BUILDER_PATH = KSS_ROOT / "benchmarks" / "build_scale_bundle.py"
ALLOWLIST_PATH = KSS_ROOT / "benchmarks" / "scale_bundle_allowlist.txt"
DEPLOYER_PATH = KSS_ROOT / "benchmarks" / "scc" / "deploy_scale_bundle.sh"
SUBMIT_RNG_K1_PATH = (
    KSS_ROOT / "benchmarks" / "scc" / "submit_rng_k1.sh"
)
SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"

spec = importlib.util.spec_from_file_location("build_scale_bundle", BUILDER_PATH)
assert spec is not None and spec.loader is not None
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


def test_allowlist_closes_installed_runtime_and_streamlined_harness() -> None:
    rows = builder.read_allowlist(ALLOWLIST_PATH)
    selected = set(rows)
    assert builder.package_runtime_paths(REPO_ROOT) <= selected
    assert builder.REQUIRED_INFRASTRUCTURE <= selected
    assert builder.REQUIRED_PACKAGE_METADATA <= selected
    assert builder.REQUIRED_DOCUMENTATION <= selected
    assert builder.REQUIRED_CMG_SOURCE <= selected
    assert not any("tests/" in str(path) for path in rows)
    assert not any(path.suffix in {".dta", ".log", ".csv", ".tsv", ".mat"}
                   for path in rows)
    assert all(
        str(path) == "cmg_plan.md" or
        str(path).startswith(("kss_bc/", "shared/cmg/"))
        for path in rows
    )
    assert PurePosixPath(
        "kss_bc/benchmarks/prod_bundle_allowlist.txt"
    ) not in selected
    assert PurePosixPath(
        "kss_bc/benchmarks/scc/submit_rng_k1.sh"
    ) not in selected
    matlab_support = {
        path for path in rows
        if "benchmarks/matlab_scale/" in str(path)
    }
    assert matlab_support == {
        PurePosixPath("kss_bc/benchmarks/matlab_scale/common.py"),
        PurePosixPath(
            "kss_bc/benchmarks/matlab_scale/monitor_process_tree.py"
        ),
        PurePosixPath(
            "kss_bc/benchmarks/matlab_scale/source_contract.json"
        ),
    }
    for relative in rows:
        payload, _ = builder.read_regular_no_symlinks(REPO_ROOT, relative)
        assert payload


def test_scale_bundle_is_deterministic_regular_and_manifest_complete() -> None:
    first, first_manifest = builder.build(
        REPO_ROOT, ALLOWLIST_PATH, SOURCE_COMMIT)
    second, second_manifest = builder.build(
        REPO_ROOT, ALLOWLIST_PATH, SOURCE_COMMIT)
    assert first == second
    assert first_manifest == second_manifest
    assert re.fullmatch(r"[0-9a-f]{64}", hashlib.sha256(first).hexdigest())

    rows = builder.read_allowlist(ALLOWLIST_PATH)
    payload_names = [
        "BUNDLE_FORMAT.txt",
        "SOURCE_COMMIT.txt",
        *(str(path) for path in rows),
    ]
    archive_names = [*payload_names, "BUNDLE_FILES.sha256"]
    with gzip.GzipFile(fileobj=io.BytesIO(first), mode="rb") as compressed:
        tar_payload = compressed.read()
    with tarfile.open(fileobj=io.BytesIO(tar_payload), mode="r:") as archive:
        members = archive.getmembers()
        member_by_name = {member.name: member for member in members}
        assert [member.name for member in members] == archive_names
        assert all(member.isfile() for member in members)
        assert all(member.uid == 0 and member.gid == 0 for member in members)
        assert all(member.mtime == 0 for member in members)
        assert archive.extractfile("BUNDLE_FORMAT.txt").read() == (
            b"KSS-STREAMLINE-SOURCE-BUNDLE-V1\n"
        )
        assert archive.extractfile("SOURCE_COMMIT.txt").read() == (
            f"{SOURCE_COMMIT}\n".encode()
        )
        assert archive.extractfile("BUNDLE_FILES.sha256").read() == (
            first_manifest.encode()
        )
        assert member_by_name[
            "kss_bc/benchmarks/scc/deploy_scale_bundle.sh"
        ].mode == 0o755

    manifest_rows = first_manifest.rstrip("\n").splitlines()
    assert len(manifest_rows) == len(payload_names)
    assert [line.split("  ", 1)[1] for line in manifest_rows] == payload_names
    assert all(re.fullmatch(r"[0-9a-f]{64}  [A-Za-z0-9._/-]+", line)
               for line in manifest_rows)


def test_builder_rejects_symlinks_in_any_component(tmp_path: Path) -> None:
    root = tmp_path / "root"
    real = root / "real"
    real.mkdir(parents=True)
    (real / "payload.txt").write_text("payload\n", encoding="utf-8")
    (root / "linked").symlink_to(real, target_is_directory=True)
    (root / "final.txt").symlink_to(real / "payload.txt")

    with pytest.raises(ValueError, match="symlink in bundle path"):
        builder.read_regular_no_symlinks(
            root, PurePosixPath("linked/payload.txt"))
    with pytest.raises(ValueError, match="symlink in bundle path"):
        builder.read_regular_no_symlinks(root, PurePosixPath("final.txt"))


@pytest.mark.parametrize(
    "unsafe",
    ("../escape", "/absolute", "path//double", "path/./dot", "bad path"),
)
def test_allowlist_rejects_unsafe_path_spellings(
    tmp_path: Path, unsafe: str,
) -> None:
    candidate = tmp_path / "allowlist.txt"
    candidate.write_text(f"{unsafe}\n", encoding="utf-8")
    with pytest.raises(ValueError):
        builder.read_allowlist(candidate)


def test_builder_check_only_creates_no_output_directory(tmp_path: Path) -> None:
    output = tmp_path / "must-not-exist"
    completed = subprocess.run(
        [
            str(REPO_ROOT / ".venv" / "bin" / "python"),
            str(BUILDER_PATH),
            "--root", str(REPO_ROOT),
            "--allowlist", str(ALLOWLIST_PATH),
            "--source-commit", SOURCE_COMMIT,
            "--output-dir", str(output),
            "--check-only",
        ],
        check=True,
        text=True,
        capture_output=True,
    )
    assert re.fullmatch(r"[0-9a-f]{64}\n", completed.stdout)
    assert completed.stderr == ""
    assert not output.exists()


def test_deployer_check_only_never_calls_remote_tools(tmp_path: Path) -> None:
    fake_bin = tmp_path / "bin"
    fake_bin.mkdir()
    marker = tmp_path / "remote-tool-called"
    for name in ("ssh", "rsync", "qsub"):
        command = fake_bin / name
        command.write_text(
            "#!/bin/sh\nprintf '%s\\n' called > \"$KSS_TEST_MARKER\"\nexit 99\n",
            encoding="utf-8",
        )
        command.chmod(0o755)
    environment = os.environ.copy()
    environment["PATH"] = f"{fake_bin}{os.pathsep}{environment['PATH']}"
    environment["KSS_TEST_MARKER"] = str(marker)
    completed = subprocess.run(
        ["bash", str(DEPLOYER_PATH), "--check-only", str(REPO_ROOT)],
        check=True,
        text=True,
        capture_output=True,
        env=environment,
    )
    assert completed.stderr == ""
    assert re.fullmatch(
        r"KSS_STREAMLINE_BUNDLE_CHECK_ONLY bundle_sha256=[0-9a-f]{64} "
        r"source_commit=[0-9a-f]{40} worktree_clean=([01]) "
        r"deployment_ready=\1\n",
        completed.stdout,
    )
    assert not marker.exists()


def test_deployer_preserves_single_job_and_prod_boundaries() -> None:
    script = DEPLOYER_PATH.read_text(encoding="utf-8")
    assert "prod_bundle_allowlist" not in script
    assert "deploy_prod_bundle" not in script
    assert "qsub" not in script
    assert "--delete" not in script
    assert "--require-git-tracked" in script
    assert "status --porcelain --untracked-files=all" in script
    assert "KSS-STREAMLINE deployment requires a clean committed checkout" in script
    assert "KSS-STREAMLINE-SOURCE-BUNDLE-V1" in script
    assert '"milestone":"KSS-STREAMLINE-1"' in script
    assert "run_rng_k1.sge" not in script
    assert "matlab_scale" not in script
    assert "one SGE job, one Stata process per estimate" in script
    assert "/projectnb/welfgr/kss-bc/bundles/$bundle_sha" in script


def test_rng_k1_submitter_is_source_bound_and_scalar() -> None:
    script = SUBMIT_RNG_K1_PATH.read_text(encoding="utf-8")
    assert len(re.findall(r"(?m)^raw_job_id=\$\(qsub ", script)) == 1
    assert " -t " not in script
    assert "-pe omp 14" in script
    assert "mem_per_core=4G" in script
    assert "h_rt=01:30:00" in script
    assert "KSS_REQUESTED_SLOTS=14" in script
    assert "KSS_STATA_PROCESSORS=4" in script
    assert "KSS_HARD_WALL_SECONDS=5400" in script
    assert "application_timeout_seconds\\t5280" in script
    assert "one_scalar_job_no_array" in script
    assert "one_process_four_processors" in script
    assert 'test "$source_dir" = "$bundle_dir/source"' in script
    assert 'sha256sum "$bundle_archive"' in script
    assert 'sha256sum -c "$source_manifest"' in script
    assert '"$output_dir/job_id.txt"' in script
    assert "run_rng_k1.sge" in script
