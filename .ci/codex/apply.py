from pathlib import Path


PRODUCTION = Path("varcomp_kss/varcomp_kss.ado")
PUBLIC_TEST = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")


def replace_once(source: str, old: str, new: str, label: str) -> str:
    count = source.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one source block, found {count}")
    print(f"replaced {label}")
    return source.replace(old, new, 1)


source = PRODUCTION.read_text(encoding="utf-8")
old_predicate = '''        local rust_planned_generic_supported =                 ///
            `algorithm_supplied' & "`algorithm'" == "jla" &       ///
            `engine_supplied' & "`engine_requested'" == "generic" & ///
            `preconditioner_supplied' &                         ///
            inlist("`preconditioner'","auto","cmg") &             ///
            `batch_supplied' &                                     ///
            `rng_supplied' & "`rng_requested'" == "counter_v1" & ///
            inlist("`deletion'","match","observation") &          ///
            inlist("`nuisance'","joint","fixedoffset") &          ///
            "`stayers'" == "movers" & "`probeorder'" == ""
'''
new_predicate = '''        local rust_planned_generic_supported =                 ///
            `algorithm_supplied' & "`algorithm'" == "jla" &       ///
            `engine_supplied' & "`engine_requested'" == "generic" & ///
            `preconditioner_supplied' &                            ///
            (inlist("`preconditioner'","auto","cmg") |           ///
                ("`preconditioner'"=="diagonal" &                 ///
                    ("`batch_requested'"=="auto" |                ///
                        `wallseconds_supplied'))) &                 ///
            `batch_supplied' &                                     ///
            `rng_supplied' & "`rng_requested'" == "counter_v1" & ///
            inlist("`deletion'","match","observation") &          ///
            inlist("`nuisance'","joint","fixedoffset") &          ///
            "`stayers'" == "movers" & "`probeorder'" == ""
'''
source = replace_once(
    source,
    old_predicate,
    new_predicate,
    "planned forced-diagonal support predicate",
)
source = replace_once(
    source,
    "The Rust route supports exact estimation, the frozen compressed JLA subset, the explicit generic-diagonal tuple, or the planned generic auto/CMG tuple.",
    "The Rust route supports exact estimation, the frozen compressed JLA subset, the explicit generic-diagonal tuple, or planned generic auto/CMG and diagonal-with-planning tuples.",
    "planned route support message",
)
PRODUCTION.write_text(source, encoding="utf-8")

public_test = PUBLIC_TEST.read_text(encoding="utf-8")
forced_diagonal = r'''
// Forced diagonal enters V4/V7 only when automatic batching or wall planning
// is requested.  The explicit numeric-batch/no-wall tuple above remains V2.
tempname forced_diagonal_results forced_diagonal_memory
local forced_diagonal_rng `"`c(rng)'"'
local forced_diagonal_stream = c(rngstream)
local forced_diagonal_state `"`c(rngstate)'"'
local forced_diagonal_sortedby : sortedby
quietly _datasignature
local forced_diagonal_signature `"`r(datasignature)'"'
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) wallseconds(60) nodisplay
matrix `forced_diagonal_results' = e(results)
matrix `forced_diagonal_memory' = e(rust_memory_receipt)
assert mreldif(`forced_diagonal_results',`planned_reference') == 0
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
assert `"`e(batch_requested)'"' == "auto"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-GENERIC-PLANNED-V4-V7"
assert `"`e(rust_capability_profile)'"' == "PLANNED_V1"
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_result_cap_schema) == 3 & e(rust_result_cap_profile) == 4
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2 & e(route_code) == 2
assert e(rust_full_fit_route) == 2
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 0
assert e(rust_leverage_batch_mode_code) == 0
assert e(rust_target_batch_mode_code) == 0
assert e(leverage_batch) >= 1 & e(leverage_batch) <= e(probes)
assert e(target_batch) >= 1 & e(target_batch) <= e(probes)
assert e(batch) == max(e(leverage_batch),e(target_batch))
assert e(rust_plan_schema) == 1 & e(rust_plan_route_schema) == 2
assert e(rust_wallseconds_supplied) == 1
assert e(rust_wallseconds_requested) == 60
assert e(rust_wallseconds_forecast) >= 0
assert e(rust_wallseconds_advisory) >= 0
assert e(rust_wallseconds_margin) >= 0
assert e(rust_plan_solve_peak_bytes) == `forced_diagonal_memory'[1,11]
assert `forced_diagonal_memory'[1,12] == max(                  ///
    `forced_diagonal_memory'[1,5],`forced_diagonal_memory'[1,11])
assert `forced_diagonal_memory'[1,12] <= `forced_diagonal_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`forced_diagonal_rng'"'
assert c(rngstream) == `forced_diagonal_stream'
assert `"`c(rngstate)'"' == `"`forced_diagonal_state'"'
local forced_diagonal_sortedby_after : sortedby
assert `"`forced_diagonal_sortedby_after'"' == `"`forced_diagonal_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`forced_diagonal_signature'"'

'''
public_test = replace_once(
    public_test,
    "// Forced CMG uses the same V4/V7 lifecycle but may never fall back.\n",
    forced_diagonal
    + "// Forced CMG uses the same V4/V7 lifecycle but may never fall back.\n",
    "planned forced-diagonal public test",
)
old_forbidden = '''foreach forbidden in "stayers(movers)" "probeorder(replicate)"      ///
    "wallseconds(10)" "batch(auto)" {
    local preconditioner_option preconditioner(diagonal)
    local batch_option batch(2)
    if "`forbidden'" == "batch(auto)" local batch_option
'''
new_forbidden = '''foreach forbidden in "stayers(movers)" "probeorder(replicate)" {
    local preconditioner_option preconditioner(diagonal)
    local batch_option batch(2)
'''
public_test = replace_once(
    public_test,
    old_forbidden,
    new_forbidden,
    "planned diagonal supported-option test update",
)
PUBLIC_TEST.write_text(public_test, encoding="utf-8")
print("staged planned forced-diagonal public route and regression coverage")
