#!/usr/bin/env python3
"""Deterministically instantiate the canonical Mata CMG template.

This is a development-time tool.  Generated Mata files remain standalone and
have no Python runtime dependency.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import tempfile
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
CMG_ROOT = ROOT / "shared" / "cmg"
TEMPLATE = CMG_ROOT / "src" / "cmg_core.mata.in"
MANIFEST = CMG_ROOT / "generated" / "manifest.json"
TOKEN = "@CMG_NS@"
GENERATOR_API = 1


@dataclass(frozen=True)
class Target:
    name: str
    namespace: str
    output: Path


TARGETS = {
    "test": Target("test", "cmgtest", CMG_ROOT / "generated" / "cmg_test.mata"),
    "ppml_talo": Target(
        "ppml_talo", "ppmltalo_cmg", CMG_ROOT / "generated" / "ppmltalo_cmg_core.mata"
    ),
    "kss_bc": Target("kss_bc", "kssbc_cmg", CMG_ROOT / "generated" / "kssbc_cmg_core.mata"),
    "kss_runtime": Target(
        "kss_runtime", "kssbc_cmg", ROOT / "kss_bc" / "kss_bc_cmg.mata"
    ),
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def normalized_template() -> bytes:
    data = TEMPLATE.read_bytes()
    if b"\r" in data:
        raise ValueError("canonical template must use LF line endings")
    if not data.endswith(b"\n"):
        raise ValueError("canonical template must end with one LF")
    text = data.decode("utf-8")
    if text.count(TOKEN) == 0:
        raise ValueError(f"canonical template does not contain {TOKEN}")
    forbidden = ["application/veneto-kss", "mx_", "CMG, Copyright"]
    for marker in forbidden:
        if marker in text:
            raise ValueError(f"canonical template crosses clean-room boundary: {marker}")
    return data


def render(target: Target) -> tuple[bytes, dict[str, object]]:
    canonical = normalized_template()
    canonical_hash = sha256(canonical)
    body = canonical.decode("utf-8").replace(TOKEN, target.namespace).encode("utf-8")
    if TOKEN.encode() in body:
        raise ValueError("unresolved namespace token")
    body_hash = sha256(body)
    header = (
        "*! generated clean-room CMG-inspired Mata core; do not edit\n"
        f"*! generator_api {GENERATOR_API}\n"
        f"*! namespace {target.namespace}\n"
        f"*! canonical_template_sha256 {canonical_hash}\n"
        f"*! generated_section_sha256 {body_hash}\n\n"
    ).encode("utf-8")
    artifact = header + body
    metadata: dict[str, object] = {
        "target": target.name,
        "namespace": target.namespace,
        "generator_api": GENERATOR_API,
        "canonical_template_sha256": canonical_hash,
        "generated_section_sha256": body_hash,
        "artifact_sha256": sha256(artifact),
        "output": str(target.output.relative_to(ROOT)),
    }
    return artifact, metadata


def write_targets(targets: list[Target]) -> None:
    metadata = []
    for target in targets:
        artifact, one = render(target)
        target.output.parent.mkdir(parents=True, exist_ok=True)
        target.output.write_bytes(artifact)
        metadata.append(one)
    all_metadata = {
        "schema_version": 1,
        "generator_api": GENERATOR_API,
        "canonical_template": str(TEMPLATE.relative_to(ROOT)),
        "targets": sorted(metadata, key=lambda row: str(row["target"])),
    }
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(all_metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def check_targets(targets: list[Target]) -> None:
    errors: list[str] = []
    metadata = []
    with tempfile.TemporaryDirectory(prefix="cmg-assemble-") as directory:
        temporary = Path(directory)
        for target in targets:
            artifact, one = render(target)
            (temporary / target.output.name).write_bytes(artifact)
            metadata.append(one)
            if not target.output.exists():
                errors.append(f"missing generated artifact: {target.output.relative_to(ROOT)}")
            elif target.output.read_bytes() != artifact:
                errors.append(f"generated artifact drift: {target.output.relative_to(ROOT)}")

            instantiated = artifact.split(b"\n\n", 1)[1].decode("utf-8")
            reversed_body = instantiated.replace(target.namespace, TOKEN).encode("utf-8")
            if reversed_body != normalized_template():
                errors.append(f"reverse-substitution mismatch: {target.name}")

    expected_manifest = {
        "schema_version": 1,
        "generator_api": GENERATOR_API,
        "canonical_template": str(TEMPLATE.relative_to(ROOT)),
        "targets": sorted(metadata, key=lambda row: str(row["target"])),
    }
    expected_bytes = (json.dumps(expected_manifest, indent=2, sort_keys=True) + "\n").encode()
    if not MANIFEST.exists():
        errors.append(f"missing generated manifest: {MANIFEST.relative_to(ROOT)}")
    elif MANIFEST.read_bytes() != expected_bytes:
        errors.append(f"generated manifest drift: {MANIFEST.relative_to(ROOT)}")
    if errors:
        raise SystemExit("\n".join(errors))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    selection = parser.add_mutually_exclusive_group(required=True)
    selection.add_argument("--target", choices=sorted(TARGETS))
    selection.add_argument("--all", action="store_true")
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    targets = list(TARGETS.values()) if args.all else [TARGETS[args.target]]
    if args.check:
        check_targets(targets)
    else:
        write_targets(targets)


if __name__ == "__main__":
    main()
