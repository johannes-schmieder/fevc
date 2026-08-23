from pathlib import Path


PRODUCTION = Path("varcomp_kss/varcomp_kss.ado")
PUBLIC_TEST = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
PLANNED_START = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
IMPL_START = "program define _vckss_impl, eclass sortpreserve\n"


def replace_once(source: str, old: str, new: str, label: str) -> str:
    count = source.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one source block, found {count}")
    print(f"replaced {label}")
    return source.replace(old, new, 1)


source = PRODUCTION.read_text(encoding="utf-8")
if source.count(PLANNED_START) != 1 or source.count(IMPL_START) != 1:
    raise RuntimeError("planned and implementation program boundaries are not unique")
prefix, tail = source.split(PLANNED_START, 1)
planned_body, impl_body = tail.split(IMPL_START, 1)
planned = PLANNED_START + planned_body
impl = IMPL_START + impl_body

# Scope every receipt/dispatch edit to the planned program.  Identical-looking
# fields in the frozen V2 generic program are deliberately outside this block.
planned = replace_once(
    planned,
    '''    local control_count : word count `controls'
    local deletion_code = cond("`deletionmode'"=="match",1,2)
''',
    '''    local control_count : word count `controls'
    local engine_requested = lower(strtrim("`enginerequested'"))
    local engine_expected_code = cond("`engine_requested'"=="auto",0,2)
    local engine_defer_expected = cond("`engine_requested'"=="auto",1,0)
    local generic_engine_guaranteed =                              ///
        "`engine_requested'"=="generic" |                        ///
        "`deletionmode'"=="observation" | `control_count'>0
    local deletion_code = cond("`deletionmode'"=="match",1,2)
''',
    "planned engine request locals",
)
planned = replace_once(
    planned,
    '''    if !inlist("`preconditioner_requested'","auto","diagonal","cmg") | ///
        !inlist("`phase_batch_mode'","auto","explicit") |            ///
''',
    '''    if !inlist("`engine_requested'","generic","auto") |          ///
        !`generic_engine_guaranteed' |                               ///
        !inlist("`preconditioner_requested'","auto","diagonal","cmg") | ///
        !inlist("`phase_batch_mode'","auto","explicit") |            ///
''',
    "planned engine validation",
)
planned = replace_once(
    planned,
    '''            "The planned Rust route received an invalid route, batch, or wall tuple."
''',
    '''            "The planned Rust route received an invalid engine, route, batch, or wall tuple."
''',
    "planned engine validation message",
)
if planned.count("        engine(generic) batchmode(`phase_batch_mode')") != 2:
    raise RuntimeError(
        "planned engine dispatch: expected capability and solve engine(generic) blocks"
    )
planned = planned.replace(
    "        engine(generic) batchmode(`phase_batch_mode')",
    "        engine(`engine_requested') batchmode(`phase_batch_mode')",
)
print("replaced planned capability and solve engine requests")
planned = replace_once(
    planned,
    "            `cap_engine_code'==2 &                                 ///\n",
    "            `cap_engine_code'==`engine_expected_code' &            ///\n",
    "planned capability engine code",
)
planned = replace_once(
    planned,
    "            `cap_eng_defer'==0 &                  ///\n",
    "            `cap_eng_defer'==`engine_defer_expected' &             ///\n",
    "planned capability engine deferral",
)
planned = replace_once(
    planned,
    "            `r_exact_flags'==256 & `r_engine_req'==2 & `r_engine_sel'==2 & ///\n",
    "            `r_exact_flags'==256 &                                ///\n            `r_engine_req'==`engine_expected_code' & `r_engine_sel'==2 & ///\n",
    "planned result engine reconciliation",
)
planned = replace_once(
    planned,
    "    ereturn scalar rust_cap_signature_lo = `cap_request_signature_lo'\n",
    "    ereturn scalar rust_cap_signature_lo = `cap_request_signature_lo'\n    ereturn scalar rust_cap_engine_deferred = `cap_eng_defer'\n",
    "planned engine deferral return",
)
planned = replace_once(
    planned,
    '''    ereturn local engine_requested "generic"
    ereturn local engine_selected "generic"
''',
    '''    ereturn local engine_requested "`engine_requested'"
    ereturn local engine_selected "generic"
''',
    "planned engine labels",
)

