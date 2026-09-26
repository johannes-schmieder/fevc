"""Keep the private Windows entrypoint bounded and fail-closed."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def test_windows_driver_uses_one_stata_process_and_installed_plugin_first():
    driver = (ROOT / "windows-ci.do").read_text()
    assert "set processors 2" in driver
    assert "assert _rc == 601" in driver
    assert driver.index("net install fevc") < driver.index("test_rust_plugin.do")
    assert driver.index("test_rust_match_component_inference.do") < driver.index('"WINDOWS_CI=PASS"')
    assert "assert r(state) == 0 & r(handle) == 0" in driver
    assert "stata-mp" not in driver and "/e do" not in driver


def test_windows_build_pins_inputs_and_has_no_cloud_or_license_access():
    build = (ROOT / "rust/stata_backend/build_windows_ci.ps1").read_text()
    assert "cargo +1.85.1 build --release --locked" in build
    assert "-C target-feature=+crt-static" in build
    assert "Get-FileHash $destination -Algorithm SHA256" in build
    assert build.index("audit_windows_plugin.ps1") < build.index("FEVC_WINDOWS_BUILD=PASS")
    audit = (ROOT / "rust/stata_backend/audit_windows_plugin.ps1").read_text()
    assert "0x8664" in audit and "include/vckss_rust.h" in audit
    assert "candidate_sha256" in build and "Transferred Windows candidate hash mismatch" in build
    assert "BUILD_ONLY_NOT_QUALIFICATION" in build
    for forbidden in ("aws ", "SSM", "stcodes", "Serial number", "Remove-Item"):
        assert forbidden not in build
