from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
VENDOR = ROOT / "rust/vendor/cmg"
CMG_COMMIT = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"


def test_full_cmg_vendor_is_source_pinned_and_normal_build_owned() -> None:
    record = (VENDOR / "VENDOR.md").read_text(encoding="utf-8")
    assert CMG_COMMIT in record
    assert "GPL-3.0-only" in record
    assert (VENDOR / "LICENSE").is_file()
    assert not (VENDOR / "src/bin").exists()

    workspace = (ROOT / "rust/Cargo.toml").read_text(encoding="utf-8")
    core = (ROOT / "rust/crates/vckss-core/Cargo.toml").read_text(encoding="utf-8")
    assert '"vendor/cmg"' in workspace
    assert 'cmg = { path = "../../vendor/cmg", features = ["parallel"] }' in core


def test_upstream_manifest_names_every_imported_upstream_file() -> None:
    entries: dict[str, str] = {}
    for line in (VENDOR / "UPSTREAM_MANIFEST.sha256").read_text(
        encoding="utf-8"
    ).splitlines():
        digest, relative = line.split("  ", 1)
        assert len(digest) == 64
        assert all(character in "0123456789abcdef" for character in digest)
        assert relative not in entries
        entries[relative] = digest
        assert (VENDOR / relative).is_file()
    assert "Cargo.toml" in entries
    assert "LICENSE" in entries
    assert "src/lib.rs" in entries
    assert "src/parallel_solver.rs" in entries
    assert len(entries) == 40


def test_rust_185_is_the_normal_and_standalone_msrv() -> None:
    toolchain = (ROOT / "rust/rust-toolchain.toml").read_text(encoding="utf-8")
    workspace = (ROOT / "rust/Cargo.toml").read_text(encoding="utf-8")
    standalone = (ROOT / "rust/stata_backend/Cargo.toml").read_text(
        encoding="utf-8"
    )
    assert 'channel = "1.85.1"' in toolchain
    assert 'rust-version = "1.85"' in workspace
    assert 'rust-version = "1.85"' in standalone
