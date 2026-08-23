from __future__ import annotations

from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one source block, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
    print(f"replaced {label}")


wrapper = Path("varcomp_kss/varcomp_kss_rust.ado")

replace_once(
    wrapper,
    """        local signature_hi = scalar(__vckss_rust_solve_signature_hi)\n        local signature_lo = scalar(__vckss_rust_solve_signature_lo)\n        local receipt_mismatch = 0\n""",
    """        local signature_hi = scalar(__vckss_rust_solve_signature_hi)\n        local signature_lo = scalar(__vckss_rust_solve_signature_lo)\n        if `capability_schema' == 3 {\n            capture noisily _vckss_rust_plan_receipt\n            local plan_rc = _rc\n            if `plan_rc' {\n                quietly _vckss_rust_release_idle `plugin' `handle'\n                local cleanup_certified = r(certified)\n                if !`cleanup_certified' {\n                    di as err \"Rust V7 cleanup did not certify an idle native session\"\n                }\n                exit `plan_rc'\n            }\n            return add\n        }\n        local receipt_mismatch = 0\n""",
    "V7 result reconciliation dispatch",
)

replace_once(
    wrapper,
    """            if `engine_requested' != 2 | `engine_selected' != 2 |             ///\n                `capability_schema' != 2 | `capability_profile' != 3 |        ///\n""",
    """            if !((`capability_schema' == 2 & `engine_requested' == 2) |        ///\n                 (`capability_schema' == 3 &                                  ///\n                    inlist(`engine_requested', 0, 2))) |                      ///\n                `engine_selected' != 2 | !inlist(`capability_schema', 2, 3) | ///\n                (`capability_schema' == 2 & `capability_profile' != 3) |      ///\n                (`capability_schema' == 3 & `capability_profile' != 4) |      ///\n""",
    "planned generic result profile",
)

replace_once(
    wrapper,
    """            CAPABILITYPROFILE(integer 0) FREQUENCYUSED(integer 0)           ///\n            SIGNATUREHI(real 0) SIGNATURELO(real 0)]\n""",
    """            CAPABILITYPROFILE(integer 0) FREQUENCYUSED(integer 0)           ///\n            SIGNATUREHI(real 0) SIGNATURELO(real 0)                          ///\n            LEVERAGEBATCHMODE(string) TARGETBATCHMODE(string)                ///\n            FALLBACK(integer -1) WALLSECONDS(real 0)]\n""",
    "V4 solve syntax",
)

replace_once(
    wrapper,
    """        local block_tolerance_arg = strtrim(strofreal(`blocktolerance', \"%21.17f\"))\n        local engine = lower(strtrim(\"`engine'\"))\n        if \"`engine'\" == \"\" {\n""",
    """        local block_tolerance_arg = strtrim(strofreal(`blocktolerance', \"%21.17f\"))\n        local engine = lower(strtrim(\"`engine'\"))\n        local planned_solve = (`capabilityschema' == 3 | `capabilityprofile' == 4)\n        if `planned_solve' {\n            if `capabilityschema' != 3 | `capabilityprofile' != 4 {\n                di as err \"planned Rust solve requires capability schema 3/profile 4\"\n                exit 198\n            }\n            if \"`engine'\" == \"\" local engine auto\n            local batchmode = lower(strtrim(\"`batchmode'\"))\n            if \"`batchmode'\" == \"\" local batchmode explicit\n            local leveragebatchmode = lower(strtrim(\"`leveragebatchmode'\"))\n            local targetbatchmode = lower(strtrim(\"`targetbatchmode'\"))\n            if \"`batchmode'\" == \"independent\" &                         ///\n                (\"`leveragebatchmode'\" == \"\" | \"`targetbatchmode'\" == \"\") {\n                di as err \"batchmode(independent) requires both phase batch modes\"\n                exit 198\n            }\n            if \"`leveragebatchmode'\" == \"\" local leveragebatchmode `batchmode'\n            if \"`targetbatchmode'\" == \"\" local targetbatchmode `batchmode'\n            local stayers = lower(strtrim(\"`stayers'\"))\n            if \"`stayers'\" == \"\" local stayers movers\n            local targetweightmode = lower(strtrim(\"`targetweightmode'\"))\n            if \"`targetweightmode'\" == \"\" local targetweightmode frequency\n            local deletionsource = lower(strtrim(\"`deletionsource'\"))\n            if \"`deletionsource'\" == \"\" {\n                local deletionsource cell\n                if \"`deletion'\" == \"observation\" local deletionsource observation\n            }\n            if `fallback' < 0 local fallback = (\"`route'\" == \"auto\")\n            foreach value in physicallimit signaturehi signaturelo {\n                if missing(``value'') | ``value'' < 0 |                    ///\n                    ``value'' != floor(``value'') {\n                    di as err \"`value'() must be a nonnegative exactly represented integer\"\n                    exit 198\n                }\n            }\n            if `physicallimit' <= 0 | `physicallimit' > 9007199254740992 | ///\n                `signaturehi' > 4294967295 | `signaturelo' > 4294967295 {\n                di as err \"physical limit or capability signature half is out of range\"\n                exit 198\n            }\n            local physical_arg = strtrim(strofreal(`physicallimit', \"%21.0f\"))\n            local signature_hi_arg = strtrim(strofreal(`signaturehi', \"%21.0f\"))\n            local signature_lo_arg = strtrim(strofreal(`signaturelo', \"%21.0f\"))\n            local wallseconds_arg = strtrim(strofreal(`wallseconds', \"%21.17g\"))\n            _vckss_rust_solve_v4 `plugin' `handle' `seed' `probes'        ///\n                `leveragebatch' `targetbatch' `route' `tolerance_arg'    ///\n                `maxiter' `algorithm' `deletion' `nuisance' `exactlimit' ///\n                `blocksizelimit' `rank_tolerance_arg'                    ///\n                `block_tolerance_arg' `engine' `batchmode' `stayers'     ///\n                `targetweightmode' `deletionsource' `probeordersupplied' ///\n                `wallsecondssupplied' `physical_arg' `capabilityschema'  ///\n                `capabilityprofile' `frequencyused' `signature_hi_arg'   ///\n                `signature_lo_arg' `leveragebatchmode'                   ///\n                `targetbatchmode' `fallback' `wallseconds_arg'\n            return add\n            exit\n        }\n        if \"`engine'\" == \"\" {\n""",
    "V4 solve dispatch",
)

layout = Path("varcomp_kss/tests/python/test_package_layout.py")
replace_once(
    layout,
    """        \"_vckss_rust_plugin_call.ado\",\n        \"_vckss_rust_macos.ado\",\n""",
    """        \"_vckss_rust_plugin_call.ado\",\n        \"_vckss_rust_solve_v4.ado\",\n        \"_vckss_rust_plan_receipt.ado\",\n        \"_vckss_rust_macos.ado\",\n""",
    "package layout helper set",
)
