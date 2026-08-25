from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "rust/tools/run_supply_chain_checks.sh"
VALIDATOR = ROOT / "rust/tools/validate_supply_chain.py"


def load_validator():
    spec = importlib.util.spec_from_file_location("validate_supply_chain", VALIDATOR)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_supply_chain_entrypoints_are_valid() -> None:
    subprocess.run(["bash", "-n", str(SCRIPT)], check=True)
    compile(VALIDATOR.read_text(encoding="utf-8"), str(VALIDATOR), "exec")
    assert SCRIPT.stat().st_mode & 0o111
    assert VALIDATOR.stat().st_mode & 0o111


def test_supply_chain_gate_is_source_bound_and_pinned() -> None:
    source = SCRIPT.read_text(encoding="utf-8")
    for required in (
        "VCKSS-SUPPLY-CHAIN-QUALIFICATION-V1",
        "cargo-audit 0.22.2",
        "cargo-cyclonedx 0.5.9",
        "--deny warnings",
        "--spec-version 1.5",
        "--license-strict",
        "SOURCE_DATE_EPOCH",
        "git -C \"${repo_root}\" archive",
        "refusing to overwrite SBOM directory",
        "human-license-provenance-approval",
    ):
        assert required in source


def test_validator_normalizes_paths_and_requires_registered_licenses() -> None:
    module = load_validator()
    component = {
        "bom-ref": "registry+https://github.com/rust-lang/crates.io-index#demo@1.2.3",
        "name": "demo",
        "version": "1.2.3",
        "licenses": [{"expression": "MIT OR Apache-2.0"}],
        "purl": "pkg:cargo/demo@1.2.3",
        "hashes": [{"alg": "SHA-256", "content": "a" * 64}],
    }
    assert module.validate_component(component, sbom_name="fixture") == [
        "MIT OR Apache-2.0"
    ]
    normalized = module.normalize_paths(
        {"ref": "path+file:///private/tmp/archive/rust#demo"},
        "/private/tmp/archive",
    )
    assert normalized == {"ref": "path+file:///SOURCE/rust#demo"}
    component["licenses"] = [{"expression": "UNKNOWN"}]
    try:
        module.validate_component(component, sbom_name="fixture")
    except ValueError as error:
        assert "unreviewed licenses" in str(error)
    else:
        raise AssertionError("unknown license unexpectedly passed")


def test_fuzz_workspace_declares_its_package_license() -> None:
    manifest = (ROOT / "rust/fuzz/Cargo.toml").read_text(encoding="utf-8")
    assert 'license = "GPL-3.0-only"' in manifest
    assert 'repository = "https://github.com/johannes-schmieder/vckss"' in manifest


def test_validator_allowlist_documents_only_one_legacy_bridge() -> None:
    module = load_validator()
    assert module.LEGACY_LICENSE_EQUIVALENCES == {
        "MIT/Apache-2.0": "MIT OR Apache-2.0"
    }
    assert set(module.LEGACY_LICENSE_EQUIVALENCES) <= module.ALLOWED_LICENSES


def test_validator_reconciles_pinned_no_fetch_audit_metadata(tmp_path: Path) -> None:
    module = load_validator()
    database_commit = "a" * 40
    database = {
        "advisory-count": 12,
        "last-commit": database_commit,
        "last-updated": "2026-08-25T00:00:00Z",
    }
    audits = []
    locks = {}
    for index, name in enumerate(("workspace", "stata_backend", "fuzz")):
        audit_path = tmp_path / f"{name}.json"
        audit_database = database if index == 0 else {
            "advisory-count": 12,
            "last-commit": None,
            "last-updated": None,
        }
        audit_path.write_text(
            json.dumps(
                {
                    "database": audit_database,
                    "lockfile": {"dependency-count": index + 1},
                    "vulnerabilities": {"found": False, "count": 0, "list": []},
                    "warnings": {},
                }
            ),
            encoding="utf-8",
        )
        lock_path = tmp_path / f"{name}.lock"
        lock_path.write_text(f"lock-{name}\n", encoding="utf-8")
        audits.append(module.NamedPath(name=name, path=audit_path))
        locks[name] = lock_path

    summary = module.validate_audits(audits, locks)
    assert summary["database"] == database

    divergent = json.loads(audits[-1].path.read_text(encoding="utf-8"))
    divergent["database"]["advisory-count"] = 11
    audits[-1].path.write_text(json.dumps(divergent), encoding="utf-8")
    try:
        module.validate_audits(audits, locks)
    except ValueError as error:
        assert "advisory counts" in str(error)
    else:
        raise AssertionError("divergent no-fetch database unexpectedly passed")
