from pathlib import Path
import sys

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
import build_release_artifact as portable


def test_shipped_helpers_use_the_package_prefix():
    assert not list(ROOT.glob("_fevc*.ado"))
    helpers = sorted(ROOT.glob("fevc__*.ado"))
    assert len(helpers) == 31
    shipped = {str(item.relative) for item in portable.package_files(ROOT)}
    assert {p.name for p in helpers} <= shipped


@pytest.mark.parametrize("name,source,error", [
    ("_fevc_example.ado", "program define _fevc_example\nend\n", "underscore"),
    ("fevc__example.ado", "program define wrong_name\nend\n", "entrypoint"),
    ("fevc__" + "x" * 27 + ".ado", "program define too_long\nend\n", "name"),
])
def test_artifact_rejects_unloadable_or_legacy_ado_names(tmp_path, name, source, error):
    (tmp_path / "stata.toc").write_text("v 0.5.0-rc.1\n")
    (tmp_path / "fevc.pkg").write_text(f"v 3\nf {name}\n")
    (tmp_path / name).write_text(source)
    with pytest.raises(ValueError, match=error):
        portable.package_files(tmp_path)


def test_artifact_accepts_plugin_entrypoints_and_32_character_names(tmp_path):
    name = "fevc__" + "x" * 26
    (tmp_path / "stata.toc").write_text("v 0.5.0-rc.1\n")
    (tmp_path / "fevc.pkg").write_text(f"v 3\nf {name}.ado\n")
    (tmp_path / f"{name}.ado").write_text(
        f'if 1 {{\n    capture program {name}, plugin using("fevc_rust_macos.plugin")\n}}\n'
    )
    assert len(portable.package_files(tmp_path)) == 3


def test_artifact_rejects_duplicate_installation_basenames(tmp_path):
    (tmp_path / "stata.toc").write_text("v 0.5.0-rc.1\n")
    (tmp_path / "fevc.pkg").write_text("v 3\nf a/fevc__example.ado\nf b/fevc__example.ado\n")
    for directory in ("a", "b"):
        (tmp_path / directory).mkdir()
        (tmp_path / directory / "fevc__example.ado").write_text(
            "program define fevc__example\nend\n"
        )
    with pytest.raises(ValueError, match="duplicate.*basename"):
        portable.package_files(tmp_path)


@pytest.mark.parametrize("records,error", [
    ("f fevc__example.ado\nf fevc__example.ado\n", "duplicate"),
    ("f missing.ado\n", "missing package file"),
])
def test_artifact_rejects_duplicate_or_missing_files(tmp_path, records, error):
    (tmp_path / "stata.toc").write_text("v 0.5.0-rc.1\n")
    (tmp_path / "fevc.pkg").write_text("v 3\n" + records)
    (tmp_path / "fevc__example.ado").write_text("program define fevc__example\nend\n")
    with pytest.raises(ValueError, match=error):
        portable.package_files(tmp_path)
