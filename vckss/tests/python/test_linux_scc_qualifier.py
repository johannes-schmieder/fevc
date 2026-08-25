from __future__ import annotations

import gzip
import hashlib
import importlib.util
import io
import subprocess
import tarfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
SCC = ROOT / "rust/stata_backend/scc"
QUALIFIER = ROOT / "rust/stata_backend/qualify_linux_scc.sh"
BUILDER = SCC / "build_linux_bundle.py"


def load_builder():
    spec = importlib.util.spec_from_file_location("linux_scc_bundle", BUILDER)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_linux_scc_shell_entrypoints_are_syntactically_valid() -> None:
    scripts = [
        QUALIFIER,
        SCC / "deploy_linux_bundle.sh",
        SCC / "run_linux_qualifier.sge",
        SCC / "submit_linux_qualifier.sh",
    ]
    subprocess.run(["bash", "-n", *map(str, scripts)], check=True)
    for script in scripts:
        assert script.stat().st_mode & 0o111


def test_linux_qualifier_keeps_platform_and_scientific_gates() -> None:
    qualifier = QUALIFIER.read_text(encoding="utf-8")
    for required in (
        "uname -m) == x86_64",
        "rustc 1.84.0",
        "sha256sum -c SOURCE_FILES.sha256",
        "cargo clippy",
        "--locked --all-targets -- -D warnings",
        "cshim_interrupt_test.c",
        "cshim_error_transport_test.c",
        "abi_header_compat_test.c",
        "ELF 64-bit LSB shared object, x86-64",
        "VCKSS RUST PLUGIN PASS",
        "VCKSS RUST MATA DIAGNOSTIC PASS",
        "VCKSS TEST SUITE PASS: full",
        "PASS test_rust_public_install.do",
        "CLEAN_SCC_LINUX_X86_64_CANDIDATE_QUALIFICATION",
    ):
        assert required in qualifier
    assert "Windows,macOS,native-Intel,scale" in qualifier


def test_exact_commit_bundle_is_deterministic_and_self_verifying() -> None:
    commit = subprocess.run(
        ["git", "-C", str(ROOT), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    builder = load_builder()
    first, first_sha, first_count = builder.build(ROOT, commit)
    second, second_sha, second_count = builder.build(ROOT, commit)
    assert first == second
    assert first_sha == second_sha == hashlib.sha256(first).hexdigest()
    assert first_count == second_count

    with gzip.GzipFile(fileobj=io.BytesIO(first), mode="rb") as compressed:
        tar_payload = compressed.read()
    with tarfile.open(fileobj=io.BytesIO(tar_payload), mode="r:") as archive:
        files = {
            member.name.removeprefix("source/"): archive.extractfile(member).read()
            for member in archive.getmembers()
            if member.isfile()
        }
    for required in (
        "SOURCE_COMMIT.txt",
        "SOURCE_FILES.sha256",
        "rust/stata_backend/qualify_linux_scc.sh",
        "rust/stata_backend/scc/run_linux_qualifier.sge",
        "vckss/tests/stata/run_all.do",
        "vckss/tests/stata/test_rust_public_install.do",
    ):
        assert required in files
    assert files["SOURCE_COMMIT.txt"] == f"{commit}\n".encode()
    manifest = files["SOURCE_FILES.sha256"].decode().splitlines()
    assert len(manifest) + 1 == first_count
    for row in manifest:
        expected, relative = row.split("  ", 1)
        assert relative != "SOURCE_FILES.sha256"
        assert hashlib.sha256(files[relative]).hexdigest() == expected
