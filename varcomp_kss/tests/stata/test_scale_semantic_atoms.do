version 18.0
clear all
set more off
set varabbrev off

local pkgroot "varcomp_kss"
capture confirm file "`pkgroot'/varcomp_kss.mata"
if _rc {
    local pkgroot "../.."
    capture confirm file "`pkgroot'/varcomp_kss.mata"
    if _rc {
        di as error "run from the repository root or varcomp_kss/tests/stata"
        exit 601
    }
}

quietly do "`pkgroot'/varcomp_kss.mata"
quietly do "`pkgroot'/varcomp_kss_scale.mata"
quietly do "`pkgroot'/varcomp_kss_rng.mata"
quietly do "`pkgroot'/varcomp_kss_scale_engine.mata"

/*
This is a semantic-atom boundary test.  The raw-row route below constructs
deletion units and exact target strata independently from the compressed
constructor, then asks the registered cursor for their atoms.  It therefore
checks the route-independent atom contract without depending on solver
batching.  The legacy generic estimator does not currently expose its actual
row-loop atom trace; see the handoff note for the minimal accessor needed to
test consumption inside that engine.
*/

set rng kiss32
set seed 20260816
local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'

mata:
mata set matastrict on
mata set matalnum on

struct ksssa_trace
{
    string scalar status
    string scalar contract
    real colvector unit_rank
    real colvector unit_trials
    real matrix leverage
    real colvector stratum_rank
    real colvector stratum_trials
    real matrix target
}

real matrix ksssa__key_panel(real matrix sorted_key)
{
    real scalar row
    real colvector group_code

    if (rows(sorted_key) == 0 | cols(sorted_key) == 0 |
        hasmissing(sorted_key)) return(J(0,2,.))
    group_code = J(rows(sorted_key),1,1)
    for (row=2; row<=rows(sorted_key); row++) {
        if (any(sorted_key[row,.] :!= sorted_key[row-1,.])) {
            group_code[row] = group_code[row-1]+1
        }
        else group_code[row] = group_code[row-1]
    }
    return(panelsetup(group_code,1))
}

real colvector ksssa__group_min(
    real colvector value,
    real colvector row_order,
    real matrix panel)
{
    real scalar group
    real colvector out

    out = J(rows(panel),1,.)
    for (group=1; group<=rows(panel); group++) {
        out[group] = min(value[row_order[|
            panel[group,1] \ panel[group,2]|]])
    }
    return(out)
}

real colvector ksssa__group_sum(
    real colvector value,
    real colvector row_order,
    real matrix panel)
{
    real scalar group, position, subtotal
    real colvector out

    out = J(rows(panel),1,0)
    for (group=1; group<=rows(panel); group++) {
        subtotal = 0
        for (position=panel[group,1]; position<=panel[group,2];
            position++) {
            subtotal = subtotal+value[row_order[position]]
        }
        out[group] = subtotal
    }
    return(out)
}

struct ksssa_trace scalar ksssa__raw_trace(
    real colvector worker,
    real colvector firm,
    real colvector deletion_id,
    real colvector frequency,
    real colvector target_weight,
    real colvector semantic_rank,
    real scalar seed,
    real scalar probes)
{
    struct ksssa_trace scalar out
    struct vckss_rng__cursor scalar cursor
    struct vckss_rng__result scalar generated
    real colvector row_order, unit_rank, stratum_rank, per_copy
    real matrix panel

    out.status = "INVALID_INPUT"
    out.contract = vckss_rng__production_contract()
    out.unit_rank = J(0,1,.)
    out.unit_trials = J(0,1,.)
    out.leverage = J(0,0,.)
    out.stratum_rank = J(0,1,.)
    out.stratum_trials = J(0,1,.)
    out.target = J(0,0,.)

    /* Generic-row semantic route: group directly from raw identifiers. */
    row_order = order(deletion_id,1)
    panel = ksssa__key_panel(deletion_id[row_order])
    unit_rank = ksssa__group_min(semantic_rank,row_order,panel)
    out.unit_trials = ksssa__group_sum(frequency,row_order,panel)
    out.unit_rank = unit_rank
    cursor = vckss_rng__open_cursor(
        seed,"leverage",out.unit_rank,out.unit_trials)
    if (cursor.status != "OK") {
        out.status = cursor.status
        return(out)
    }
    generated = vckss_rng__cursor_next(&cursor,probes)
    if (generated.status != "OK") {
        out.status = generated.status
        return(out)
    }
    out.unit_rank = generated.semantic_rank
    out.unit_trials = cursor.trials
    out.leverage = generated.atoms

    /* Target atoms are indexed by exact (coefficient cell, per-copy scale). */
    per_copy = target_weight:/frequency
    row_order = order((worker,firm,per_copy),(1,2,3))
    panel = ksssa__key_panel((worker,firm,per_copy)[row_order,.])
    stratum_rank = ksssa__group_min(semantic_rank,row_order,panel)
    out.stratum_trials = ksssa__group_sum(frequency,row_order,panel)
    out.stratum_rank = stratum_rank
    cursor = vckss_rng__open_cursor(
        seed,"target",out.stratum_rank,out.stratum_trials)
    if (cursor.status != "OK") {
        out.status = cursor.status
        return(out)
    }
    generated = vckss_rng__cursor_next(&cursor,probes)
    if (generated.status != "OK") {
        out.status = generated.status
        return(out)
    }
    out.stratum_rank = generated.semantic_rank
    out.stratum_trials = cursor.trials
    out.target = generated.atoms
    out.status = "CONVERGED"
    return(out)
}

