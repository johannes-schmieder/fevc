import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "fevc/tools"))
from check_cmg_vendor import check
from cmg_vendor import inventory


def test_full_cmg_vendor_is_unmodified_and_source_pinned() -> None:
    check()
    record = json.loads((ROOT / "rust/vendor/CMG-SOURCE.json").read_text())
    assert inventory(ROOT / "rust/vendor/cmg") == record["files"]
    assert {"Cargo.toml", "LICENSE", "src/lib.rs", "src/parallel_solver.rs"} <= record["files"].keys()
    assert not (ROOT / "rust/vendor/cmg/src/vckss_bridge.rs").exists()


def test_rust_185_is_the_normal_and_standalone_msrv() -> None:
    toolchain = (ROOT / "rust/rust-toolchain.toml").read_text(encoding="utf-8")
    workspace = (ROOT / "rust/Cargo.toml").read_text(encoding="utf-8")
    standalone = (ROOT / "rust/stata_backend/Cargo.toml").read_text(
        encoding="utf-8"
    )
    push_quick = (ROOT / "ci/run_rust_quick.sh").read_text(encoding="utf-8")
    assert 'channel = "1.85.1"' in toolchain
    assert 'rust-version = "1.85"' in workspace
    assert 'rust-version = "1.85"' in standalone
    assert 'toolchain="${RUST_TOOLCHAIN:-1.85.1}"' in push_quick
    assert "1.81.0" not in push_quick
