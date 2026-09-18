from __future__ import annotations

import gzip
import hashlib
import importlib.util
import io
import subprocess
import tarfile
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
SCC = ROOT / "rust/stata_backend/scc"
QUALIFIER = ROOT / "rust/stata_backend/qualify_linux_scc.sh"
BUILDER = SCC / "build_linux_bundle.py"
DIRTY_BUILDER = ROOT / "rust/experiments/optimization_parity_20260913/build_dirty_linux_bundle.py"


def load_builder(path=BUILDER):
    spec = importlib.util.spec_from_file_location("linux_scc_bundle", path)
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
        "rustc 1.85.1",
        "sha256sum -c SOURCE_FILES.sha256",
        "--locked --all-targets -- -D warnings",
        "cargo_fmt_binary=$(command -v cargo-fmt)",
        "cargo_clippy_binary=$(command -v cargo-clippy)",
        'RUSTFMT="${rustfmt_binary}"',
        '"${cargo_fmt_binary}" --manifest-path "${manifest_path}"',
        '"${cargo_clippy_binary}" clippy --manifest-path "${manifest_path}"',
        "<vckss-rust-1.85.1-cargo-fmt> --manifest-path",
        "<vckss-rust-1.85.1-cargo-clippy> clippy --manifest-path",
        "cshim_interrupt_test.c",
        "cshim_error_transport_test.c",
        "abi_header_compat_test.c",
        "ELF 64-bit LSB shared object, x86-64",
        'chmod -R u+w "${test_package_dir}"',
        "FEVC RUST PLUGIN PASS",
        "FEVC RUST MATA DIAGNOSTIC PASS",
        "FEVC TEST SUITE PASS: full",
        "VCKSS_STATA_CASE_CWD=${test_root}",
        "VCKSS_STATA_MARKER_FILE=${test_root}/run_all.log",
        "marker file existed before the fresh process",
        "omitted PASS marker from its fresh batch log",
        "PASS test_rust_public_install.do",
        "CLEAN_SCC_LINUX_X86_64_CANDIDATE_QUALIFICATION",
        "DIRTY_SCC_LINUX_X86_64_CANDIDATE_QUALIFICATION",
    ):
        assert required in qualifier
    assert (
        '"${script_dir}/tests/cshim_error_transport_test.c" \\\n'
        '  -lm -Wl,--gc-sections -o "${cshim_error}"'
    ) in qualifier
    assert 'plugin_cargo fmt ' not in qualifier
    assert 'plugin_cargo clippy ' not in qualifier
    assert (
        "public-release,Windows,macOS,native-Intel,representative-scale,"
        "human-license-provenance-review"
    ) in qualifier


def test_scc_wrapper_preserves_pinned_stata_and_rust_tools() -> None:
    wrapper = (SCC / "run_linux_qualifier.sge").read_text(encoding="utf-8")
    assert "module load stata-mp/19" in wrapper
    assert "stata_binary=${SCC_STATA_MP_BIN:" in wrapper
    assert 'test -x "${rust_toolchain}/bin/rustfmt"' in wrapper
    assert 'test -x "${rust_toolchain}/bin/cargo-fmt"' in wrapper
    assert 'test -x "${rust_toolchain}/bin/cargo-clippy"' in wrapper
    assert (
        "export PATH=${rust_toolchain}/bin:${SCC_STATA_MP_BIN}:/usr/bin:/bin"
        in wrapper
    )
    assert '--stata "${stata_binary}"' in wrapper
    assert "export PATH=${rust_toolchain}/bin:/usr/bin:/bin" not in wrapper
    assert "SOURCE_SNAPSHOT_KIND.txt" in wrapper


def test_clean_install_distinguishes_macos_from_unix_linux() -> None:
    install_test = (
        ROOT / "fevc/tests/stata/test_rust_public_install.do"
    ).read_text(encoding="utf-8")
    assert """local machine_type = lower(`"`c(machine_type)'"')""" in install_test
    assert """if strpos(`"`machine_type'"',"mac")""" in install_test
    assert """else if `"`c(os)'"' == "Unix""" in install_test