struct ksssa_trace scalar ksssa__compressed_trace(
    real colvector worker,
    real colvector firm,
    real colvector deletion_id,
    real colvector frequency,
    real colvector outcome,
    real colvector target_weight,
    real colvector semantic_rank,
    real scalar seed,
    real scalar probes)
{
    struct ksssa_trace scalar out
    struct vckss_scale_design scalar design
    struct vckss_scale_rng_context scalar context
    struct vckss_scale_atom_provider scalar provider
    struct vckss_scale_engine_atom_batch scalar leverage, target
    real colvector unit_rank, stratum_rank, unit_order, stratum_order

    out.status = "INVALID_INPUT"
    out.contract = vckss_rng__production_contract()
    out.unit_rank = J(0,1,.)
    out.unit_trials = J(0,1,.)
    out.leverage = J(0,0,.)
    out.stratum_rank = J(0,1,.)
    out.stratum_trials = J(0,1,.)
    out.target = J(0,0,.)

    design = vckss_scale__prepare(
        worker,firm,deletion_id,frequency,outcome,target_weight,1e-12)
    if (design.status != "CONVERGED") {
        out.status = design.status
        return(out)
    }
    unit_rank = ksssa__group_min(
        semantic_rank,design.unit_row_order,design.unit_row_panel)
    stratum_rank = ksssa__group_min(
        semantic_rank,design.strata.row_order,design.strata.row_panel)
    context = vckss_scale_eng__rng_context(
        vckss_rng__k1_recommendation(),seed,
        unit_rank,design.unit_frequency,
        stratum_rank,design.strata.physical_count)
    if (context.status != "CONVERGED") {
        out.status = context.status
        return(out)
    }
    provider = vckss_scale_eng__rng_provider(&context)
    if (provider.status != "CONVERGED") {
        out.status = provider.status
        return(out)
    }
    leverage = (*provider.leverage)(provider.context,1,probes)
    target = (*provider.target)(provider.context,1,probes)
    if (leverage.status != "CONVERGED") {
        out.status = leverage.status
        return(out)
    }
    if (target.status != "CONVERGED") {
        out.status = target.status
        return(out)
    }

    unit_order = order(unit_rank,1)
    stratum_order = order(stratum_rank,1)
    out.contract = provider.contract
    out.unit_rank = unit_rank[unit_order]
    out.unit_trials = design.unit_frequency[unit_order]
    out.leverage = leverage.value[unit_order,.]
    out.stratum_rank = stratum_rank[stratum_order]
    out.stratum_trials = design.strata.physical_count[stratum_order]
    out.target = target.value[stratum_order,.]
    out.status = "CONVERGED"
    return(out)
}

void ksssa__assert_same(
    struct ksssa_trace scalar left,
    struct ksssa_trace scalar right)
{
    assert(left.status == "CONVERGED")
    assert(right.status == "CONVERGED")
    assert(left.contract != "")
    assert(left.contract == right.contract)
    assert(left.unit_rank == right.unit_rank)
    assert(left.unit_trials == right.unit_trials)
    assert(left.leverage == right.leverage)
    assert(left.stratum_rank == right.stratum_rank)
    assert(left.stratum_trials == right.stratum_trials)
    assert(left.target == right.target)
}

