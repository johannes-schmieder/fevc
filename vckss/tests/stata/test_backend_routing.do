version 18.0
clear all
set more off
set varabbrev off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

// A trap proves that fail-closed public routing never reaches the developer
// wrapper, and therefore cannot prepare a native Rust context.
global VCKSS_ROUTING_NATIVE_CALLED 0
global VCKSS_ROUTING_PREPARE_CALLED 0
global VCKSS_ROUTING_PROXY_MODE unavailable
capture program drop vckss_rust
program define vckss_rust, rclass
    global VCKSS_ROUTING_NATIVE_CALLED 1
    gettoken subcommand rest : 0, parse(" ,")
    local subcommand = lower(strtrim("`subcommand'"))
    if "`subcommand'" == "prepare" {
        global VCKSS_ROUTING_PREPARE_CALLED 1
    }
    if "$VCKSS_ROUTING_PROXY_MODE" == "unqualified" &       ///
        "`subcommand'" == "probe" {
        return scalar abi_compiled = 1
        return scalar abi_runtime = 1
        return scalar core_ready_flags = 237
        return scalar support_flags = 0
        return scalar deterministic_parallelism = 1
        exit 0
    }
    if inlist("$VCKSS_ROUTING_PROXY_MODE","cap_missing",      ///
        "cap_corrupt") & "`subcommand'" == "probe" {
        return scalar abi_compiled = 1
        return scalar abi_runtime = 1
        return scalar core_ready_flags = 511
        return scalar support_flags = 38
        return scalar deterministic_parallelism = 1
        exit 0
    }
    if "$VCKSS_ROUTING_PROXY_MODE" == "stale_runtime" &       ///
        "`subcommand'" == "probe" {
        return scalar abi_compiled = 1
        return scalar abi_runtime = 1
        return scalar core_ready_flags = 255
        return scalar support_flags = 38
        return scalar deterministic_parallelism = 1
        exit 0
    }
    if "$VCKSS_ROUTING_PROXY_MODE" == "cap_missing" &        ///
        "`subcommand'" == "requestcapability" {
        di as error "requestcapability unavailable"
        exit 198
    }
    if "$VCKSS_ROUTING_PROXY_MODE" == "cap_corrupt" &        ///
        "`subcommand'" == "requestcapability" {
        return scalar struct_size = 64
        return scalar abi_version = 1
        return scalar request_schema = 1
        return scalar supported = 1
        return scalar reason_code = 0
        return scalar profile_code = 1
        return scalar algorithm_code = 1
        return scalar deletion_mode_code = 1
        return scalar nuisance_mode_code = 1
        return scalar solver_route_code = 1
        return scalar rng_contract_code = 0
        return scalar controls_count = 2
        return scalar frequency_use_code = 0
        return scalar request_signature_hi = 0
        return scalar request_signature_lo = 0
        return local reason "SUPPORTED"
        return local profile "EXACT_V1"
        exit 0
    }
    if inlist("$VCKSS_ROUTING_PROXY_MODE","cap_missing",      ///
        "cap_corrupt") & "`subcommand'" == "clear" {
        exit 0
    }
    di as error "public backend routing unexpectedly called vckss_rust"
    exit 9
end

set obs 24
generate long obsid = _n
generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
generate double c1 = time - 1.5
generate double c2 = time == 2
generate byte firm = .
generate long match = .
generate double noise = .

local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `matches'
    quietly replace match = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5 + .3*worker - .2*firm + .4*c1 - .15*c2 + noise
sort obsid
set rng kiss32
set seed 20260821

local caller_rng `"`c(rng)'"'
local caller_rngstream = c(rngstream)
local caller_rngstate `"`c(rngstate)'"'
local caller_sortedby : sortedby
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

tempname default_results default_plugin default_correction default_kss
tempname mata_results auto_results

// Omitted backend()/rng() prefer Rust, but an unavailable plugin falls back
// before native preparation and estimator RNG.
quietly vckss y c1 c2, worker(worker) firm(firm)          ///
    deletion(match) deletionid(match) algorithm(exact) nodisplay
