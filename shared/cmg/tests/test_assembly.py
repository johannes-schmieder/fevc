from __future__ import annotations

import importlib.util
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "shared" / "cmg" / "tools" / "assemble.py"


def _module():
    spec = importlib.util.spec_from_file_location("cmg_assemble", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_every_generated_symbol_is_namespaced() -> None:
    module = _module()
    for target in module.TARGETS.values():
        artifact, metadata = module.render(target)
        assert module.TOKEN.encode() not in artifact
        assert target.namespace.encode() in artifact
        assert metadata["canonical_template_sha256"]
        assert metadata["generated_section_sha256"]


def test_reverse_substitution_recovers_template() -> None:
    module = _module()
    template = module.normalized_template()
    for target in module.TARGETS.values():
        artifact, _ = module.render(target)
        body = artifact.split(b"\n\n", 1)[1].decode("utf-8")
        assert body.replace(target.namespace, module.TOKEN).encode("utf-8") == template


def test_every_declared_template_symbol_carries_the_namespace_token() -> None:
    module = _module()
    text = module.normalized_template().decode("utf-8")
    structures = re.findall(r"^struct\s+([@A-Za-z0-9_]+)\s*$", text, re.MULTILINE)
    definitions = re.findall(
        r"^(?:(?:real|string)\s+(?:scalar|matrix|colvector)|"
        r"pointer\([^)]+\)\s+scalar|struct\s+\S+\s+scalar)\s+"
        r"([@A-Za-z0-9_]+)\s*\(",
        text,
        re.MULTILINE,
    )
    assert len(structures) >= 10
    assert len(definitions) >= 40
    assert all(name.startswith(f"{module.TOKEN}__") for name in structures)
    assert all(name.startswith(f"{module.TOKEN}__") for name in definitions)
