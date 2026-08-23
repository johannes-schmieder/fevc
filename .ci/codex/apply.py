from __future__ import annotations

from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one source block, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
    print(f"replaced {label}")


outer = Path("varcomp_kss/varcomp_kss_rust.ado")
replace_once(
    outer,
    """            `seed' < 0 | `probes' < 2 | `leveragebatch' <= 0 |          ///
            `targetbatch' <= 0 | `tolerance' <= 0 | `maxiter' <= 0 |       ///
""",
    """            `seed' < 0 | `probes' < 2 | `leveragebatch' < 0 |           ///
            `targetbatch' < 0 | `tolerance' <= 0 | `maxiter' <= 0 |        ///
""",
    "outer nonnegative batch admission",
)
replace_once(
    outer,
    """            return add
            exit
        }
        if "`engine'" == "" {
""",
    """            return add
            exit
        }
        if `leveragebatch' <= 0 | `targetbatch' <= 0 {
            di as err "legacy Rust solve requires positive leveragebatch() and targetbatch()"
            exit 198
        }
        if "`engine'" == "" {
""",
    "legacy positive batch fence",
)

helper = Path("varcomp_kss/_vckss_rust_solve_v4.ado")
replace_once(
    helper,
    """    if "`targetbatchmode'" == "auto" & `targetbatch' != 0 {
        di as err "targetbatch() must be zero with targetbatchmode(auto)"
        exit 198
    }
    if !inlist(`fallback', 0, 1) | (`fallback' == 1 & "`route'" != "auto") {
""",
    """    if "`targetbatchmode'" == "auto" & `targetbatch' != 0 {
        di as err "targetbatch() must be zero with targetbatchmode(auto)"
        exit 198
    }
    if "`leveragebatchmode'" == "explicit" & `leveragebatch' <= 0 {
        di as err "leveragebatch() must be positive with leveragebatchmode(explicit)"
        exit 198
    }
    if "`targetbatchmode'" == "explicit" & `targetbatch' <= 0 {
        di as err "targetbatch() must be positive with targetbatchmode(explicit)"
        exit 198
    }
    if !inlist(`fallback', 0, 1) | (`fallback' == 1 & "`route'" != "auto") {
""",
    "planned explicit positive batch fences",
)

fixture = Path("varcomp_kss/tests/stata/test_rust_planned_v4.do")
replace_once(
    fixture,
    """local probes = 7
quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match)   ///
    nuisance(joint) route(auto) seed(81227) probes(`probes')              ///
    leveragebatch(0) targetbatch(2) tolerance(1e-12) engine(generic)     ///
""",
    """local probes = 7
capture quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(auto) seed(81227) probes(`probes')              ///
    leveragebatch(3) targetbatch(2) tolerance(1e-12) engine(generic)      ///
    batchmode(independent) leveragebatchmode(auto)                        ///
    targetbatchmode(explicit) stayers(movers)                             ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    physicallimit(50000000) capabilityschema(`capability_schema')        ///
    capabilityprofile(`capability_profile') frequencyused(1)             ///
    signaturehi(`signature_hi') signaturelo(`signature_lo') fallback(1)  ///
    wallsecondssupplied(1) wallseconds(60)
local invalid_auto_rc = _rc
assert `invalid_auto_rc' == 198

capture quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(auto) seed(81227) probes(`probes')              ///
    leveragebatch(0) targetbatch(0) tolerance(1e-12) engine(generic)      ///
    batchmode(independent) leveragebatchmode(auto)                        ///
    targetbatchmode(explicit) stayers(movers)                             ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    physicallimit(50000000) capabilityschema(`capability_schema')        ///
    capabilityprofile(`capability_profile') frequencyused(1)             ///
    signaturehi(`signature_hi') signaturelo(`signature_lo') fallback(1)  ///
    wallsecondssupplied(1) wallseconds(60)
local invalid_explicit_rc = _rc
assert `invalid_explicit_rc' == 198

quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match)   ///
    nuisance(joint) route(auto) seed(81227) probes(`probes')              ///
    leveragebatch(0) targetbatch(2) tolerance(1e-12) engine(generic)     ///
""",
    "planned batch tuple regression cases",
)

# Fail closed on the exact postconditions that matter to the Stata/native contract.
outer_text = outer.read_text(encoding="utf-8")
helper_text = helper.read_text(encoding="utf-8")
fixture_text = fixture.read_text(encoding="utf-8")
for required in (
    "`leveragebatch' < 0",
    "`targetbatch' < 0",
    "legacy Rust solve requires positive leveragebatch() and targetbatch()",
):
    if outer_text.count(required) != 1:
        raise RuntimeError(f"outer solve postcondition failed for {required!r}")
for required in (
    "leveragebatch() must be zero with leveragebatchmode(auto)",
    "targetbatch() must be zero with targetbatchmode(auto)",
    "leveragebatch() must be positive with leveragebatchmode(explicit)",
    "targetbatch() must be positive with targetbatchmode(explicit)",
):
    if helper_text.count(required) != 1:
        raise RuntimeError(f"V4 helper postcondition failed for {required!r}")
if fixture_text.count("invalid_auto_rc") != 2 or fixture_text.count("invalid_explicit_rc") != 2:
    raise RuntimeError("planned V4 fixture did not acquire both batch tuple regressions")
print("aligned outer, planned, and legacy batch-width validation")