matrix `default_results' = e(results)
matrix `default_plugin' = e(plugin)
matrix `default_correction' = e(correction)
matrix `default_kss' = e(kss)
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(backend_selected)'"' == "mata"
assert `"`e(backend_routing_reason)'"' ==                      ///
    "Rust preflight unavailable; fell back to Mata before preparation and estimator RNG"
assert e(backend_option_supplied) == 0
assert e(backend_fallback) == 1
assert `"`e(backend_fallback_reason)'"' == "RUST_BACKEND_UNAVAILABLE"
assert `"`e(backend_fallback_phase)'"' == "preflight"
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == "stata"
assert e(rng_option_supplied) == 0
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"
assert "$VCKSS_ROUTING_PREPARE_CALLED" == "0"
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_rngstream'
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
local restored_sortedby : sortedby
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'

generate byte default_sample = e(sample)
quietly _datasignature
local routed_signature `"`r(datasignature)'"'
mata: VCKSS_BACKEND_ROUTING_BEFORE = vckss_rng__capture_full()
mata: assert(VCKSS_BACKEND_ROUTING_BEFORE.status == "OK")

// Omitted algorithm() mirrors the MATLAB package's randomized default.
global VCKSS_ROUTING_NATIVE_CALLED 0
quietly vckss y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) nodisplay
assert `"`e(algorithm)'"' == "jla"
assert e(probes) == 200
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(backend_selected)'"' == "mata"
assert e(backend_fallback) == 1
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == "stata"
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"
global VCKSS_ROUTING_NATIVE_CALLED 0