def test_planned_v4_raw_receipt_test_uses_the_active_platform_plugin() -> None:
    source = (
        ROOT / "fevc/tests/stata/test_rust_planned_v4.do"
    ).read_text(encoding="utf-8")
    assert "local rust_plugin _fevc_rust_macos" in source
    assert "local rust_plugin _fevc_rust_linux" in source
    assert "local rust_plugin _fevc_rust_windows" in source
    assert "_fevc_rust_plugin_call `rust_plugin', result" in source
    assert "_fevc_rust_plugin_call `rust_plugin', rhsresult" in source
    assert "_fevc_rust_plugin_call _fevc_rust_macos," not in source


def test_scale_fixture_does_not_require_user_written_egen_extensions() -> None:
    source = (
        ROOT / "fevc/tests/stata/test_scale_fixtures.do"
    ).read_text(encoding="utf-8")
    assert "nvals(" not in source
    assert "connector_firms_distinct" in source
    assert "firm[1] != firm[_N] if connector" in source


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
        "fevc/tests/stata/run_all.do",
        "fevc/tests/stata/test_rust_public_install.do",
    ):
        assert required in files
    assert files["SOURCE_COMMIT.txt"] == f"{commit}\n".encode()
    manifest = files["SOURCE_FILES.sha256"].decode().splitlines()
    assert len(manifest) + 1 == first_count
    for row in manifest:
        expected, relative = row.split("  ", 1)
        assert relative != "SOURCE_FILES.sha256"
        assert hashlib.sha256(files[relative]).hexdigest() == expected


def test_dirty_bundle_binds_current_source_without_private_veneto() -> None:
    commit = subprocess.run(
        ["git", "-C", str(ROOT), "rev-parse", "HEAD"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    builder = load_builder(DIRTY_BUILDER)
    first, digest, count = builder.build(ROOT, commit)
    second, second_digest, second_count = builder.build(ROOT, commit)
    assert first == second
    assert digest == second_digest == hashlib.sha256(first).hexdigest()
    assert count == second_count
    with tarfile.open(fileobj=io.BytesIO(first), mode="r:gz") as archive:
        files = {
            member.name.removeprefix("source/"): archive.extractfile(member).read()
            for member in archive.getmembers() if member.isfile()
        }
    assert files["SOURCE_SNAPSHOT_KIND.txt"] == b"DIRTY_WORKTREE_SNAPSHOT_V1\n"
    assert files["SOURCE_COMMIT.txt"] == f"{commit}\n".encode()
    assert "fevc/_fevc_rust_comp_batch_receipt.ado" in files
    assert "rust/crates/vckss-plugin/src/generic_execution_api.rs" in files
    assert "rust/experiments/optimization_parity_20260913/run_development_smoke.sge" in files
    assert "rust/experiments/optimization_parity_20260913/run_development_cell.sge" in files
    assert not any(path.startswith("KSS_Veneto_replication/") for path in files)
    assert len(files) == count
    for row in files["SOURCE_FILES.sha256"].decode().splitlines():
        expected, relative = row.split("  ", 1)
        assert hashlib.sha256(files[relative]).hexdigest() == expected


def test_dirty_bundle_rejects_unexpected_untracked_source(monkeypatch, tmp_path) -> None:
    builder = load_builder(DIRTY_BUILDER)
    def fake_paths(_root, *arguments):
        return [Path("safe.txt")] if arguments == ("--cached",) else [Path("private.csv")]
    monkeypatch.setattr(builder, "git_paths", fake_paths)
    with pytest.raises(ValueError, match="unexpected untracked source path"):
        builder.source_rows(tmp_path)


def test_pipeline_snapshot_extension_admits_code_not_data() -> None:
    from pathlib import PurePosixPath
    builder = load_builder(DIRTY_BUILDER)
    for name in (
        "rust/experiments/pipeline_optimization_20260915/campaign.py",
        "rust/experiments/pipeline_optimization_20260915/exact_transport_smoke.do",
        "fevc/tests/python/test_macos_toolchain_identity.py",
    ):
        assert builder.permitted_untracked(PurePosixPath(name))
    for name in (
        "rust/experiments/pipeline_optimization_20260915/private.csv",
        "rust/experiments/pipeline_optimization_20260915/credentials.env",
        "rust/experiments/unknown/source.py",
        "fevc/tests/python/private.csv",
        "fevc/tests/python/unreviewed.py",
        "KSS_Veneto_replication/private.csv",
    ):
        assert not builder.permitted_untracked(PurePosixPath(name))
