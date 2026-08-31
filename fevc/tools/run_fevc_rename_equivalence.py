#!/usr/bin/env python3
"""Qualify the vckss -> fevc hard-cut rename at 0.5.0-alpha.1.

The runner is intentionally fail-closed.  It accepts only the registered
predecessor, requires a clean committed candidate equal to HEAD, archives
both revisions, and starts a new Stata process for every role/case pair.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import os
import re
import secrets
import shutil
import subprocess
import sys
import tarfile
import tempfile
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path, PurePosixPath

BASELINE_COMMIT = "a5805145b93961a98068de3452ff793013847942"
BASELINE_TREE = "7a3753d0a8550b3de7e8dee32e5671dc35b6d05d"
RECEIPT_SCHEMA = "fevc-rename-equivalence-qualification-v1"
EVIDENCE_DIR = Path("fevc/qualification/fevc_rename_equivalence")
RECEIPT_PATH = EVIDENCE_DIR / "receipt.json"
ORCHESTRATOR_PATH = Path("fevc/tools/run_fevc_rename_equivalence.py")
DRIVER_PATH = Path("fevc/tests/equivalence/fevc_equivalence_driver.do")
TSV_COLUMNS = ("case_id", "kind", "name", "row", "col", "value")

REQUIRED_CASE_IDS = (
    "exact_match_joint",
    "exact_match_fixed_fw_target",
    "exact_observation_joint_fw_target",
    "jla_match_generic_controls",
    "jla_observation_generic_fixed",
    "compressed_match_fw_target",
    "rust_exact_match_joint",
    "rust_compressed_match_fw_target",
    "cmg_forced_cheap_1200x300",
    "cmg_auto_cheap_1200x300",
    "auto_diagonal_small",
    "failure_cross_coordinate_match",
    "failure_exact_size_limit",
    "failure_singular_nuisance",
    "failure_fastpath_controls",
    "failure_invalid_memory",
)

EXPECTED_FAILURES = {
    "failure_cross_coordinate_match": (198, "CROSS_COORDINATE_MATCH"),
    "failure_exact_size_limit": (198, "EXACT_SIZE_LIMIT"),
    "failure_singular_nuisance": (498, "SINGULAR_NUISANCE_BLOCK"),
    "failure_fastpath_controls": (498, "FASTPATH_CONTROLS"),
    "failure_invalid_memory": (198, "INVALID_MEMORY_ENVELOPE"),
}

TIMING_SCALARS = frozenset(
    {
        "graph_seconds",
        "fit_seconds",
        "leverage_seconds",
        "target_seconds",
        "correction_seconds",
        "setup_seconds",
        "preconditioner_seconds",
        "schur_seconds",
        "preconditioner_apply_seconds",
        "pcg_seconds",
        "solver_backend_seconds",
        "compression_seconds",
        "life_transition_seconds",
        "life_work_seconds",
        "life_restore_seconds",
        "rng_seconds",
        "sample_selection_seconds",
        "validation_seconds",
    }
)

TIMING_MATRIX_COLUMNS = {
    "prep_profile": frozenset(range(1, 8)),
    "prep_boundary_profile": frozenset(range(1, 12)),
    "rhs_profile": frozenset(range(1, 9)),
    "route_diagnostics": frozenset({3, 22}),
    "scale_receipt": frozenset({10, 11}),
}

COMPARISON_MODE = (
    "exact_binary64_hex_and_text_except_registered_timing_and_runtime_footprint"
)

RUNTIME_FOOTPRINT_SCALARS = frozenset(
    {
        "life_mem_before_bytes",
        "life_mem_cleared_bytes",
        "life_mem_work_bytes",
        "life_mem_restored_bytes",
        "memory_forecast_bytes",
        "resource_mem_admit_bytes",
        "resource_numerical_peak_bytes",
        "resource_peak_bytes",
        "resource_raw_stata_bytes",
        "resource_restore_peak_bytes",
        "resource_select_peak_bytes",
        "resource_transition_peak_bytes",
    }
)

RUNTIME_OPTIONAL_MISSING_SCALARS = frozenset(
    {"life_mem_cleared_bytes", "life_mem_work_bytes", "life_mem_restored_bytes"}
)

RUNTIME_FOOTPRINT_MATRIX_CELLS = frozenset(
    {
        ("resource_components", row, 1) for row in (1, 2)
    }
    | {
        ("resource_forecasts", row, col)
        for row in (1, 2)
        for col in (1, 2, 3, 4, 5, 7)
    }
)

CMG_FOOTPRINT_KEYS = frozenset(
    {
        ("matrix", "resource_components", 2, 5),
        ("matrix", "route_diagnostics", 1, 25),
        ("scalar", "resource_cmg_hierarchy_bytes", 0, 0),
        ("scalar", "resource_routed_solver_bytes", 0, 0),
        ("scalar", "route_forecast_peak_bytes", 0, 0),
    }
)

NORMALIZED_METADATA_NAMES = frozenset(
    {
        "role",
        "command",
        "private_prefix",
        "package_root",
        "resolved_ado",
        "cmd",
        "cmdline",
        "version",
        "stata_version",
        "stata_flavor",
        "os",
        "processors",
        "processors_lic",
        "cmg_api_level",
        "cmg_design_label",
    }
)

CMG_API_TRANSITIONS = {
    "cmg_api_level": ("+1.0000000000000X+003", "+1.0000000000000X+003"),
    "cmg_design_label": (
        "gpl-cmg-mata-degree3-hybrid-v8-vckss-component",
        "gpl-cmg-mata-degree3-hybrid-v8-vckss-component",
    ),
}

INVALID_MEMORY_HELP_TRANSITION = {
    "baseline": (
        "Check the option spelling and documented range in help vckss; "
        "do not loosen numerical tolerances to force an estimate through."
    ),
    "candidate": (
        "Check the option spelling and documented range in help fevc; "
        "do not loosen numerical tolerances to force an estimate through."
    ),
}

SUCCESS_CASES = frozenset(REQUIRED_CASE_IDS) - frozenset(EXPECTED_FAILURES)
RUST_CASE_IDS = frozenset(
    {"rust_exact_match_joint", "rust_compressed_match_fw_target"}
)


class QualificationError(RuntimeError):
    """A fail-closed qualification gate did not pass."""


@dataclass(frozen=True, order=True)
class Record:
    case_id: str
    kind: str
    name: str
    row: int
    col: int
    value: str

    @property
    def key(self) -> tuple[str, str, str, int, int]:
        return (self.case_id, self.kind, self.name, self.row, self.col)

    def as_dict(self) -> dict[str, object]:
        return {
            "case_id": self.case_id,
            "kind": self.kind,
            "name": self.name,
            "row": self.row,
            "col": self.col,
            "value": self.value,
        }


def _run(
    args: Sequence[str],
    *,
    cwd: Path,
    timeout: int | None = None,
    check: bool = True,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        list(args),
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        stdin=subprocess.DEVNULL,
        timeout=timeout,
        env=env,
    )
    if check and result.returncode != 0:
        raise QualificationError(
            f"command failed ({result.returncode}): {' '.join(args)}\n{result.stdout}"
        )
    return result


def _git(repo: Path, *args: str) -> str:
    return _run(("git", *args), cwd=repo).stdout.strip()


def _git_blob_sha256(repo: Path, commit: str, path: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{commit}:{path}"],
        cwd=repo,
        check=False,
        capture_output=True,
        stdin=subprocess.DEVNULL,
    )
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", errors="replace")
        raise QualificationError(f"could not read committed harness blob {path}: {detail}")
    return _sha256_bytes(result.stdout)


def _git_is_ancestor(repo: Path, ancestor: str, descendant: str) -> bool:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        cwd=repo,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        stdin=subprocess.DEVNULL,
    )
    if result.returncode == 0:
        return True
    if result.returncode == 1:
        return False
    detail = result.stderr.decode("utf-8", errors="replace")
    raise QualificationError(f"could not verify Git ancestry: {detail}")


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _json_bytes(payload: object) -> bytes:
    return (json.dumps(payload, indent=2, sort_keys=True) + "\n").encode("utf-8")


def _write_atomic(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{secrets.token_hex(8)}.tmp")
    try:
        with temporary.open("xb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def _write_atomic_new(path: Path, data: bytes) -> None:
    """Publish bytes atomically without ever replacing an existing path."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{secrets.token_hex(8)}.tmp")
    try:
        with temporary.open("xb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.link(temporary, path)
    except FileExistsError as exc:
        raise QualificationError(f"refusing to overwrite evidence: {path}") from exc
    finally:
        temporary.unlink(missing_ok=True)


def _require_full_sha(value: str, label: str) -> None:
    if re.fullmatch(r"[0-9a-f]{40}", value) is None:
        raise QualificationError(f"{label} must be a lowercase full 40-character commit SHA")


def _assert_clean_committed_candidate(repo: Path, candidate_arg: str) -> tuple[str, str]:
    candidate = _git(repo, "rev-parse", "--verify", f"{candidate_arg}^{{commit}}")
    head = _git(repo, "rev-parse", "--verify", "HEAD^{commit}")
    _require_full_sha(candidate, "candidate")
    if candidate_arg != candidate:
        raise QualificationError("--candidate must be the exact full candidate commit SHA")
    if candidate != head:
        raise QualificationError("candidate must equal HEAD")
    status = _git(repo, "status", "--porcelain=v1", "--untracked-files=all")
    if status:
        raise QualificationError("candidate worktree is not clean:\n" + status)
    tree = _git(repo, "rev-parse", "--verify", f"{candidate}^{{tree}}")
    _require_full_sha(tree, "candidate tree")
    return candidate, tree


def _safe_member_path(destination: Path, member_name: str) -> Path:
    pure = PurePosixPath(member_name)
    if pure.is_absolute() or not pure.parts or any(part in {"", ".", ".."} for part in pure.parts):
        raise QualificationError(f"unsafe git archive member: {member_name!r}")
    target = destination.joinpath(*pure.parts)
    target.resolve().relative_to(destination.resolve())
    return target


def _archive_revision(repo: Path, commit: str, pathspec: str, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=False)
    process = subprocess.Popen(
        ("git", "archive", "--format=tar", commit, "--", pathspec),
        cwd=repo,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert process.stdout is not None
    try:
        with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
            for member in archive:
                target = _safe_member_path(destination, member.name)
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                elif member.isfile():
                    target.parent.mkdir(parents=True, exist_ok=True)
                    source = archive.extractfile(member)
                    if source is None:
                        raise QualificationError(f"could not read archive member {member.name!r}")
                    with target.open("xb") as sink:
                        shutil.copyfileobj(source, sink)
                    target.chmod(member.mode & 0o777)
                else:
                    raise QualificationError(
                        f"git archive contains a non-regular member: {member.name!r}"
                    )
    finally:
        process.stdout.close()
    stderr = process.stderr.read().decode("utf-8", errors="replace") if process.stderr else ""
    returncode = process.wait()
    if process.stderr:
        process.stderr.close()
    if returncode != 0:
        raise QualificationError(f"git archive failed for {commit}:{pathspec}: {stderr}")


def _stage_source_bound_plugin(
    plugin_path: Path,
    archive_root: Path,
    package_root: Path,
    role: str,
) -> tuple[dict[str, object], bytes]:
    plugin = plugin_path.expanduser().resolve()
    plugin_name = "vckss_rust_macos_arm64.plugin"
    if plugin.is_symlink() or not plugin.is_file() or plugin.name != plugin_name:
        raise QualificationError(f"{role} private arm64 plugin is unavailable: {plugin}")

    # The private Rust/plugin identity deliberately survives the public rename.
    # Bind each staged binary to the archived private implementation inventory;
    # the caller separately requires the two plugin binaries to be byte-identical.
    source_paths = [
        "rust/Cargo.lock",
        "rust/crates/vckss-core/src/lib.rs",
        "rust/crates/vckss-plugin/src/lib.rs",
        "rust/stata_backend/src/lib.rs",
    ]
    package_name = "vckss" if role == "baseline" else "fevc"
    source_paths.extend(
        f"{package_name}/{name}"
        for name in (
            "vckss.mata",
            "vckss_rng.mata",
            "vckss_graph.mata",
            "vckss_solver.mata",
            "vckss_resource.mata",
            "vckss_cmg.mata",
            "vckss_scale.mata",
            "vckss_scale_engine.mata",
            "vckss_scale_runtime.mata",
        )
    )
    lines: list[str] = []
    for relative in source_paths:
        source = archive_root / relative
        if source.is_symlink() or not source.is_file():
            raise QualificationError(f"{role} private source is unavailable: {relative}")
        lines.append(f"{_sha256_file(source)}  {relative}")
    lines.append(f"{_sha256_file(plugin)}  external/{plugin_name}")
    manifest_bytes = ("\n".join(lines) + "\n").encode("utf-8")

    staged = package_root / plugin_name
    shutil.copyfile(plugin, staged)
    staged.chmod(0o755)
    if _sha256_file(staged) != _sha256_file(plugin):
        raise QualificationError(f"{role} staged plugin hash mismatch")
    return (
        {
            "architecture": "arm64",
            "plugin_filename": plugin_name,
            "plugin_sha256": _sha256_file(plugin),
            "source_manifest_path": (
                EVIDENCE_DIR / f"{role}.plugin-source-manifest.sha256"
            ).as_posix(),
            "source_manifest_sha256": _sha256_bytes(manifest_bytes),
            "source_file_count": len(lines),
        },
        manifest_bytes,
    )


def _runtime_filenames(command: str) -> tuple[str, ...]:
    return (
        f"{command}.ado",
        "vckss_lifecycle.ado",
        "vckss.mata",
        "vckss_rng.mata",
        "vckss_graph.mata",
        "vckss_solver.mata",
        "vckss_resource.mata",
        "vckss_cmg.mata",
        "vckss_scale.mata",
        "vckss_scale_engine.mata",
        "vckss_scale_runtime.mata",
    )


def _preflight_package(package_root: Path, command: str) -> None:
    required = _runtime_filenames(command)
    missing = [name for name in required if not (package_root / name).is_file()]
    if missing:
        raise QualificationError(
            f"archived {command} package is missing runtime files: {missing}"
        )


def _git_path_exists(repo: Path, commit: str, path: str) -> bool:
    result = subprocess.run(
        ["git", "cat-file", "-e", f"{commit}:{path}"],
        cwd=repo,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        stdin=subprocess.DEVNULL,
    )
    return result.returncode == 0


def _preflight_committed_candidate(repo: Path, commit: str) -> None:
    required = [
        *(f"fevc/{name}" for name in _runtime_filenames("fevc")),
        "fevc/fevc.sthlp",
        "fevc/fevc.pkg",
        "fevc/stata.toc",
        "fevc/cmg/src/cmg_core.mata.in",
        "fevc/cmg/tools/assemble.py",
        "fevc/cmg/generated/manifest.json",
        "CODE_LICENSE.md",
        "pyproject.toml",
        ORCHESTRATOR_PATH.as_posix(),
        DRIVER_PATH.as_posix(),
    ]
    missing = [path for path in required if not _git_path_exists(repo, commit, path)]
    if missing:
        raise QualificationError(
            f"candidate commit is missing package/component source paths: {missing}"
        )


def _resolve_executable(candidate: str) -> Path | None:
    resolved = shutil.which(candidate) if os.sep not in candidate else candidate
    if resolved and Path(resolved).is_file() and os.access(resolved, os.X_OK):
        return Path(resolved).resolve()
    return None


def _find_stata(explicit: str | None) -> tuple[Path, str]:
    if explicit is not None:
        resolved = _resolve_executable(explicit)
        if resolved is None:
            raise QualificationError(f"explicit --stata is not executable: {explicit}")
        return resolved, "explicit"
    environment = os.environ.get("VCKSS_STATA")
    if environment is not None:
        resolved = _resolve_executable(environment)
        if resolved is None:
            raise QualificationError(
                f"VCKSS_STATA is not executable: {environment}"
            )
        return resolved, "environment"
    for candidate in ("stata-mp", "stata-se", "stata"):
        resolved = _resolve_executable(candidate)
        if resolved is not None:
            return resolved, "PATH"
    for candidate in (
        "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp",
        "/Applications/Stata/StataSE.app/Contents/MacOS/stata-se",
    ):
        resolved = _resolve_executable(candidate)
        if resolved is not None:
            return resolved, "platform_default"
    raise QualificationError("Stata was not found; pass --stata or set VCKSS_STATA")


def _parse_tsv(path: Path, expected_case: str | None = None) -> list[Record]:
    if not path.is_file():
        raise QualificationError(f"Stata did not create {path}")
    records: list[Record] = []
    seen: set[tuple[str, str, str, int, int]] = set()
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        if tuple(reader.fieldnames or ()) != TSV_COLUMNS:
            raise QualificationError(f"invalid TSV header in {path}")
        for line_number, row in enumerate(reader, start=2):
            if set(row) != set(TSV_COLUMNS) or any(row[column] is None for column in TSV_COLUMNS):
                raise QualificationError(f"malformed TSV row {line_number} in {path}")
            if expected_case is not None and row["case_id"] != expected_case:
                raise QualificationError(f"wrong case ID on TSV row {line_number} in {path}")
            try:
                record = Record(
                    case_id=row["case_id"],
                    kind=row["kind"],
                    name=row["name"],
                    row=int(row["row"]),
                    col=int(row["col"]),
                    value=row["value"],
                )
            except ValueError as exc:
                raise QualificationError(f"invalid TSV index on row {line_number} in {path}") from exc
            if record.row < 0 or record.col < 0 or "\n" in record.value or "\r" in record.value:
                raise QualificationError(f"noncanonical TSV row {line_number} in {path}")
            if record.key in seen:
                raise QualificationError(f"duplicate TSV key {record.key!r} in {path}")
            seen.add(record.key)
            records.append(record)
    if not records:
        raise QualificationError(f"empty TSV output for {expected_case or 'combined roles'}")
    if expected_case is None and {record.case_id for record in records} != set(REQUIRED_CASE_IDS):
        raise QualificationError(f"combined TSV has an incomplete case set: {path}")
    return sorted(records)


def _terminal_marker_ok(stdout: str, marker: str) -> bool:
    if stdout.count(marker) != 1:
        return False
    tail = stdout.split(marker, 1)[1]
    harmless = {"", ".", "end of do-file"}
    return all(line.strip().lower() in harmless for line in tail.splitlines())


def _run_stata_case(
    *,
    stata: Path,
    driver: Path,
    role: str,
    case_id: str,
    package_root: Path,
    command: str,
    prefix: str,
    run_root: Path,
    timeout_seconds: int,
) -> tuple[list[Record], str, list[dict[str, object]]]:
    case_root = run_root / role / case_id
    case_root.mkdir(parents=True, exist_ok=False)
    (case_root / "sysdirs" / "personal").mkdir(parents=True)
    (case_root / "sysdirs" / "plus").mkdir(parents=True)
    (case_root / "sysdirs" / "oldplace").mkdir(parents=True)
    (case_root / "tmp").mkdir()
    output = case_root / "raw.tsv"
    pass_path = case_root / "stata.pass"
    nonce = secrets.token_hex(24)
    marker = f"VCKSS_RENAME_EQ_V2_PASS::{nonce}::{role}::{case_id}"
    child_env = os.environ.copy()
    child_env.pop("KSS_PHASE_FILE", None)
    child_env["STATATMP"] = str(case_root / "tmp")
    child_env["TMPDIR"] = str(case_root / "tmp")
    result = _run(
        (
            str(stata),
            "-q",
            "do",
            str(driver),
            role,
            case_id,
            str(package_root),
            str(output),
            str(pass_path),
            nonce,
            command,
            prefix,
        ),
        cwd=case_root,
        timeout=timeout_seconds,
        check=False,
        env=child_env,
    )
    stdout_path = case_root / "stata.stdout.txt"
    stdout_path.write_text(result.stdout, encoding="utf-8")
    if result.returncode != 0:
        raise QualificationError(
            f"Stata process failed for {role}/{case_id} ({result.returncode}); see {stdout_path}"
        )
    if not _terminal_marker_ok(result.stdout, marker):
        raise QualificationError(
            f"Stata terminal marker missing or nonterminal for {role}/{case_id}; see {stdout_path}"
        )
    if not pass_path.is_file() or pass_path.read_text(encoding="utf-8").strip() != marker:
        raise QualificationError(f"Stata pass marker invalid for {role}/{case_id}")
    return (
        _parse_tsv(output, case_id),
        result.stdout,
        _snapshot_startup_profiles(result.stdout),
    )


def _record_value(records: Iterable[Record], case_id: str, kind: str, name: str) -> str:
    matches = [
        record.value
        for record in records
        if record.case_id == case_id and record.kind == kind and record.name == name
    ]
    if len(matches) != 1:
        raise QualificationError(
            f"expected exactly one {case_id}/{kind}/{name} record, found {len(matches)}"
        )
    return matches[0]


def _validate_role_records(role: str, records: Sequence[Record]) -> None:
    actual_cases = {record.case_id for record in records}
    if actual_cases != set(REQUIRED_CASE_IDS):
        raise QualificationError(f"{role} case set is incomplete: {sorted(actual_cases)}")
    for record in records:
        if record.kind == "scalar" and record.name in TIMING_SCALARS:
            raise QualificationError(f"timing leaked into canonical output: {record.name}")
        if record.kind == "timing":
            _validate_timing_record(record)
        if record.name == "V":
            raise QualificationError("e(V) must not be posted")
        if record.kind == "metadata" and record.name not in NORMALIZED_METADATA_NAMES:
            raise QualificationError(f"unregistered normalized metadata field: {record.name}")
    expected_command = "vckss" if role == "baseline" else "fevc"
    expected_prefix = "vckss"
    expected_version = "0.5.0-alpha.1"
    package_roots: set[str] = set()
    for case_id in REQUIRED_CASE_IDS:
        if _record_value(records, case_id, "metadata", "role") != role:
            raise QualificationError(f"wrong role metadata for {role}/{case_id}")
        if _record_value(records, case_id, "metadata", "command") != expected_command:
            raise QualificationError(f"wrong command metadata for {role}/{case_id}")
        if _record_value(records, case_id, "metadata", "private_prefix") != expected_prefix:
            raise QualificationError(f"wrong private prefix for {role}/{case_id}")
        package_root = _record_value(records, case_id, "metadata", "package_root")
        package_roots.add(package_root)
        resolved_ado = _record_value(records, case_id, "metadata", "resolved_ado")
        if resolved_ado != f"{package_root}/{expected_command}.ado":
            raise QualificationError(f"wrong resolved ado path for {role}/{case_id}")
        if _record_value(records, case_id, "metadata", "cmd") != expected_command:
            raise QualificationError(f"wrong observed e(cmd) for {role}/{case_id}")
        if _record_value(records, case_id, "metadata", "version") != expected_version:
            raise QualificationError(f"wrong observed e(version) for {role}/{case_id}")
        if _record_value(records, case_id, "assertion", "data_restored") != "PASS":
            raise QualificationError(f"data restoration failed for {role}/{case_id}")
        if _record_value(records, case_id, "assertion", "sort_restored") != "PASS":
            raise QualificationError(f"sort restoration failed for {role}/{case_id}")
        if _record_value(records, case_id, "assertion", "rng_restored") != "PASS":
            raise QualificationError(f"RNG restoration failed for {role}/{case_id}")
        if _record_value(records, case_id, "assertion", "no_e_V") != "PASS":
            raise QualificationError(f"e(V) was posted for {role}/{case_id}")
        rc_value = int(_record_value(records, case_id, "outcome", "rc"))
        if case_id in EXPECTED_FAILURES:
            expected_rc, expected_status = EXPECTED_FAILURES[case_id]
            if rc_value != expected_rc:
                raise QualificationError(f"wrong return code for {role}/{case_id}: {rc_value}")
            status = _record_value(records, case_id, "local", "withholding_status")
            if status != expected_status:
                raise QualificationError(f"wrong typed failure for {role}/{case_id}: {status}")
            if (
                _record_value(
                    records, case_id, "assertion", "sample_function_absent"
                )
                != "PASS"
            ):
                raise QualificationError(
                    f"e(sample) function unexpectedly posted for {role}/{case_id}"
                )
        else:
            if rc_value != 0:
                raise QualificationError(f"success case failed for {role}/{case_id}: {rc_value}")
            if _record_value(records, case_id, "assertion", "sample_valid") != "PASS":
                raise QualificationError(f"sample contract failed for {role}/{case_id}")
            if (
                _record_value(
                    records, case_id, "assertion", "sample_function_posted"
                )
                != "PASS"
            ):
                raise QualificationError(f"e(sample) missing for {role}/{case_id}")
            if not any(
                record.case_id == case_id and record.kind == "matrix" and record.name == "kss"
                for record in records
            ):
                raise QualificationError(f"e(kss) missing for {role}/{case_id}")
            cmdline_records = [
                record.value
                for record in records
                if record.case_id == case_id
                and record.kind == "metadata"
                and record.name == "cmdline"
            ]
            if len(cmdline_records) > 1:
                raise QualificationError(
                    f"duplicate observed e(cmdline) for {role}/{case_id}"
                )
            if cmdline_records:
                token, separator, _ = cmdline_records[0].partition(" ")
                if token != expected_command or not separator:
                    raise QualificationError(
                        f"wrong observed e(cmdline) token for {role}/{case_id}"
                    )
    if len(package_roots) != 1:
        raise QualificationError(f"package root drifted across {role} cases")
    route_expectations = {
        "compressed_match_fw_target": ("engine_selected", "compressed"),
        "cmg_forced_cheap_1200x300": ("preconditioner_selected", "CMG"),
        "cmg_auto_cheap_1200x300": ("preconditioner_selected", "CMG"),
        "auto_diagonal_small": ("preconditioner_selected", "DIAGONAL"),
    }
    for case_id, (name, expected) in route_expectations.items():
        actual = _record_value(records, case_id, "local", name)
        if actual != expected:
            raise QualificationError(f"wrong route for {role}/{case_id}: {name}={actual}")

    observed_timing_scalars = {
        record.name
        for record in records
        if record.kind == "timing" and record.row == 0 and record.col == 0
    }
    missing_timing_scalars = TIMING_SCALARS - observed_timing_scalars
    if missing_timing_scalars:
        raise QualificationError(
            f"registered timing scalars were never observed for {role}: "
            f"{sorted(missing_timing_scalars)}"
        )
    for name, columns in TIMING_MATRIX_COLUMNS.items():
        observed_columns = {
            record.col
            for record in records
            if record.kind == "timing" and record.name == name and record.row >= 1
        }
        if not columns.issubset(observed_columns):
            raise QualificationError(
                f"registered timing matrix columns were never observed for {role}/{name}: "
                f"{sorted(columns - observed_columns)}"
            )

    expected_cmg = {
        name: values[0 if role == "baseline" else 1]
        for name, values in CMG_API_TRANSITIONS.items()
    }
    for case_id in ("cmg_forced_cheap_1200x300", "cmg_auto_cheap_1200x300"):
        for name, expected in expected_cmg.items():
            actual = _record_value(records, case_id, "metadata", name)
            if actual != expected:
                raise QualificationError(f"wrong CMG identity for {role}/{case_id}: {name}")


def _runtime_environment(records_by_role: dict[str, list[Record]]) -> dict[str, str]:
    names = ("stata_version", "stata_flavor", "os", "processors", "processors_lic")
    environment: dict[str, str] = {}
    for name in names:
        observed = {
            _record_value(records, case_id, "metadata", name)
            for records in records_by_role.values()
            for case_id in REQUIRED_CASE_IDS
        }
        if len(observed) != 1:
            raise QualificationError(f"Stata runtime metadata drifted for {name}: {observed}")
        environment[name] = observed.pop()
    return environment


def _is_runtime_footprint(record: Record) -> bool:
    if record.kind == "scalar" and record.name in RUNTIME_FOOTPRINT_SCALARS:
        return True
    if record.kind == "matrix" and (
        record.name,
        record.row,
        record.col,
    ) in RUNTIME_FOOTPRINT_MATRIX_CELLS:
        return True
    return (
        record.case_id in {"cmg_forced_cheap_1200x300", "cmg_auto_cheap_1200x300"}
        and (record.kind, record.name, record.row, record.col) in CMG_FOOTPRINT_KEYS
    )


_STATA_HEX_RE = re.compile(r"^[+-][0-9a-f]\.[0-9a-f]{13}X[+-][0-9a-f]{3}$", re.I)
_STATA_SYSTEM_MISSING_HEX = "+1.0000000000000X+3ff"


def _stata_hex_is_missing(value: str) -> bool:
    return value == "." or value.lower().endswith("x+3ff")


def _stata_is_system_missing(value: str) -> bool:
    return value == "." or value.lower() == _STATA_SYSTEM_MISSING_HEX.lower()


def _validate_timing_record(record: Record) -> None:
    scalar_timing = (
        record.row == 0 and record.col == 0 and record.name in TIMING_SCALARS
    )
    matrix_timing = (
        record.row >= 1
        and record.name in TIMING_MATRIX_COLUMNS
        and record.col in TIMING_MATRIX_COLUMNS[record.name]
    )
    if not scalar_timing and not matrix_timing:
        raise QualificationError(f"unregistered timing record: {record.key!r}")
    if _stata_hex_is_missing(record.value):
        if scalar_timing and record.name != "rng_seconds":
            raise QualificationError(f"unexpected missing timing value: {record.key!r}")
        return
    if (
        _STATA_HEX_RE.fullmatch(record.value) is None
        or record.value.startswith("-")
        or _stata_hex_is_missing(record.value)
    ):
        raise QualificationError(
            f"noncanonical finite nonnegative timing at {record.key!r}: {record.value!r}"
        )


def _validate_footprint_pairs(
    baseline: Sequence[Record], candidate: Sequence[Record]
) -> set[tuple[str, str, str, int, int]]:
    left = {record.key: record.value for record in baseline if _is_runtime_footprint(record)}
    right = {record.key: record.value for record in candidate if _is_runtime_footprint(record)}
    if left.keys() != right.keys():
        raise QualificationError("runtime-footprint record shape differs between revisions")
    for key in left:
        old, new = left[key], right[key]
        for value in (old, new):
            if _stata_is_system_missing(value):
                if key[1] != "scalar" or key[2] not in RUNTIME_OPTIONAL_MISSING_SCALARS:
                    raise QualificationError(
                        f"unexpected missing runtime footprint at {key}: {value!r}"
                    )
                continue
            if (
                _STATA_HEX_RE.fullmatch(value) is None
                or value.startswith("-")
                or value.lower().endswith("x+3ff")
            ):
                raise QualificationError(f"noncanonical runtime footprint at {key}: {value!r}")
        if _stata_is_system_missing(old) or _stata_is_system_missing(new):
            if _stata_is_system_missing(old) != _stata_is_system_missing(new):
                raise QualificationError(
                    f"runtime footprint applicability differs at {key}: {old!r} != {new!r}"
                )

    scalar_names = {key[2] for key in left if key[1] == "scalar"}
    unused_scalars = RUNTIME_FOOTPRINT_SCALARS - scalar_names
    if unused_scalars:
        raise QualificationError(
            f"registered runtime-footprint scalars were never observed: {sorted(unused_scalars)}"
        )
    base_cells = {(key[2], key[3], key[4]) for key in left if key[1] == "matrix"}
    unused_cells = RUNTIME_FOOTPRINT_MATRIX_CELLS - base_cells
    if unused_cells:
        raise QualificationError(
            f"registered runtime-footprint matrix cells were never observed: {sorted(unused_cells)}"
        )
    for case_id in ("cmg_forced_cheap_1200x300", "cmg_auto_cheap_1200x300"):
        observed = {
            (key[1], key[2], key[3], key[4]) for key in left if key[0] == case_id
        }
        missing = CMG_FOOTPRINT_KEYS - observed
        if missing:
            raise QualificationError(
                f"registered CMG footprint keys missing from {case_id}: {sorted(missing)}"
            )
    return set(left)


def _comparison_records(
    records: Sequence[Record], role: str
) -> tuple[
    dict[tuple[str, str, str, int, int], str],
    set[tuple[str, str, str, int, int]],
    set[tuple[str, str, str, int, int]],
]:
    expected_command = "vckss" if role == "baseline" else "fevc"
    compared: dict[tuple[str, str, str, int, int], str] = {}
    omitted: set[tuple[str, str, str, int, int]] = set()
    timing: set[tuple[str, str, str, int, int]] = set()
    for record in records:
        if _is_runtime_footprint(record):
            omitted.add(record.key)
            continue
        value = record.value
        if record.kind == "timing":
            _validate_timing_record(record)
            timing.add(record.key)
            value = (
                "REGISTERED_TIMING_NOT_APPLICABLE"
                if _stata_hex_is_missing(value)
                else "REGISTERED_TIMING_VALUE"
            )
        elif record.kind == "metadata":
            if record.name == "role":
                value = "REVISION_ROLE"
            elif record.name in {"command", "cmd"}:
                value = "PUBLIC_COMMAND"
            elif record.name == "private_prefix":
                value = "PRIVATE_PREFIX"
            elif record.name == "package_root":
                value = "PACKAGE_ROOT"
            elif record.name == "resolved_ado":
                value = "PACKAGE_ROOT/PUBLIC_COMMAND.ado"
            elif record.name == "version":
                value = "PACKAGE_VERSION"
            elif record.name == "cmdline":
                token, separator, remainder = value.partition(" ")
                if token != expected_command or not separator:
                    raise QualificationError(
                        f"cannot normalize e(cmdline) for {role}/{record.case_id}"
                    )
                value = f"PUBLIC_COMMAND {remainder}"
            elif record.name == "cmg_api_level":
                value = "CMG_API_LEVEL"
            elif record.name == "cmg_design_label":
                value = "CMG_DESIGN_LABEL"
        elif (
            record.case_id == "failure_invalid_memory"
            and record.kind == "local"
            and record.name == "withholding_suggestion"
        ):
            if value != INVALID_MEMORY_HELP_TRANSITION[role]:
                raise QualificationError(
                    f"unexpected invalid-memory help suggestion for {role}"
                )
            value = "INVALID_MEMORY_HELP_SUGGESTION"
        compared[record.key] = value
    return compared, omitted, timing


def _compare_records(
    baseline: Sequence[Record], candidate: Sequence[Record]
) -> dict[str, dict[str, object]]:
    footprint_keys = _validate_footprint_pairs(baseline, candidate)
    left, left_omitted, left_timing = _comparison_records(baseline, "baseline")
    right, right_omitted, right_timing = _comparison_records(candidate, "candidate")
    if left_omitted != right_omitted:
        raise QualificationError("runtime-footprint record shape differs between revisions")
    if left_timing != right_timing:
        raise QualificationError("registered timing record shape differs between revisions")
    if left.keys() != right.keys():
        missing = sorted(left.keys() - right.keys())[:20]
        extra = sorted(right.keys() - left.keys())[:20]
        raise QualificationError(f"scientific record shape mismatch; missing={missing}, extra={extra}")
    mismatches = [(key, left[key], right[key]) for key in left if left[key] != right[key]]
    if mismatches:
        detail = "\n".join(f"{key}: {old!r} != {new!r}" for key, old, new in mismatches[:30])
        raise QualificationError(f"scientific equivalence failed ({len(mismatches)} fields):\n{detail}")
    results: dict[str, dict[str, object]] = {}
    for case_id in REQUIRED_CASE_IDS:
        count = sum(key[0] == case_id for key in left)
        results[case_id] = {
            "status": "PASS",
            "comparison": COMPARISON_MODE,
            "records_compared": count,
            "timing_records_normalized": sum(
                key[0] == case_id for key in left_timing
            ),
            "timing_keys": [
                {"name": key[2], "row": key[3], "col": key[4]}
                for key in sorted(left_timing)
                if key[0] == case_id
            ],
            "timing_invariant": (
                "same shape and applicability; applicable values are canonical, "
                "nonnegative, and finite; raw values retained"
            ),
            "runtime_footprint_records_normalized": sum(
                key[0] == case_id for key in footprint_keys
            ),
            "runtime_footprint_keys": [
                {"kind": key[1], "name": key[2], "row": key[3], "col": key[4]}
                for key in sorted(footprint_keys)
                if key[0] == case_id
            ],
            "runtime_footprint_invariant": (
                "same shape and applicability; every applicable raw value is "
                "canonical, finite, and nonnegative; raw values retained"
            ),
        }
    return results


def _write_role_artifacts(
    output_dir: Path,
    role: str,
    revision: dict[str, str],
    records: Sequence[Record],
) -> tuple[dict[str, dict[str, str]], dict[str, bytes]]:
    tsv_path = output_dir / f"{role}.raw.tsv"
    buffer = io.StringIO(newline="")
    writer = csv.writer(buffer, delimiter="\t", lineterminator="\n")
    writer.writerow(TSV_COLUMNS)
    for record in sorted(records):
        writer.writerow(
            (record.case_id, record.kind, record.name, record.row, record.col, record.value)
        )
    tsv_bytes = buffer.getvalue().encode("utf-8")
    _write_atomic(tsv_path, tsv_bytes)
    json_path = output_dir / f"{role}.raw.json"
    payload = {
        "schema": "fevc-rename-equivalence-raw-v1",
        "role": role,
        "revision": revision,
        "required_case_ids": list(REQUIRED_CASE_IDS),
        "records": [record.as_dict() for record in sorted(records)],
    }
    json_bytes = _json_bytes(payload)
    _write_atomic(json_path, json_bytes)
    evidence_paths = {
        "tsv": EVIDENCE_DIR / f"{role}.raw.tsv",
        "json": EVIDENCE_DIR / f"{role}.raw.json",
    }
    artifacts = {
        kind: {"path": path.as_posix(), "sha256": _sha256_bytes(data)}
        for kind, path, data in (
            ("tsv", evidence_paths["tsv"], tsv_bytes),
            ("json", evidence_paths["json"], json_bytes),
        )
    }
    return artifacts, {"tsv": tsv_bytes, "json": json_bytes}


def _repo_regular_file(repo: Path, relative: str) -> Path:
    pure = PurePosixPath(relative)
    if pure.is_absolute() or not pure.parts or any(part in {"", ".", ".."} for part in pure.parts):
        raise QualificationError(f"unsafe repository-relative evidence path: {relative!r}")
    lexical = repo.joinpath(*pure.parts)
    if lexical.is_symlink() or not lexical.is_file():
        raise QualificationError(f"evidence must be a regular non-symlink file: {relative}")
    resolved = lexical.resolve()
    try:
        resolved.relative_to(repo.resolve())
    except ValueError as exc:
        raise QualificationError(f"evidence path escapes repository: {relative}") from exc
    return resolved


def _verify_hash(path: Path, expected: str, label: str) -> None:
    if _sha256_file(path) != expected:
        raise QualificationError(f"{label} hash mismatch")


def _validate_artifact_json(
    path: Path, role: str, revision: dict[str, str], tsv_records: Sequence[Record]
) -> None:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise QualificationError(f"invalid raw JSON artifact: {path}") from exc
    if not isinstance(payload, dict) or payload.get("schema") != "fevc-rename-equivalence-raw-v1":
        raise QualificationError(f"wrong raw JSON schema: {path}")
    if payload.get("role") != role or payload.get("revision") != revision:
        raise QualificationError(f"raw JSON revision binding failed: {path}")
    if payload.get("required_case_ids") != list(REQUIRED_CASE_IDS):
        raise QualificationError(f"raw JSON case binding failed: {path}")
    expected = [record.as_dict() for record in sorted(tsv_records)]
    if payload.get("records") != expected:
        raise QualificationError(f"raw JSON and TSV records differ: {path}")


def _comparison_descriptor() -> dict[str, object]:
    return {
        "status": "PASS",
        "mode": COMPARISON_MODE,
        "registered_metadata_names": sorted(NORMALIZED_METADATA_NAMES),
        "identity_metadata_normalized": [
            "role",
            "command",
            "private_prefix",
            "package_root",
            "resolved_ado",
            "cmd",
            "cmdline-leading-command",
            "version",
            "cmg_api_level",
            "cmg_design_label",
        ],
        "environment_metadata_compared_exact": [
            "stata_version",
            "stata_flavor",
            "os",
            "processors",
            "processors_lic",
        ],
        "timing_scalar_names": sorted(TIMING_SCALARS),
        "timing_matrix_columns": [
            {"name": name, "columns": sorted(columns)}
            for name, columns in sorted(TIMING_MATRIX_COLUMNS.items())
        ],
        "timing_invariant": (
            "same return shape and applicability; applicable values are canonical, "
            "nonnegative, and finite"
        ),
        "runtime_footprint_scalar_names": sorted(RUNTIME_FOOTPRINT_SCALARS),
        "runtime_footprint_matrix_cells": [
            {"name": name, "row": row, "col": col}
            for name, row, col in sorted(RUNTIME_FOOTPRINT_MATRIX_CELLS)
        ],
        "cmg_conditional_runtime_footprint_keys": [
            {"kind": kind, "name": name, "row": row, "col": col}
            for kind, name, row, col in sorted(CMG_FOOTPRINT_KEYS)
        ],
        "raw_values_retained": True,
        "cmg_identity_transitions": {
            name: {"baseline": values[0], "candidate": values[1]}
            for name, values in CMG_API_TRANSITIONS.items()
        },
        "typed_failure_identity_transitions": {
            "failure_invalid_memory/withholding_suggestion": dict(
                INVALID_MEMORY_HELP_TRANSITION
            )
        },
    }


def _require_head_bound_file(
    repo: Path, relative: str, expected_sha256: str, label: str
) -> None:
    if not _git_path_exists(repo, "HEAD", relative):
        raise QualificationError(f"{label} is not tracked in HEAD: {relative}")
    if _git_blob_sha256(repo, "HEAD", relative) != expected_sha256:
        raise QualificationError(f"{label} HEAD blob hash mismatch: {relative}")
    current = _repo_regular_file(repo, relative)
    _verify_hash(current, expected_sha256, f"{label} worktree file")


def _validate_external_environment(environment: dict[str, object]) -> None:
    executable = environment["stata_executable"]
    assert isinstance(executable, dict)
    executable_path = Path(str(executable["path"]))
    if not executable_path.is_file() or executable_path.is_symlink():
        raise QualificationError("recorded Stata executable is unavailable or not regular")
    _verify_hash(
        executable_path,
        str(executable["sha256"]),
        "recorded Stata executable",
    )
    profiles = environment["startup_profiles"]
    assert isinstance(profiles, list)
    for profile in profiles:
        assert isinstance(profile, dict)
        profile_path = Path(str(profile["path"])).expanduser()
        if profile["readable"] is True:
            if not profile_path.is_file() or profile_path.is_symlink():
                raise QualificationError(
                    f"recorded startup profile is unavailable or not regular: {profile_path}"
                )
            _verify_hash(
                profile_path,
                str(profile["sha256"]),
                f"recorded startup profile {profile_path}",
            )
        elif profile_path.exists():
            raise QualificationError(
                f"previously unreadable startup profile boundary changed: {profile_path}"
            )


def _validate_receipt(
    payload: object,
    repo: Path | None = None,
    *,
    require_tracked: bool | None = None,
) -> None:
    if require_tracked is None:
        require_tracked = repo is not None
    if require_tracked and repo is None:
        raise QualificationError("tracked receipt validation requires a repository")
    if not isinstance(payload, dict):
        raise QualificationError("receipt must be a JSON object")
    expected_top_level = {
        "schema",
        "status",
        "generated_at_utc",
        "baseline",
        "candidate",
        "harness",
        "required_case_ids",
        "cases",
        "environment",
        "comparison",
        "rust_artifacts",
        "raw_artifacts",
    }
    if set(payload) != expected_top_level:
        raise QualificationError("receipt top-level fields are not exact")
    if payload.get("schema") != RECEIPT_SCHEMA or payload.get("status") != "PASS":
        raise QualificationError("receipt schema/status is not a qualifying PASS")
    generated_at = payload.get("generated_at_utc")
    if not isinstance(generated_at, str) or not generated_at.endswith("Z"):
        raise QualificationError("receipt generation timestamp is invalid")
    try:
        datetime.fromisoformat(generated_at.removesuffix("Z") + "+00:00")
    except ValueError as exc:
        raise QualificationError("receipt generation timestamp is invalid") from exc
    baseline = payload.get("baseline")
    candidate = payload.get("candidate")
    if not isinstance(baseline, dict) or baseline.get("commit") != BASELINE_COMMIT:
        raise QualificationError("receipt has the wrong immutable predecessor")
    if not isinstance(candidate, dict):
        raise QualificationError("receipt candidate object is missing")
    for object_name, revision in (("baseline", baseline), ("candidate", candidate)):
        if set(revision) != {"commit", "tree"}:
            raise QualificationError(f"receipt {object_name} revision fields are invalid")
        for field in ("commit", "tree"):
            value = revision.get(field)
            if not isinstance(value, str):
                raise QualificationError(f"receipt {object_name}.{field} is missing")
            _require_full_sha(value, f"receipt {object_name}.{field}")
    if payload.get("required_case_ids") != list(REQUIRED_CASE_IDS):
        raise QualificationError("receipt required case list is not exact and ordered")
    cases = payload.get("cases")
    if not isinstance(cases, dict) or set(cases) != set(REQUIRED_CASE_IDS):
        raise QualificationError("receipt case results are incomplete")
    for case_id in REQUIRED_CASE_IDS:
        result = cases[case_id]
        if not isinstance(result, dict) or result.get("status") != "PASS":
            raise QualificationError(f"receipt case did not PASS: {case_id}")
    harness = payload.get("harness")
    if not isinstance(harness, dict):
        raise QualificationError("receipt harness object is missing")
    expected_paths = {
        "orchestrator": ORCHESTRATOR_PATH.as_posix(),
        "driver": DRIVER_PATH.as_posix(),
    }
    for name, expected_path in expected_paths.items():
        item = harness.get(name)
        if not isinstance(item, dict) or item.get("path") != expected_path:
            raise QualificationError(f"receipt harness.{name}.path is invalid")
        digest = item.get("sha256")
        if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
            raise QualificationError(f"receipt harness.{name}.sha256 is invalid")
    if payload.get("comparison") != _comparison_descriptor():
        raise QualificationError("receipt comparison descriptor is not exact")
    rust_artifacts = payload.get("rust_artifacts")
    if not isinstance(rust_artifacts, dict) or set(rust_artifacts) != {
        "baseline", "candidate"
    }:
        raise QualificationError("receipt Rust artifact inventory is incomplete")
    for role in ("baseline", "candidate"):
        item = rust_artifacts[role]
        command = "vckss" if role == "baseline" else "fevc"
        expected_manifest = (
            EVIDENCE_DIR / f"{role}.plugin-source-manifest.sha256"
        ).as_posix()
        if (
            not isinstance(item, dict)
            or set(item) != {
                "architecture",
                "plugin_filename",
                "plugin_sha256",
                "source_manifest_path",
                "source_manifest_sha256",
                "source_file_count",
            }
            or item.get("architecture") != "arm64"
            or item.get("plugin_filename") != "vckss_rust_macos_arm64.plugin"
            or item.get("source_manifest_path") != expected_manifest
            or re.fullmatch(r"[0-9a-f]{64}", str(item.get("plugin_sha256", "")))
            is None
            or re.fullmatch(
                r"[0-9a-f]{64}", str(item.get("source_manifest_sha256", ""))
            )
            is None
            or not isinstance(item.get("source_file_count"), int)
            or item["source_file_count"] <= 0
        ):
            raise QualificationError(f"receipt Rust artifact item is invalid for {role}")
    environment = payload.get("environment")
    if not isinstance(environment, dict):
        raise QualificationError("receipt environment object is missing")
    executable = environment.get("stata_executable")
    runtime = environment.get("runtime")
    profiles = environment.get("startup_profiles")
    boundary = environment.get("startup_boundary")
    if (
        not isinstance(executable, dict)
        or set(executable) != {"path", "sha256", "discovery_source"}
        or not isinstance(executable.get("path"), str)
        or not Path(executable["path"]).is_absolute()
        or executable.get("discovery_source")
        not in {"explicit", "environment", "PATH", "platform_default"}
        or re.fullmatch(r"[0-9a-f]{64}", str(executable.get("sha256", ""))) is None
    ):
        raise QualificationError("receipt Stata executable inventory is invalid")
    if not isinstance(runtime, dict) or set(runtime) != {
        "stata_version", "stata_flavor", "os", "processors", "processors_lic"
    }:
        raise QualificationError("receipt Stata runtime inventory is invalid")
    if any(not isinstance(value, str) or not value for value in runtime.values()):
        raise QualificationError("receipt Stata runtime values are invalid")
    if not isinstance(profiles, list):
        raise QualificationError("receipt startup profile inventory is invalid")
    seen_profile_paths: set[str] = set()
    for profile in profiles:
        if not isinstance(profile, dict) or not isinstance(profile.get("path"), str):
            raise QualificationError("receipt startup profile entry is invalid")
        profile_path = profile["path"]
        if not Path(profile_path).expanduser().is_absolute():
            raise QualificationError("receipt startup profile path is not absolute")
        if profile_path in seen_profile_paths:
            raise QualificationError("receipt startup profile inventory has duplicates")
        seen_profile_paths.add(profile_path)
        if profile.get("readable") is True:
            if set(profile) != {"path", "readable", "sha256"} or re.fullmatch(
                r"[0-9a-f]{64}", str(profile.get("sha256", ""))
            ) is None:
                raise QualificationError("receipt readable startup profile is invalid")
        elif profile.get("readable") is False:
            if set(profile) != {"path", "readable", "caveat"} or not isinstance(
                profile.get("caveat"), str
            ):
                raise QualificationError("receipt unreadable startup profile is invalid")
        else:
            raise QualificationError("receipt startup profile readability is invalid")
    required_boundary = {
        "profile_runs_before_driver": True,
        "discard": True,
        "mata_clear": True,
        "isolated_sysdirs": True,
        "exact_main_ado_path": True,
        "stdin_devnull": True,
        "KSS_PHASE_FILE": "removed",
        "STATATMP": "case-local",
        "TMPDIR": "case-local",
    }
    if boundary != required_boundary:
        raise QualificationError("receipt startup/isolation boundary is invalid")
    artifacts = payload.get("raw_artifacts")
    if not isinstance(artifacts, dict) or set(artifacts) != {"baseline", "candidate"}:
        raise QualificationError("receipt raw artifact inventory is incomplete")
    for role in ("baseline", "candidate"):
        if not isinstance(artifacts[role], dict) or set(artifacts[role]) != {"tsv", "json"}:
            raise QualificationError(f"receipt raw artifact inventory is incomplete for {role}")
        for kind in ("tsv", "json"):
            item = artifacts[role][kind]
            expected_artifact_path = (EVIDENCE_DIR / f"{role}.raw.{kind}").as_posix()
            if (
                not isinstance(item, dict)
                or item.get("path") != expected_artifact_path
            ):
                raise QualificationError(f"receipt raw {role}/{kind} item is invalid")
            if re.fullmatch(r"[0-9a-f]{64}", str(item.get("sha256", ""))) is None:
                raise QualificationError(f"receipt raw {role}/{kind} hash is invalid")

    if repo is None:
        return
    _validate_external_environment(environment)
    revisions = {"baseline": baseline, "candidate": candidate}
    for role, revision in revisions.items():
        commit = _git(repo, "rev-parse", "--verify", f"{revision['commit']}^{{commit}}")
        tree = _git(repo, "rev-parse", "--verify", f"{commit}^{{tree}}")
        if commit != revision["commit"] or tree != revision["tree"]:
            raise QualificationError(f"receipt {role} Git object binding failed")
        manifest_item = rust_artifacts[role]
        manifest_path = _repo_regular_file(repo, manifest_item["source_manifest_path"])
        _verify_hash(
            manifest_path,
            manifest_item["source_manifest_sha256"],
            f"receipt Rust source manifest {role}",
        )
        manifest_lines = manifest_path.read_text(encoding="utf-8").splitlines()
        if len(manifest_lines) != manifest_item["source_file_count"]:
            raise QualificationError(
                f"receipt Rust source manifest count differs for {role}"
            )
        if require_tracked:
            _require_head_bound_file(
                repo,
                manifest_item["source_manifest_path"],
                manifest_item["source_manifest_sha256"],
                f"receipt Rust source manifest {role}",
            )
    _preflight_committed_candidate(repo, candidate["commit"])
    head = _git(repo, "rev-parse", "--verify", "HEAD^{commit}")
    if not _git_is_ancestor(repo, candidate["commit"], head):
        raise QualificationError("receipt candidate commit is not an ancestor of HEAD")
    for name, expected_path in expected_paths.items():
        committed_digest = _git_blob_sha256(repo, candidate["commit"], expected_path)
        if committed_digest != harness[name]["sha256"]:
            raise QualificationError(
                f"receipt harness {name} does not match candidate commit"
            )
        current = _repo_regular_file(repo, expected_path)
        _verify_hash(current, harness[name]["sha256"], f"receipt harness {name}")
    if require_tracked:
        _require_head_bound_file(
            repo,
            RECEIPT_PATH.as_posix(),
            _sha256_bytes(_json_bytes(payload)),
            "canonical receipt",
        )
    parsed: dict[str, list[Record]] = {}
    for role in ("baseline", "candidate"):
        tsv_item = artifacts[role]["tsv"]
        json_item = artifacts[role]["json"]
        tsv_path = _repo_regular_file(repo, tsv_item["path"])
        json_path = _repo_regular_file(repo, json_item["path"])
        _verify_hash(tsv_path, tsv_item["sha256"], f"receipt raw {role}/tsv")
        _verify_hash(json_path, json_item["sha256"], f"receipt raw {role}/json")
        if require_tracked:
            _require_head_bound_file(
                repo,
                tsv_item["path"],
                tsv_item["sha256"],
                f"receipt raw {role}/tsv",
            )
            _require_head_bound_file(
                repo,
                json_item["path"],
                json_item["sha256"],
                f"receipt raw {role}/json",
            )
        parsed[role] = _parse_tsv(tsv_path)
        _validate_artifact_json(json_path, role, revisions[role], parsed[role])
        _validate_role_records(role, parsed[role])
    if _runtime_environment(parsed) != runtime:
        raise QualificationError("receipt runtime metadata does not match raw evidence")
    regenerated_cases = _compare_records(parsed["baseline"], parsed["candidate"])
    if regenerated_cases != cases:
        raise QualificationError("receipt case accounting does not match raw evidence")


def _snapshot_startup_profiles(stdout: str) -> list[dict[str, object]]:
    inventory: list[dict[str, object]] = []
    for raw_path in re.findall(r"(?m)^Running (.+?) \.\.\.\s*$", stdout):
        path = Path(raw_path).expanduser()
        item: dict[str, object] = {"path": raw_path}
        if path.is_file():
            item.update({"readable": True, "sha256": _sha256_file(path.resolve())})
        else:
            item.update({"readable": False, "caveat": "startup path not readable"})
        inventory.append(item)
    return inventory


def _profile_inventory(
    observations: Sequence[list[dict[str, object]]],
) -> list[dict[str, object]]:
    if not observations or any(item != observations[0] for item in observations):
        raise QualificationError("Stata startup profile inventory differed across processes")
    return observations[0]


def _publish_evidence_bundle(
    repo: Path,
    raw_bytes: dict[str, dict[str, bytes]],
    rust_manifest_bytes: dict[str, bytes],
    receipt: dict[str, object],
) -> Path:
    destination = repo / EVIDENCE_DIR
    if destination.exists() or destination.is_symlink():
        raise QualificationError(f"refusing to overwrite evidence directory: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = destination.parent / f".{destination.name}.{secrets.token_hex(8)}.tmp"
    staging.mkdir()
    try:
        for role in ("baseline", "candidate"):
            for kind in ("tsv", "json"):
                _write_atomic_new(staging / f"{role}.raw.{kind}", raw_bytes[role][kind])
            _write_atomic_new(
                staging / f"{role}.plugin-source-manifest.sha256",
                rust_manifest_bytes[role],
            )
        _write_atomic_new(staging / "receipt.json", _json_bytes(receipt))
        os.rename(staging, destination)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return destination / "receipt.json"


def _receipt_payload(
    *,
    baseline: dict[str, str],
    candidate: dict[str, str],
    harness: dict[str, dict[str, str]],
    cases: dict[str, dict[str, object]],
    artifacts: dict[str, dict[str, dict[str, str]]],
    environment: dict[str, object],
    rust_artifacts: dict[str, dict[str, object]],
) -> dict[str, object]:
    payload: dict[str, object] = {
        "schema": RECEIPT_SCHEMA,
        "status": "PASS",
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "baseline": baseline,
        "candidate": candidate,
        "harness": harness,
        "required_case_ids": list(REQUIRED_CASE_IDS),
        "cases": cases,
        "environment": environment,
        "comparison": _comparison_descriptor(),
        "rust_artifacts": rust_artifacts,
        "raw_artifacts": artifacts,
    }
    _validate_receipt(payload)
    return payload


def _self_test() -> None:
    zero = "+0.0000000000000X-3ff"
    one = "+1.0000000000000X+000"

    def role_fixture(role: str) -> list[Record]:
        command = "vckss" if role == "baseline" else "fevc"
        prefix = "vckss"
        version = "0.5.0-alpha.1"
        package_root = f"/archive/{command}"
        records: list[Record] = []
        for case_id in REQUIRED_CASE_IDS:
            metadata = {
                "role": role,
                "command": command,
                "private_prefix": prefix,
                "package_root": package_root,
                "resolved_ado": f"{package_root}/{command}.ado",
                "cmd": command,
                "version": version,
                "stata_version": "19",
                "stata_flavor": "MP",
                "os": "MacOSX",
                "processors": "8",
                "processors_lic": "8",
            }
            if case_id in SUCCESS_CASES:
                metadata["cmdline"] = f"{command} y, nodisplay"
            if case_id in {"cmg_forced_cheap_1200x300", "cmg_auto_cheap_1200x300"}:
                metadata.update(
                    {
                        "cmg_api_level": CMG_API_TRANSITIONS["cmg_api_level"][
                            0 if role == "baseline" else 1
                        ],
                        "cmg_design_label": CMG_API_TRANSITIONS["cmg_design_label"][
                            0 if role == "baseline" else 1
                        ],
                    }
                )
            records.extend(
                Record(case_id, "metadata", name, 0, 0, value)
                for name, value in metadata.items()
            )
            records.extend(
                Record(case_id, "assertion", name, 0, 0, "PASS")
                for name in ("data_restored", "sort_restored", "rng_restored", "no_e_V")
            )
            if case_id in EXPECTED_FAILURES:
                rc, status = EXPECTED_FAILURES[case_id]
                records.append(Record(case_id, "outcome", "rc", 0, 0, str(rc)))
                records.append(
                    Record(case_id, "local", "withholding_status", 0, 0, status)
                )
                records.append(
                    Record(
                        case_id,
                        "assertion",
                        "sample_function_absent",
                        0,
                        0,
                        "PASS",
                    )
                )
            else:
                records.extend(
                    (
                        Record(case_id, "outcome", "rc", 0, 0, "0"),
                        Record(case_id, "assertion", "sample_valid", 0, 0, "PASS"),
                        Record(
                            case_id,
                            "assertion",
                            "sample_function_posted",
                            0,
                            0,
                            "PASS",
                        ),
                        Record(case_id, "timing", "fit_seconds", 0, 0, one),
                        Record(case_id, "matrix", "kss", 1, 1, one),
                    )
                )
        routes = {
            "compressed_match_fw_target": ("engine_selected", "compressed"),
            "cmg_forced_cheap_1200x300": ("preconditioner_selected", "CMG"),
            "cmg_auto_cheap_1200x300": ("preconditioner_selected", "CMG"),
            "auto_diagonal_small": ("preconditioner_selected", "DIAGONAL"),
        }
        records.extend(
            Record(case_id, "local", name, 0, 0, value)
            for case_id, (name, value) in routes.items()
        )
        records.extend(
            Record("exact_match_joint", "timing", name, 0, 0, one)
            for name in sorted(TIMING_SCALARS - {"fit_seconds"})
        )
        for name, columns in TIMING_MATRIX_COLUMNS.items():
            records.extend(
                Record("exact_match_joint", "timing", name, 1, col, one)
                for col in sorted(columns)
            )
        return sorted(records)

    baseline_role = role_fixture("baseline")
    candidate_role = role_fixture("candidate")
    _validate_role_records("baseline", baseline_role)
    _validate_role_records("candidate", candidate_role)
    bad_identity = list(candidate_role)
    bad_identity[bad_identity.index(
        next(
            record
            for record in bad_identity
            if record.case_id == "exact_match_joint"
            and record.kind == "metadata"
            and record.name == "version"
        )
    )] = Record("exact_match_joint", "metadata", "version", 0, 0, "0.5.0-alpha.2")
    try:
        _validate_role_records("candidate", bad_identity)
    except QualificationError:
        pass
    else:
        raise AssertionError("identity metadata self-test did not fail closed")

    baseline = [
        Record("exact_match_joint", "metadata", "command", 0, 0, "vckss"),
        Record("exact_match_joint", "metadata", "cmdline", 0, 0, "vckss y, nodisplay"),
        Record("exact_match_joint", "timing", "fit_seconds", 0, 0, one),
        Record("exact_match_joint", "matrix", "kss", 1, 1, one),
    ]
    candidate = [
        Record("exact_match_joint", "metadata", "command", 0, 0, "fevc"),
        Record(
            "exact_match_joint", "metadata", "cmdline", 0, 0, "fevc y, nodisplay"
        ),
        Record("exact_match_joint", "timing", "fit_seconds", 0, 0, zero),
        Record("exact_match_joint", "matrix", "kss", 1, 1, one),
    ]
    for name in sorted(RUNTIME_FOOTPRINT_SCALARS):
        baseline.append(Record("exact_match_joint", "scalar", name, 0, 0, one))
        candidate.append(Record("exact_match_joint", "scalar", name, 0, 0, zero))
    for name, row, col in sorted(RUNTIME_FOOTPRINT_MATRIX_CELLS):
        baseline.append(Record("exact_match_joint", "matrix", name, row, col, one))
        candidate.append(Record("exact_match_joint", "matrix", name, row, col, zero))
    for case_id in ("cmg_forced_cheap_1200x300", "cmg_auto_cheap_1200x300"):
        for kind, name, row, col in sorted(CMG_FOOTPRINT_KEYS):
            baseline.append(Record(case_id, kind, name, row, col, one))
            candidate.append(Record(case_id, kind, name, row, col, zero))
    compared = _compare_records(baseline, candidate)
    if compared["exact_match_joint"]["status"] != "PASS":
        raise AssertionError("comparison self-test did not pass")
    if compared["exact_match_joint"]["timing_records_normalized"] != 1:
        raise AssertionError("timing normalization self-test did not pass")
    broken = [
        Record(
            record.case_id,
            record.kind,
            record.name,
            record.row,
            record.col,
            "+1.0000000000001X+000",
        )
        if record.kind == "matrix" and record.name == "kss"
        else record
        for record in candidate
    ]
    try:
        _compare_records(baseline, broken)
    except QualificationError:
        pass
    else:
        raise AssertionError("comparison self-test did not fail closed")
    bad_cmdline = list(candidate)
    bad_cmdline[1] = Record(
        "exact_match_joint", "metadata", "cmdline", 0, 0, "vckss z, nodisplay"
    )
    try:
        _compare_records(baseline, bad_cmdline)
    except QualificationError:
        pass
    else:
        raise AssertionError("cmdline normalization self-test did not fail closed")
    missing_timing = [record for record in candidate if record.kind != "timing"]
    try:
        _compare_records(baseline, missing_timing)
    except QualificationError:
        pass
    else:
        raise AssertionError("timing-shape self-test did not fail closed")
    missing_registry = [
        record
        for record in baseline
        if not (record.kind == "scalar" and record.name == "life_mem_before_bytes")
    ]
    missing_registry_candidate = [
        record
        for record in candidate
        if not (record.kind == "scalar" and record.name == "life_mem_before_bytes")
    ]
    try:
        _compare_records(missing_registry, missing_registry_candidate)
    except QualificationError:
        pass
    else:
        raise AssertionError("unused footprint registry self-test did not fail closed")
    non_cmg_old = baseline + [
        Record("exact_match_joint", "scalar", "route_forecast_peak_bytes", 0, 0, one)
    ]
    non_cmg_new = candidate + [
        Record("exact_match_joint", "scalar", "route_forecast_peak_bytes", 0, 0, zero)
    ]
    try:
        _compare_records(non_cmg_old, non_cmg_new)
    except QualificationError:
        pass
    else:
        raise AssertionError("CMG-only footprint selector self-test did not fail closed")
    with tempfile.TemporaryDirectory() as temporary:
        artifact = Path(temporary) / "raw.tsv"
        artifact.write_bytes(b"good\n")
        try:
            _verify_hash(artifact, "0" * 64, "self-test artifact")
        except QualificationError:
            pass
        else:
            raise AssertionError("artifact hash self-test did not fail closed")
    dummy_revision = {"commit": "1" * 40, "tree": "2" * 40}
    payload = {
        "schema": RECEIPT_SCHEMA,
        "status": "PASS",
        "generated_at_utc": "2026-08-18T00:00:00Z",
        "baseline": {"commit": BASELINE_COMMIT, "tree": "3" * 40},
        "candidate": dummy_revision,
        "harness": {
            "orchestrator": {"path": ORCHESTRATOR_PATH.as_posix(), "sha256": "4" * 64},
            "driver": {"path": DRIVER_PATH.as_posix(), "sha256": "5" * 64},
        },
        "required_case_ids": list(REQUIRED_CASE_IDS),
        "cases": compared,
        "environment": {
            "stata_executable": {
                "path": "/Applications/Stata/stata-mp",
                "sha256": "7" * 64,
                "discovery_source": "explicit",
            },
            "runtime": {
                "stata_version": "19",
                "stata_flavor": "MP",
                "os": "MacOSX",
                "processors": "8",
                "processors_lic": "8",
            },
            "startup_profiles": [],
            "startup_boundary": {
                "profile_runs_before_driver": True,
                "discard": True,
                "mata_clear": True,
                "isolated_sysdirs": True,
                "exact_main_ado_path": True,
                "stdin_devnull": True,
                "KSS_PHASE_FILE": "removed",
                "STATATMP": "case-local",
                "TMPDIR": "case-local",
            },
        },
        "comparison": _comparison_descriptor(),
        "rust_artifacts": {
            role: {
                "architecture": "arm64",
                "plugin_filename": "vckss_rust_macos_arm64.plugin",
                "plugin_sha256": "8" * 64,
                "source_manifest_path": (
                    EVIDENCE_DIR / f"{role}.plugin-source-manifest.sha256"
                ).as_posix(),
                "source_manifest_sha256": "9" * 64,
                "source_file_count": 1,
            }
            for role in ("baseline", "candidate")
        },
        "raw_artifacts": {
            role: {
                kind: {
                    "path": (EVIDENCE_DIR / f"{role}.raw.{kind}").as_posix(),
                    "sha256": "6" * 64,
                }
                for kind in ("tsv", "json")
            }
            for role in ("baseline", "candidate")
        },
    }
    _validate_receipt(payload)
    payload["comparison"]["raw_values_retained"] = False  # type: ignore[index]
    try:
        _validate_receipt(payload)
    except QualificationError:
        pass
    else:
        raise AssertionError("receipt descriptor self-test did not fail closed")
    payload["comparison"] = _comparison_descriptor()
    payload["cases"][REQUIRED_CASE_IDS[-1]]["status"] = "FAIL"  # type: ignore[index]
    try:
        _validate_receipt(payload)
    except QualificationError:
        pass
    else:
        raise AssertionError("receipt self-test did not fail closed")
    print("PASS run_fevc_rename_equivalence.py --self-test")


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--self-test", action="store_true")
    action.add_argument("--validate-receipt", type=Path)
    parser.add_argument("--candidate", help="exact full candidate commit SHA")
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--stata")
    parser.add_argument("--baseline-plugin", type=Path)
    parser.add_argument("--candidate-plugin", type=Path)
    parser.add_argument("--timeout-seconds", type=int, default=1800)
    parser.add_argument(
        "--write-receipt",
        action="store_true",
        help=f"atomically write {RECEIPT_PATH} only after every gate passes",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(sys.argv[1:] if argv is None else argv)
    repo = Path(__file__).resolve().parents[2]
    if args.self_test:
        _self_test()
        return 0
    if args.validate_receipt is not None:
        receipt_path = args.validate_receipt.expanduser().resolve()
        if receipt_path != (repo / RECEIPT_PATH).resolve():
            raise QualificationError(
                f"receipt must use the canonical path: {repo / RECEIPT_PATH}"
            )
        with receipt_path.open("r", encoding="utf-8") as handle:
            _validate_receipt(json.load(handle), repo)
        print(f"PASS {receipt_path}")
        return 0
    if (
        not args.candidate
        or args.output_dir is None
        or args.baseline_plugin is None
        or args.candidate_plugin is None
    ):
        raise QualificationError(
            "--candidate, --output-dir, --baseline-plugin, and "
            "--candidate-plugin are required for qualification"
        )
    if args.timeout_seconds <= 0:
        raise QualificationError("--timeout-seconds must be positive")

    candidate_commit, candidate_tree = _assert_clean_committed_candidate(repo, args.candidate)
    _preflight_committed_candidate(repo, candidate_commit)
    baseline_commit = _git(repo, "rev-parse", "--verify", f"{BASELINE_COMMIT}^{{commit}}")
    if baseline_commit != BASELINE_COMMIT:
        raise QualificationError("registered predecessor did not resolve exactly")
    baseline_tree = _git(repo, "rev-parse", "--verify", f"{BASELINE_COMMIT}^{{tree}}")
    _require_full_sha(baseline_tree, "baseline tree")
    if baseline_tree != BASELINE_TREE:
        raise QualificationError("registered predecessor tree did not resolve exactly")

    output_dir = args.output_dir.expanduser().resolve()
    try:
        output_dir.relative_to(repo.resolve())
    except ValueError:
        pass
    else:
        raise QualificationError("--output-dir must be outside the repository")
    if output_dir.exists():
        raise QualificationError("--output-dir must not already exist")
    output_dir.mkdir(parents=True)
    stata, stata_discovery_source = _find_stata(args.stata)
    stata_hash_before = _sha256_file(stata)

    with tempfile.TemporaryDirectory(prefix="fevc-rename-eq-v1-") as temporary:
        run_root = Path(temporary)
        baseline_archive = run_root / "archive-baseline"
        candidate_archive = run_root / "archive-candidate"
        _archive_revision(repo, BASELINE_COMMIT, ".", baseline_archive)
        _archive_revision(repo, candidate_commit, ".", candidate_archive)
        baseline_package = baseline_archive / "vckss"
        candidate_package = candidate_archive / "fevc"
        driver = candidate_archive / DRIVER_PATH
        orchestrator = candidate_archive / ORCHESTRATOR_PATH
        for required in (driver, orchestrator):
            if not required.is_file():
                raise QualificationError(f"archived required file is missing: {required}")
        _preflight_package(baseline_package, "vckss")
        _preflight_package(candidate_package, "fevc")
        rust_artifacts: dict[str, dict[str, object]] = {}
        rust_manifest_bytes: dict[str, bytes] = {}
        for role, plugin_path, archive_root, package_root in (
            (
                "baseline",
                args.baseline_plugin,
                baseline_archive,
                baseline_package,
            ),
            (
                "candidate",
                args.candidate_plugin,
                candidate_archive,
                candidate_package,
            ),
        ):
            rust_artifacts[role], rust_manifest_bytes[role] = (
                _stage_source_bound_plugin(
                    plugin_path, archive_root, package_root, role
                )
            )
        if (
            rust_artifacts["baseline"]["plugin_sha256"]
            != rust_artifacts["candidate"]["plugin_sha256"]
        ):
            raise QualificationError(
                "private Rust plugin changed across the public-only rename"
            )
        harness = {
            "orchestrator": {
                "path": ORCHESTRATOR_PATH.as_posix(),
                "sha256": _sha256_file(orchestrator),
            },
            "driver": {"path": DRIVER_PATH.as_posix(), "sha256": _sha256_file(driver)},
        }
        roles = {
            "baseline": (baseline_package, "vckss", "vckss"),
            "candidate": (candidate_package, "fevc", "vckss"),
        }
        role_records: dict[str, list[Record]] = {role: [] for role in roles}
        startup_profiles_by_process: list[list[dict[str, object]]] = []
        for case_id in REQUIRED_CASE_IDS:
            for role, (package_root, command, prefix) in roles.items():
                records, _stdout, startup_profiles = _run_stata_case(
                    stata=stata,
                    driver=driver,
                    role=role,
                    case_id=case_id,
                    package_root=package_root,
                    command=command,
                    prefix=prefix,
                    run_root=run_root,
                    timeout_seconds=args.timeout_seconds,
                )
                role_records[role].extend(records)
                startup_profiles_by_process.append(startup_profiles)

        for role in roles:
            role_records[role].sort()
            _validate_role_records(role, role_records[role])
        runtime = _runtime_environment(role_records)
        cases = _compare_records(role_records["baseline"], role_records["candidate"])
        if _sha256_file(stata) != stata_hash_before:
            raise QualificationError("Stata executable changed during qualification")
        profiles = _profile_inventory(startup_profiles_by_process)
        environment: dict[str, object] = {
            "stata_executable": {
                "path": str(stata),
                "sha256": stata_hash_before,
                "discovery_source": stata_discovery_source,
            },
            "runtime": runtime,
            "startup_profiles": profiles,
            "startup_boundary": {
                "profile_runs_before_driver": True,
                "discard": True,
                "mata_clear": True,
                "isolated_sysdirs": True,
                "exact_main_ado_path": True,
                "stdin_devnull": True,
                "KSS_PHASE_FILE": "removed",
                "STATATMP": "case-local",
                "TMPDIR": "case-local",
            },
        }
        final_commit, final_tree = _assert_clean_committed_candidate(repo, args.candidate)
        if (final_commit, final_tree) != (candidate_commit, candidate_tree):
            raise QualificationError("candidate commit/tree changed during qualification")
        revisions = {
            "baseline": {"commit": BASELINE_COMMIT, "tree": baseline_tree},
            "candidate": {"commit": candidate_commit, "tree": candidate_tree},
        }
        artifacts: dict[str, dict[str, dict[str, str]]] = {}
        raw_bytes: dict[str, dict[str, bytes]] = {}
        for role in roles:
            artifacts[role], raw_bytes[role] = _write_role_artifacts(
                output_dir, role, revisions[role], role_records[role]
            )
        receipt = _receipt_payload(
            baseline=revisions["baseline"],
            candidate=revisions["candidate"],
            harness=harness,
            cases=cases,
            artifacts=artifacts,
            environment=environment,
            rust_artifacts=rust_artifacts,
        )
        _write_atomic(output_dir / "qualification.preview.json", _json_bytes(receipt))
        if args.write_receipt:
            publish_commit, publish_tree = _assert_clean_committed_candidate(
                repo, args.candidate
            )
            if (publish_commit, publish_tree) != (candidate_commit, candidate_tree):
                raise QualificationError("candidate changed before evidence publication")
            receipt_path = _publish_evidence_bundle(
                repo, raw_bytes, rust_manifest_bytes, receipt
            )
            _validate_receipt(receipt, repo, require_tracked=False)
            if receipt_path != repo / RECEIPT_PATH:
                raise QualificationError("published receipt path is not canonical")

    print(
        "PASS rename equivalence: "
        f"{BASELINE_COMMIT} -> {candidate_commit}; artifacts={output_dir}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except QualificationError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1) from error