// Explicit Mata follows the same numerical and sample path.
quietly vckss y c1 c2, worker(worker) firm(firm)          ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    backend(mata) nodisplay
matrix `mata_results' = e(results)
assert mreldif(`default_results',`mata_results') == 0
assert mreldif(`default_plugin',e(plugin)) == 0
assert mreldif(`default_correction',e(correction)) == 0
assert mreldif(`default_kss',e(kss)) == 0
quietly count if default_sample != e(sample)
assert r(N) == 0
assert `"`e(backend_requested)'"' == "mata"
assert `"`e(backend_selected)'"' == "mata"
assert `"`e(backend_routing_reason)'"' == "backend(mata) explicitly selected"
assert e(backend_option_supplied) == 1
assert e(backend_fallback) == 0
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == "stata"
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

// Explicit auto has the same preflight-only fallback policy.
global VCKSS_ROUTING_NATIVE_CALLED 0
quietly vckss y c1 c2, worker(worker) firm(firm)          ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    backend(auto) nodisplay
matrix `auto_results' = e(results)
assert mreldif(`default_results',`auto_results') == 0
quietly count if default_sample != e(sample)
assert r(N) == 0
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(backend_selected)'"' == "mata"
assert `"`e(backend_routing_reason)'"' ==                      ///
    "Rust preflight unavailable; fell back to Mata before preparation and estimator RNG"
assert e(backend_option_supplied) == 1
assert e(backend_fallback) == 1
assert `"`e(rng_requested)'"' == "auto"
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"
assert "$VCKSS_ROUTING_PREPARE_CALLED" == "0"

// Exact Rust accepts omitted RNG and reaches the native availability gate;
// no RNG contract is selected or consumed.
capture quietly vckss y c1 c2, worker(worker) firm(firm)  ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    backend(rust) nodisplay
local rust_rc = _rc
assert `rust_rc' == 498
assert `"`e(status)'"' == "WITHHELD"
assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNAVAILABLE"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == ""
assert `"`e(backend_routing_reason)'"' ==                  ///
    "explicit Rust exact route could not load or probe the native backend"
assert e(backend_option_supplied) == 1
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == ""
assert e(rng_option_supplied) == 0
assert e(backend_fallback) == 0
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"
global VCKSS_ROUTING_NATIVE_CALLED 0

// Explicit Mata and Counter-V1 conflict before any plugin call.
capture quietly vckss y c1 c2, worker(worker) firm(firm)  ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    backend(mata) rng(counter_v1) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "COUNTER_RNG_BACKEND_MISMATCH"
assert `"`e(backend_requested)'"' == "mata"
assert `"`e(backend_selected)'"' == ""
assert e(backend_fallback) == 0
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

// Counter-V1 with automatic backend selection pins strict Rust and therefore
// does not fall back when the plugin is unavailable.
foreach auto_backend in "" "backend(auto)" {
    global VCKSS_ROUTING_NATIVE_CALLED 0
    capture quietly vckss y c1 c2, worker(worker) firm(firm) ///
        deletion(match) deletionid(match) algorithm(exact)        ///
        `auto_backend' rng(counter_v1) nodisplay
    assert _rc == 498
    assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNAVAILABLE"
    assert `"`e(backend_requested)'"' == "auto"
    assert `"`e(backend_selected)'"' == ""
    assert `"`e(rng_requested)'"' == "counter_v1"
    assert `"`e(rng_selected)'"' == ""
    assert e(backend_fallback) == 0
    assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"
}
global VCKSS_ROUTING_NATIVE_CALLED 0

// rng(stata) selects Mata immediately on an automatic backend request.
capture quietly vckss y c1 c2, worker(worker) firm(firm)  ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    rng(stata) nodisplay
assert _rc == 0
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(backend_selected)'"' == "mata"
assert e(backend_option_supplied) == 0
assert `"`e(rng_requested)'"' == "stata"
assert `"`e(rng_selected)'"' == "stata"
assert e(rng_option_supplied) == 1
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

capture quietly vckss y c1 c2, worker(worker) firm(firm)  ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    backend(mata) rng(stata) nodisplay
assert _rc == 0
assert `"`e(rng_requested)'"' == "stata"
assert `"`e(rng_selected)'"' == "stata"
assert e(rng_option_supplied) == 1
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

capture quietly vckss y c1 c2, worker(worker) firm(firm)  ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    backend(auto) rng(stata) nodisplay
assert _rc == 0
assert `"`e(rng_requested)'"' == "stata"
assert `"`e(rng_selected)'"' == "stata"
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

capture quietly vckss y c1 c2, worker(worker) firm(firm)  ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    rng(garbage) nodisplay
assert _rc == 198
assert `"`e(withholding_status)'"' == "INVALID_RNG"
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(backend_selected)'"' == ""
assert `"`e(backend_routing_reason)'"' == "invalid rng() value"
assert e(backend_option_supplied) == 0
assert `"`e(rng_requested)'"' == "garbage"
assert `"`e(rng_selected)'"' == ""
assert e(rng_option_supplied) == 1
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

// Even with Counter-V1 consent, unsupported structure fails before probing.
capture quietly vckss y c1 c2, worker(worker) firm(firm)  ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)      ///
    batch(2) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == ""
assert `"`e(backend_routing_reason)'"' ==                  ///
    "explicit Rust exact route rejected an unsupported option combination"
assert e(backend_option_supplied) == 1
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == ""
assert e(rng_option_supplied) == 1
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

// General validation after strict-route consent keeps the complete routing
// receipt and still fails before the native helper is probed.
capture quietly vckss y, worker(worker) firm(firm)        ///
    deletion(match) algorithm(jla) engine(compressed)           ///
    preconditioner(diagonal) batch(2) probes(1)                 ///
    backend(rust) rng(counter_v1) nodisplay
assert _rc == 198
assert `"`e(withholding_status)'"' == "INVALID_TUNING"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == ""
assert `"`e(backend_routing_reason)'"' ==                  ///
    "strict Rust route selected before capability preflight"
assert e(backend_option_supplied) == 1
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == ""
assert e(rng_option_supplied) == 1
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

// The public route rejects a GiB limit that rounds to zero bytes before it
// probes the helper.  This avoids losing the typed error in helper-side
// argument validation.
capture quietly vckss y, worker(worker) firm(firm)        ///
    deletion(match) deletionid(match) algorithm(jla)            ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)       ///
    batch(2) memory_gib(1e-12) nodisplay