# The router edits are scoped to _vckss_impl, separately from both native
# wrapper programs.
impl = replace_once(
    impl,
    '''        local rust_planned_generic_supported =                 ///
            `algorithm_supplied' & "`algorithm'" == "jla" &       ///
            `engine_supplied' & "`engine_requested'" == "generic" & ///
''',
    '''        local rust_auto_engine_generic =                        ///
            "`engine_requested'"=="auto" &                         ///
            ("`deletion'"=="observation" |                        ///
                strtrim("`controlvars'")!="")
        local rust_planned_generic_supported =                 ///
            `algorithm_supplied' & "`algorithm'" == "jla" &       ///
            `engine_supplied' &                                   ///
            ("`engine_requested'"=="generic" |                   ///
                `rust_auto_engine_generic') &                      ///
''',
    "public generic-only engine-auto predicate",
)
impl = replace_once(
    impl,
    "The Rust route supports exact estimation, the frozen compressed JLA subset, the explicit generic-diagonal tuple, or planned generic auto/CMG and diagonal-with-planning tuples.",
    "The Rust route supports exact estimation, the frozen compressed JLA subset, the explicit generic-diagonal tuple, or planned generic routes including scientifically generic-only engine(auto) tuples.",
    "engine-auto support message",
)
PRODUCTION.write_text(prefix + planned + impl, encoding="utf-8")

public_test = PUBLIC_TEST.read_text(encoding="utf-8")
engine_auto_test = r'''
// Engine auto is public only where scientific eligibility guarantees generic.
tempname engine_auto_results engine_auto_memory engine_auto_capability
local engine_auto_rng `"`c(rng)'"'
local engine_auto_stream = c(rngstream)
local engine_auto_state `"`c(rngstate)'"'
local engine_auto_sortedby : sortedby
quietly _datasignature
local engine_auto_signature `"`r(datasignature)'"'
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(diagonal) ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) wallseconds(60) nodisplay
matrix `engine_auto_results' = e(results)
matrix `engine_auto_memory' = e(rust_memory_receipt)
matrix `engine_auto_capability' = e(rust_request_capability_receipt)
assert mreldif(`engine_auto_results',`forced_diagonal_results') == 0
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
assert `"`e(batch_requested)'"' == "auto"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-GENERIC-PLANNED-V4-V7"
assert `"`e(rust_capability_profile)'"' == "PLANNED_V1"
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_cap_engine_deferred) == 1
assert `engine_auto_capability'[1,14] == 0
assert e(rust_result_cap_schema) == 3 & e(rust_result_cap_profile) == 4
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 2
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2 & e(route_code) == 2
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 0
assert e(rust_leverage_batch_mode_code) == 0
assert e(rust_target_batch_mode_code) == 0
assert e(leverage_batch) >= 1 & e(leverage_batch) <= e(probes)
assert e(target_batch) >= 1 & e(target_batch) <= e(probes)
assert e(rust_wallseconds_supplied) == 1
assert e(rust_wallseconds_requested) == 60
assert e(rust_plan_solve_peak_bytes) == `engine_auto_memory'[1,11]
assert `engine_auto_memory'[1,12] == max(                       ///
    `engine_auto_memory'[1,5],`engine_auto_memory'[1,11])
assert `engine_auto_memory'[1,12] <= `engine_auto_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`engine_auto_rng'"'
assert c(rngstream) == `engine_auto_stream'
assert `"`c(rngstate)'"' == `"`engine_auto_state'"'
local engine_auto_sortedby_after : sortedby
assert `"`engine_auto_sortedby_after'"' == `"`engine_auto_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`engine_auto_signature'"'

'''
public_test = replace_once(
    public_test,
    "// Forced CMG uses the same V4/V7 lifecycle but may never fall back.\n",
    engine_auto_test
    + "// Forced CMG uses the same V4/V7 lifecycle but may never fall back.\n",
    "generic-only engine-auto public test",
)
public_test = replace_once(
    public_test,
    '''foreach auto_option in "algorithm(auto)" "engine(auto)"           ///
    "engine(compressed)" {
''',
    '''foreach auto_option in "algorithm(auto)" "engine(compressed)" {
''',
    "engine-auto unsupported-loop removal",
)
negative_auto = r'''
// No-control match tuples may select compressed and remain withheld until the
// public planned reconciler supports both compressed and generic result families.
capture quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(diagonal) ///
    batch(auto) probes(4) seed(81227) memory_gib(1) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""

'''
public_test = replace_once(
    public_test,
    "// Registered deletion/nuisance/control/weight combinations are all public.\n",
    negative_auto
    + "// Registered deletion/nuisance/control/weight combinations are all public.\n",
    "compressed-eligible engine-auto withholding test",
)
PUBLIC_TEST.write_text(public_test, encoding="utf-8")
print("staged generic-only engine(auto) public routing and tests")
