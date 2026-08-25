#!/usr/bin/env python3
"""Normalize and validate exact-source Rust audit and CycloneDX evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ALLOWED_LICENSES = {
    "GPL-3.0-only",
    "MIT",
    "MIT OR Apache-2.0",
    "(MIT OR Apache-2.0) AND NCSA",
    # version_check 0.9.5 publishes this deprecated pre-SPDX separator.
    "MIT/Apache-2.0",
}
LEGACY_LICENSE_EQUIVALENCES = {
    "MIT/Apache-2.0": "MIT OR Apache-2.0",
}
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")


@dataclass(frozen=True)
class NamedPath:
    name: str
    path: Path


def parse_named_path(value: str) -> NamedPath:
    name, separator, raw_path = value.partition("=")
    if not separator or not name or not raw_path:
        raise argparse.ArgumentTypeError("expected NAME=PATH")
    return NamedPath(name=name, path=Path(raw_path))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def normalize_paths(value: Any, archive_root: str) -> Any:
    if isinstance(value, str):
        return value.replace(archive_root, "/SOURCE")
    if isinstance(value, list):
        return [normalize_paths(item, archive_root) for item in value]
    if isinstance(value, dict):
        return {
            key: normalize_paths(item, archive_root)
            for key, item in value.items()
        }
    return value


def component_licenses(component: dict[str, Any]) -> list[str]:
    values: list[str] = []
    for entry in component.get("licenses", []):
        if "expression" in entry:
            values.append(str(entry["expression"]))
            continue
        license_value = entry.get("license", {})
        identifier = license_value.get("id") or license_value.get("name")
        if identifier:
            values.append(str(identifier))
    return values


def validate_component(component: dict[str, Any], *, sbom_name: str) -> list[str]:
    name = component.get("name")
    version = component.get("version")
    if not isinstance(name, str) or not name:
        raise ValueError(f"{sbom_name}: component without a name")
    if not isinstance(version, str) or not version:
        raise ValueError(f"{sbom_name}: {name} lacks a version")
    purl = component.get("purl")
    if not isinstance(purl, str) or not purl.startswith("pkg:cargo/"):
        raise ValueError(f"{sbom_name}: {name} lacks a Cargo purl")
    licenses = component_licenses(component)
    if not licenses:
        raise ValueError(f"{sbom_name}: {name} lacks a license inventory")
    unexpected = sorted(set(licenses) - ALLOWED_LICENSES)
    if unexpected:
        raise ValueError(f"{sbom_name}: {name} has unreviewed licenses {unexpected}")
    bom_ref = str(component.get("bom-ref", ""))
    if bom_ref.startswith("registry+"):
        hashes = component.get("hashes", [])
        sha256_hashes = [
            str(item.get("content", "")).lower()
            for item in hashes
            if item.get("alg") == "SHA-256"
        ]
        if len(sha256_hashes) != 1 or not SHA256_RE.fullmatch(sha256_hashes[0]):
            raise ValueError(f"{sbom_name}: registry component {name} lacks one SHA-256")
    if "git+" in bom_ref or "git+" in purl:
        raise ValueError(f"{sbom_name}: unregistered git dependency {name}")
    return licenses


def normalize_sbom(
    path: Path,
    *,
    archive_root: str,
    source_commit: str,
    source_epoch: int,
) -> tuple[dict[str, Any], list[tuple[str, str, str, str]]]:
    data = normalize_paths(json.loads(path.read_text(encoding="utf-8")), archive_root)
    if data.get("bomFormat") != "CycloneDX" or data.get("specVersion") != "1.5":
        raise ValueError(f"{path}: expected CycloneDX 1.5")
    if data.get("version") != 1 or "serialNumber" in data:
        raise ValueError(f"{path}: SBOM is not deterministic version 1")
    metadata = data.get("metadata", {})
    expected_timestamp = datetime.fromtimestamp(
        source_epoch, tz=timezone.utc
    ).strftime("%Y-%m-%dT%H:%M:%S.000000000Z")
    if metadata.get("timestamp") != expected_timestamp:
        raise ValueError(f"{path}: timestamp is not bound to SOURCE_DATE_EPOCH")
    tools = metadata.get("tools", [])
    if not any(
        tool.get("name") == "cargo-cyclonedx" and tool.get("version") == "0.5.9"
        for tool in tools
    ):
        raise ValueError(f"{path}: unexpected SBOM generator")
    properties = [
        item
        for item in metadata.get("properties", [])
        if not str(item.get("name", "")).startswith("vckss:source:")
    ]
    properties.extend(
        [
            {"name": "vckss:source:commit", "value": source_commit},
            {"name": "vckss:source:archive-root", "value": "/SOURCE"},
        ]
    )
    metadata["properties"] = properties

    top = metadata.get("component")
    if not isinstance(top, dict):
        raise ValueError(f"{path}: missing top-level component")
    sbom_name = str(top.get("name", path.stem))
    records: list[tuple[str, str, str, str]] = []
    for component in [top, *data.get("components", [])]:
        licenses = validate_component(component, sbom_name=sbom_name)
        source = (
            "crates.io"
            if str(component.get("bom-ref", "")).startswith("registry+")
            else "local"
        )
        records.append(
            (
                str(component["name"]),
                str(component["version"]),
                " OR ".join(licenses),
                source,
            )
        )
    rendered = json.dumps(data, indent=2, sort_keys=True) + "\n"
    if archive_root in rendered or "/Users/" in rendered or "/private/tmp/" in rendered:
        raise ValueError(f"{path}: normalized SBOM leaks a machine-local path")
    return data, records


def validate_audits(audits: list[NamedPath], locks: dict[str, Path]) -> dict[str, Any]:
    if {item.name for item in audits} != set(locks):
        raise ValueError("audit and lock labels differ")
    database: dict[str, Any] | None = None
    reports: dict[str, Any] = {}
    for audit in audits:
        data = json.loads(audit.path.read_text(encoding="utf-8"))
        current_database = data.get("database", {})
        if not COMMIT_RE.fullmatch(str(current_database.get("last-commit", ""))):
            raise ValueError(f"{audit.name}: invalid RustSec database commit")
        if int(current_database.get("advisory-count", 0)) <= 0:
            raise ValueError(f"{audit.name}: empty RustSec database")
        if database is None:
            database = current_database
        elif current_database != database:
            raise ValueError("audits used different RustSec database snapshots")
        vulnerabilities = data.get("vulnerabilities", {})
        if vulnerabilities.get("found") is not False or vulnerabilities.get("count") != 0:
            raise ValueError(f"{audit.name}: RustSec vulnerabilities found")
        if data.get("warnings"):
            raise ValueError(f"{audit.name}: RustSec warnings found")
        dependency_count = int(data.get("lockfile", {}).get("dependency-count", 0))
        if dependency_count <= 0:
            raise ValueError(f"{audit.name}: invalid dependency count")
        reports[audit.name] = {
            "dependency_count": dependency_count,
            "lock_sha256": sha256(locks[audit.name]),
            "vulnerabilities": 0,
            "warnings": 0,
        }
    assert database is not None
    return {"database": database, "lockfiles": dict(sorted(reports.items()))}


def write_evidence(
    *,
    sboms: list[Path],
    audits: list[NamedPath],
    locks: dict[str, Path],
    archive_root: str,
    source_commit: str,
    source_epoch: int,
    output_dir: Path,
) -> dict[str, Any]:
    output_dir.mkdir(parents=True, exist_ok=False)
    inventory: list[tuple[str, str, str, str, str]] = []
    output_sboms: list[Path] = []
    seen_top_names: set[str] = set()
    for input_path in sboms:
        data, records = normalize_sbom(
            input_path,
            archive_root=archive_root,
            source_commit=source_commit,
            source_epoch=source_epoch,
        )
        top_name = str(data["metadata"]["component"]["name"])
        if top_name in seen_top_names:
            raise ValueError(f"duplicate top-level SBOM component: {top_name}")
        seen_top_names.add(top_name)
        output_path = output_dir / f"{top_name}.cdx.json"
        output_path.write_text(
            json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        output_sboms.append(output_path)
        inventory.extend((*record, top_name) for record in records)

    audit_summary = validate_audits(audits, locks)
    audit_summary.update(
        {
            "cargo_audit_version": "0.22.2",
            "source_commit": source_commit,
        }
    )
    audit_path = output_dir / "RUSTSEC_AUDIT_SUMMARY.json"
    audit_path.write_text(
        json.dumps(audit_summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    inventory_path = output_dir / "LICENSE_INVENTORY.tsv"
    inventory_lines = ["component\tversion\tlicense\tsource\tsbom"]
    inventory_lines.extend("\t".join(row) for row in sorted(inventory))
    inventory_path.write_text("\n".join(inventory_lines) + "\n", encoding="utf-8")

    evidence_files = sorted([*output_sboms, audit_path, inventory_path])
    manifest_path = output_dir / "SBOM_MANIFEST.sha256"
    manifest_path.write_text(
        "".join(f"{sha256(path)}  {path.name}\n" for path in evidence_files),
        encoding="utf-8",
    )
    license_values = sorted({row[2] for row in inventory})
    return {
        "advisory_count": audit_summary["database"]["advisory-count"],
        "component_records": len(inventory),
        "legacy_license_equivalences": LEGACY_LICENSE_EQUIVALENCES,
        "license_values": license_values,
        "manifest_sha256": sha256(manifest_path),
        "rustsec_database_commit": audit_summary["database"]["last-commit"],
        "rustsec_database_updated": audit_summary["database"]["last-updated"],
        "sbom_count": len(output_sboms),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sbom", action="append", type=Path, required=True)
    parser.add_argument("--audit", action="append", type=parse_named_path, required=True)
    parser.add_argument("--lock", action="append", type=parse_named_path, required=True)
    parser.add_argument("--archive-root", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-epoch", type=int, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    if not COMMIT_RE.fullmatch(args.source_commit):
        parser.error("--source-commit must be a full SHA")
    lock_map = {item.name: item.path for item in args.lock}
    if len(lock_map) != len(args.lock):
        parser.error("duplicate --lock label")
    summary = write_evidence(
        sboms=args.sbom,
        audits=args.audit,
        locks=lock_map,
        archive_root=args.archive_root,
        source_commit=args.source_commit,
        source_epoch=args.source_epoch,
        output_dir=args.output_dir,
    )
    for key, value in sorted(summary.items()):
        if isinstance(value, dict):
            rendered = ",".join(f"{left}->{right}" for left, right in sorted(value.items()))
        elif isinstance(value, list):
            rendered = "|".join(value)
        else:
            rendered = str(value)
        print(f"{key}={rendered}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