void ksssa__run()
{
    struct vckss_scale_design scalar design
    struct vckss_semantic_order scalar semantic
    struct ksssa_trace scalar raw, compressed, raw_tied, compressed_tied
    struct ksssa_trace scalar raw_alt, compressed_alt
    real colvector worker, firm, deletion_id, frequency, outcome, target
    real colvector semantic_rank, permutation, tied_permutation
    real colvector worker_alt, firm_alt, deletion_alt
    real scalar probes, seed

    /* Eligible no-control/match fixture with repeated rows, two deletion
       IDs in cells 1 and 6, and eight exact target-scale strata. */
    worker =   (1\1\1\1\1\2\2\2\3\3\3\3)
    firm =     (1\1\1\2\2\1\2\2\1\1\2\2)
    deletion_id = (101\101\102\103\103\104\105\105\
        106\106\107\108)
    frequency = (2\1\3\2\1\2\2\1\1\2\3\1)
    outcome =  (1.2\1.2\1.5\2.1\1.9\-.4\.3\.7\1.1\.9\-.2\.2)
    target =   (2\1\3\2\.5\2\4\1\1\2\3\1)
    semantic = vckss_scale__semantic_order(
        worker,firm,deletion_id,frequency,outcome,target,J(12,0,.))
    assert(semantic.status == "CONVERGED")
    semantic_rank = semantic.rank
    seed = 24681357
    probes = 6

    assert(vckss_rng__production_contract() != "")
    design = vckss_scale__prepare(
        worker,firm,deletion_id,frequency,outcome,target,1e-12)
    assert(design.status == "CONVERGED")
    assert(design.coefficient_cells == 6)
    assert(design.deletion_units == 8)
    assert(design.strata.count == 8)
    assert(design.diagnostic.cross_cell_deletion_units == 0)
    assert(design.diagnostic.max_units_per_cell == 2)
    assert(design.diagnostic.max_rows_per_deletion_unit == 2)

    raw = ksssa__raw_trace(
        worker,firm,deletion_id,frequency,target,semantic_rank,seed,probes)
    compressed = ksssa__compressed_trace(
        worker,firm,deletion_id,frequency,outcome,target,semantic_rank,
        seed,probes)
    ksssa__assert_same(raw,compressed)

    /* Rows 1 and 2 have an exact full semantic-key tie despite distinct
       integer frequencies and proportional targets (2/2 == 1/1).  Swapping
       only those tied rows cannot change any raw or compressed RNG atom. */
    assert(frequency[1] != frequency[2])
    assert(target[1]/frequency[1] == target[2]/frequency[2])
    assert(semantic_rank[1] == semantic_rank[2])
    tied_permutation = (2\1\3\4\5\6\7\8\9\10\11\12)
    raw_tied = ksssa__raw_trace(
        worker[tied_permutation],firm[tied_permutation],
        deletion_id[tied_permutation],frequency[tied_permutation],
        target[tied_permutation],semantic_rank[tied_permutation],seed,probes)
    compressed_tied = ksssa__compressed_trace(
        worker[tied_permutation],firm[tied_permutation],
        deletion_id[tied_permutation],frequency[tied_permutation],
        outcome[tied_permutation],target[tied_permutation],
        semantic_rank[tied_permutation],seed,probes)
    ksssa__assert_same(raw_tied,compressed_tied)
    ksssa__assert_same(raw,raw_tied)
    ksssa__assert_same(compressed,compressed_tied)

    /* This lower-level engine-equivalence check conditions on an explicitly
       supplied semantic rank. Relabel IDs and shuffle rows while carrying
       that rank; this is not a public pathwise ID-relabeling requirement. */
    worker_alt = 30:*(worker:==1) + 10:*(worker:==2) +
        20:*(worker:==3)
    firm_alt = 20:*(firm:==1) + 10:*(firm:==2)
    deletion_alt = 700000:-17:*deletion_id
    permutation = (12\3\8\1\10\5\2\11\6\4\9\7)
    worker_alt = worker_alt[permutation]
    firm_alt = firm_alt[permutation]
    deletion_alt = deletion_alt[permutation]
    frequency = frequency[permutation]
    outcome = outcome[permutation]
    target = target[permutation]
    semantic_rank = semantic_rank[permutation]

    raw_alt = ksssa__raw_trace(
        worker_alt,firm_alt,deletion_alt,frequency,target,semantic_rank,
        seed,probes)
    compressed_alt = ksssa__compressed_trace(
        worker_alt,firm_alt,deletion_alt,frequency,outcome,target,
        semantic_rank,seed,probes)
    ksssa__assert_same(raw_alt,compressed_alt)
    ksssa__assert_same(raw,raw_alt)
    ksssa__assert_same(compressed,compressed_alt)
}

ksssa__run()
end

assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'

di as result "PASS test_scale_semantic_atoms.do"
exit 0