assert _rc == 198
assert `"`e(withholding_status)'"' == "INVALID_MEMORY_ENVELOPE"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == ""
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == ""
assert `"`e(backend_routing_reason)'"' ==                  ///
    "explicit strict Rust route rejected by memory-envelope validation"
assert e(backend_option_supplied) == 1
assert e(rng_option_supplied) == 1
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

// A fully supported request that cannot load the helper is typed unavailable
// and retains every routing field.
global VCKSS_ROUTING_PROXY_MODE unavailable
capture quietly vckss y, worker(worker) firm(firm)        ///
    deletion(match) algorithm(jla) engine(compressed)           ///
    preconditioner(diagonal) batch(2) probes(4)                 ///
    backend(rust) rng(counter_v1) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNAVAILABLE"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == ""
assert `"`e(backend_routing_reason)'"' ==                  ///
    "explicit strict Rust route could not load or probe the native backend"
assert e(backend_option_supplied) == 1
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == ""
assert e(rng_option_supplied) == 1
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"

// A probe that loads but withholds any required capability is typed
// unqualified and retains the same complete routing surface.
global VCKSS_ROUTING_NATIVE_CALLED 0
global VCKSS_ROUTING_PROXY_MODE unqualified
capture quietly vckss y, worker(worker) firm(firm)        ///
    deletion(match) algorithm(jla) engine(compressed)           ///
    preconditioner(diagonal) batch(2) probes(4)                 ///
    backend(rust) rng(counter_v1) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNQUALIFIED"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == ""
assert `"`e(backend_routing_reason)'"' ==                  ///
    "explicit strict Rust route rejected an unqualified native capability receipt"
assert e(backend_option_supplied) == 1
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == ""
assert e(rng_option_supplied) == 1
assert e(rust_core_ready_flags) == 237
assert e(rust_support_flags) == 0
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"
global VCKSS_ROUTING_NATIVE_CALLED 0

// A plugin predating the production full-CMG readiness bit is rejected before
// preparation or estimator RNG even when every legacy bit is present.
global VCKSS_ROUTING_PROXY_MODE stale_runtime
global VCKSS_ROUTING_PREPARE_CALLED 0
capture quietly vckss y, worker(worker) firm(firm)        ///
    deletion(match) algorithm(jla) engine(compressed)           ///
    preconditioner(diagonal) batch(2) probes(4)                 ///
    backend(rust) rng(counter_v1) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNQUALIFIED"
assert e(rust_core_ready_flags) == 255
assert "$VCKSS_ROUTING_PREPARE_CALLED" == "0"
global VCKSS_ROUTING_NATIVE_CALLED 0

// Exact fails closed when the typed request query is missing or when any
// echoed tuple/signature field is corrupted.  Neither path reaches prepare.
foreach capability_mode in cap_missing cap_corrupt {
    global VCKSS_ROUTING_PROXY_MODE `capability_mode'
    global VCKSS_ROUTING_NATIVE_CALLED 0
    global VCKSS_ROUTING_PREPARE_CALLED 0
    capture quietly vckss y c1 c2, worker(worker) firm(firm) ///
        deletion(match) deletionid(match) algorithm(exact)        ///
        backend(rust) rng(auto) engine(generic) nodisplay
    assert _rc == 498
    if "`capability_mode'" == "cap_missing" {
        assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNAVAILABLE"
        assert `"`e(native_error_phase)'"' == "request_capability"
    }
    else {
        assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNQUALIFIED"
        assert `"`e(native_error_phase)'"' ==                        ///
            "request_capability_reconcile"
    }
    assert `"`e(rng_requested)'"' == "auto"
    assert `"`e(rng_selected)'"' == ""
    assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"
    assert "$VCKSS_ROUTING_PREPARE_CALLED" == "0"
}
global VCKSS_ROUTING_NATIVE_CALLED 0

