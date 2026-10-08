from __future__ import annotations

import gzip
import hashlib
import importlib.util
import io
import json
import os
import subprocess
import sys
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


def test_clean_deploy_reaches_remote_preflight_with_empty_snapshot_arguments(tmp_path):
    source = tmp_path / "source"
    builder = source / "rust/stata_backend/scc/build_linux_bundle.py"
    builder.parent.mkdir(parents=True)
    (source / ".venv/bin").mkdir(parents=True)
    (source / ".venv/bin/python").symlink_to(sys.executable)
    builder.write_text(
        "import hashlib, pathlib, sys\n"
        "assert '--snapshot' not in sys.argv\n"
        "path = pathlib.Path(sys.argv[sys.argv.index('--output') + 1])\n"
        "path.write_bytes(b'source-fixture')\n"
        "print('bundle_sha256=' + hashlib.sha256(path.read_bytes()).hexdigest())\n"
    )
    commands = tmp_path / "commands"
    commands.mkdir()
    stubs = {
        "git": "case \"$3\" in symbolic-ref) echo main;; status) :;; "
        "rev-parse) echo 1111111111111111111111111111111111111111;; *) exit 90;; esac\n",
        "ssh": "echo REACHED_REMOTE_PREFLIGHT; exit 19\n",
    }
    for name, body in stubs.items():
        executable = commands / name
        executable.write_text("#!/bin/sh\n" + body)
        executable.chmod(0o755)
    environment = os.environ.copy()
    environment["PATH"] = str(commands) + os.pathsep + environment["PATH"]
    result = subprocess.run(
        ["/bin/bash", str(SCC / "deploy_linux_bundle.sh"), str(source),
         "20260918T000000Z-fixture"],
        env=environment, text=True, capture_output=True,
    )
    assert result.returncode == 19, result.stderr
    assert "REACHED_REMOTE_PREFLIGHT" in result.stdout


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


@pytest.mark.parametrize("failure", ["", "centering-capability",
    "installed-centering-capability", "installed-mean-projection", "installed-hash"])
def test_linux_centering_gates_require_staged_and_installed_success(tmp_path, failure):
    source = QUALIFIER.read_text(encoding="utf-8")
    runner = source.split("last_run_directory=\n", 1)[1].split("\nenvironment_do=", 1)[0]
    staged = source.split("centering_capability_do=", 1)[1].split("\nrun_stata_case lifecycle", 1)[0]
    installed = source.split("installed_package_dir=", 1)[1].split("\nsource_after=", 1)[0]
    package = tmp_path / "package"
    install = tmp_path / "install"
    (install / "f").mkdir(parents=True)
    (package / "tests/stata").mkdir(parents=True)
    candidate = b"candidate"
    (install / "f/fevc_rust_linux_x64.plugin").write_bytes(
        b"corrupt" if failure == "installed-hash" else candidate)
    fake_stata = tmp_path / "stata"
    fake_stata.write_text(
        f"#!{sys.executable}\n"
        "import json, os, pathlib, sys\n"
        "label = pathlib.Path.cwd().name.removeprefix('stata-')\n"
        "with open(os.environ['CALLS'], 'a') as f: f.write(json.dumps(sys.argv[1:]) + '\\n')\n"
        "if label == os.environ['FAILURE']: sys.exit(0)\n"
        "name = pathlib.Path(sys.argv[4]).name\n"
        "markers = {'centering-capability.do': 'FEVC LINUX CENTERING CAPABILITY PASS',\n"
        " 'test_projection_mean_native.do': 'PASS test_projection_mean_native.do cells=8',\n"
        " 'test_centering_mean.do': 'SIMPLE_MEAN_PASS',\n"
        " 'test_centering_exact.do': 'SIMPLE_EXACT_ORACLE_PASS',\n"
        " 'test_centering_jla.do': 'SIMPLE_CORRECTED_MCSE_PASS',\n"
        " 'test_centering_options.do': 'SIMPLE_CENTERING_OPTIONS_PASS'}\n"
        "print(markers[name])\n"
    )
    fake_stata.chmod(0o755)
    driver = tmp_path / "driver.sh"
    driver.write_text(
        "set -euo pipefail\n"
        "fail() { echo \"$*\" >&2; exit 1; }\n"
        "hash_file() { shasum -a 256 \"$1\" | awk '{print $1}'; }\n"
        f'temporary_root="{tmp_path}"\n'
        f'test_package_dir="{package}"\n'
        f'install_root="{install}"\n'
        f'stata_binary="{fake_stata}"\n'
        f'candidate_sha256={hashlib.sha256(candidate).hexdigest()}\n'
        + runner + "\ncentering_capability_do=" + staged
        + "\ninstalled_package_dir=" + installed
    )
    env = dict(os.environ, CALLS=str(tmp_path / "calls.jsonl"), FAILURE=failure)
    result = subprocess.run(["bash", str(driver)], env=env, capture_output=True, text=True)
    if failure:
        assert result.returncode != 0
        assert ("artifact hash mismatch" if failure == "installed-hash"
                else "omitted PASS marker") in result.stderr
        return
    assert result.returncode == 0, result.stderr
    calls = [json.loads(line) for line in (tmp_path / "calls.jsonl").read_text().splitlines()]
    assert len(calls) == 8
    assert [call[4] for call in calls[:2]] == [str(package)] * 2
    assert [call[4] for call in calls[2:]] == [str(install / "f")] * 6
    assert [Path(call[3]).name for call in calls[4:]] == [
        "test_centering_mean.do", "test_centering_exact.do",
        "test_centering_jla.do", "test_centering_options.do"]
    assert all(call[5] == "rust" for call in calls[4:])
    capability = (tmp_path / "centering-capability.do").read_text()
    assert "assert r(centering_api)==1" in capability
    assert "assert r(projection_centering_api)==1" in capability
    assert "assert r(numerical_api)==2" in capability


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
    assert "local rust_plugin fevc__rust_macos" in source
    assert "local rust_plugin fevc__rust_linux" in source
    assert "local rust_plugin fevc__rust_windows" in source
    assert "fevc__rust_plugin_call `rust_plugin', result" in source
    assert "fevc__rust_plugin_call `rust_plugin', rhsresult" in source
    assert "fevc__rust_plugin_call fevc__rust_macos," not in source


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
    assert not any(Path(name).suffix == ".plugin" for name in files)
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
    assert "fevc/fevc__rust_comp_batch_receipt.ado" in files
    assert "rust/crates/vckss-plugin/src/generic_execution_api.rs" in files
    assert "rust/experiments/optimization_parity_20260913/run_development_smoke.sge" in files
    assert "rust/experiments/optimization_parity_20260913/run_development_cell.sge" in files
    assert not any(path.startswith("KSS_Veneto_replication/") for path in files)
    assert not any(Path(name).suffix == ".plugin" for name in files)
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
