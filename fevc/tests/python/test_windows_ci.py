"""Keep the private Windows entrypoint bounded and fail-closed."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def test_windows_driver_uses_one_stata_process_and_installed_plugin_first():
    wrapper = (ROOT / "windows-ci.do").read_text()
    driver = (ROOT / "rust/stata_backend/windows_runtime.do").read_text()
    assert "set processors 2" in driver
    assert "assert _rc == 601" in driver
    assert driver.index("net install fevc") < driver.index("test_rust_plugin.do")
    assert "test_rust_match_component_inference.do" in driver
    assert "assert r(component_centering_api) == 1" in driver
    assert "WINDOWS_CI=PASS" not in driver
    assert "assert r(state) == 0 & r(handle) == 0" in driver
    assert "capture noisily do" in wrapper
    assert wrapper.index("local rc_result = _rc") < wrapper.index("write_windows_failure.ps1")
    assert wrapper.index("exit `rc_result'") < wrapper.index('"WINDOWS_CI=PASS"')
    assert "stata-mp" not in driver + wrapper and "/e do" not in driver + wrapper


def test_windows_smoke_keeps_full_profile_and_distinguishes_qualification():
    driver = (ROOT / "rust/stata_backend/windows_runtime.do").read_text()
    build = (ROOT / "rust/stata_backend/build_windows_ci.ps1").read_text()
    receipt = (ROOT / "rust/stata_backend/write_windows_checks.ps1").read_text()
    full = driver.index('if `"`rc_profile\'"\' == "full"')
    for name in ("test_windows_runtime_smoke.do", "test_projection_mean_native.do"):
        assert driver.index(name) < full
    for name in ("test_rust_component_inference.do", "test_rust_individual_inference.do",
                 "test_rust_match_component_inference.do", "test_pooled_deletion.do",
                 "test_mcse_modes.do", "test_mcse_attachments.do",
                 "test_centering_mean.do", "test_centering_exact.do",
                 "test_centering_jla.do", "test_centering_options.do",
                 "test_component_centering_exact.do", "test_component_centering_native.do"):
        assert driver.index(name) > full
    assert "$runtimeProfile = 'full'" in build
    assert "-notin @('smoke', 'full')" in build
    assert "PRIVATE_BUILD_ONLY_SMOKE" in build and "PRIVATE_BUILD_ONLY_SMOKE" in receipt
    assert "runtime_profile=$build.runtime_profile" in receipt
    assert "'projection_mean'" in receipt and "'point_mean'" in receipt


def test_windows_failure_diagnostics_are_bounded_and_cannot_claim_pass():
    failure = (ROOT / "rust/stata_backend/write_windows_failure.ps1").read_text()
    assert "[ValidateRange(1, 99999)][int]$ReturnCode" in failure
    assert "$item.Length -le 128" in failure
    assert "ReparsePoint" in failure
    assert "$candidate -cin $allowed" in failure
    assert "status='FAIL'" in failure and "WINDOWS_CI=FAIL" in failure
    assert "WINDOWS_CI=PASS" not in failure
    assert ".log" not in failure and "Get-ChildItem" not in failure
    assert "stata_rc=$ReturnCode" in failure


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