// The engine-aware generic route crosses the same fail-closed query boundary
// only after its explicit tuple and materialized controls have been accepted.
foreach capability_mode in cap_missing cap_corrupt {
    global VCKSS_ROUTING_PROXY_MODE `capability_mode'
    global VCKSS_ROUTING_NATIVE_CALLED 0
    global VCKSS_ROUTING_PREPARE_CALLED 0
    capture quietly vckss y c1 c2, worker(worker) firm(firm) ///
        deletion(observation) nuisance(fixedoffset) algorithm(jla) ///
        backend(rust) rng(counter_v1) engine(generic)              ///
        preconditioner(diagonal) batch(2) probes(4) nodisplay
    assert _rc == 498
    if "`capability_mode'" == "cap_missing" {
        assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNAVAILABLE"
        assert `"`e(native_error_phase)'"' == "request_capability"
    }
    else {
        assert `"`e(withholding_status)'"' == "RUST_BACKEND_UNQUALIFIED"
        assert `"`e(native_error_phase)'"' ==                     ///
            "request_capability_reconcile"
    }
    assert `"`e(algorithm)'"' == "jla"
    assert `"`e(engine_requested)'"' == "generic"
    assert e(engine_option_supplied) == 1
    assert `"`e(backend_selected)'"' == ""
    assert `"`e(rng_selected)'"' == ""
    assert "$VCKSS_ROUTING_NATIVE_CALLED" == "1"
    assert "$VCKSS_ROUTING_PREPARE_CALLED" == "0"
}
global VCKSS_ROUTING_NATIVE_CALLED 0

capture quietly vckss y c1 c2, worker(worker) firm(firm)  ///
    deletion(match) deletionid(match) algorithm(exact)         ///
    backend(garbage) nodisplay
local invalid_rc = _rc
assert `invalid_rc' == 198
assert `"`e(status)'"' == "WITHHELD"
assert `"`e(withholding_status)'"' == "INVALID_BACKEND"
assert `"`e(backend_requested)'"' == "garbage"
assert `"`e(backend_selected)'"' == ""
assert `"`e(backend_routing_reason)'"' == "invalid backend() value"
assert e(backend_option_supplied) == 1
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == ""
assert e(rng_option_supplied) == 0
assert "$VCKSS_ROUTING_NATIVE_CALLED" == "0"

// The successful and typed-failure routes cumulatively preserve caller state.
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_rngstream'
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`routed_signature'"'
local restored_sortedby : sortedby
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'
mata:
VCKSS_BACKEND_ROUTING_AFTER = vckss_rng__capture_full()
assert(VCKSS_BACKEND_ROUTING_AFTER.status == "OK")
assert(VCKSS_BACKEND_ROUTING_AFTER.active.algorithm ==
    VCKSS_BACKEND_ROUTING_BEFORE.active.algorithm)
assert(VCKSS_BACKEND_ROUTING_AFTER.active.stream ==
    VCKSS_BACKEND_ROUTING_BEFORE.active.stream)
assert(VCKSS_BACKEND_ROUTING_AFTER.active.state ==
    VCKSS_BACKEND_ROUTING_BEFORE.active.state)
assert(VCKSS_BACKEND_ROUTING_AFTER.sort_state ==
    VCKSS_BACKEND_ROUTING_BEFORE.sort_state)
assert(VCKSS_BACKEND_ROUTING_AFTER.mt64s_stream1_state ==
    VCKSS_BACKEND_ROUTING_BEFORE.mt64s_stream1_state)
assert(VCKSS_BACKEND_ROUTING_AFTER.mt64s_stream2_state ==
    VCKSS_BACKEND_ROUTING_BEFORE.mt64s_stream2_state)
assert(VCKSS_BACKEND_ROUTING_AFTER.mt64s_selected_stream_state ==
    VCKSS_BACKEND_ROUTING_BEFORE.mt64s_selected_stream_state)
end

capture program drop vckss_rust
macro drop VCKSS_ROUTING_NATIVE_CALLED
macro drop VCKSS_ROUTING_PREPARE_CALLED
macro drop VCKSS_ROUTING_PROXY_MODE
di as result "PASS test_backend_routing.do"
