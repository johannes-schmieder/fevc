version 18.0
clear all
set more off
set varabbrev off

/*
Source-bound developer-backend differential oracle.

The production RNGs are intentionally different: Rust uses
VCKSS-COUNTER-V1, while the public Mata command uses registered Stata mt64s
streams.  This test therefore never compares equal seeds from those two RNGs.
Instead, the five-probe Counter-V1 sign sums below are frozen for the
independent 13-copy oracle in rust/crates/vckss-core/src/engine.rs and are
injected into Mata through vckss_scale_eng__run_atoms().

The public Mata exact call is used only to compare graph retention and the
nonrandom plug-in row.  It is not a correction parity claim.
*/

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

quietly do `"`package_dir'/varcomp_kss.mata"'
quietly do `"`package_dir'/varcomp_kss_scale.mata"'
quietly do `"`package_dir'/varcomp_kss_rng.mata"'
quietly do `"`package_dir'/varcomp_kss_scale_engine.mata"'

/* The first component has 8 stored rows and 13 literal copies.  The second
   component has 4 rows/copies.  Permutation makes retained-mask alignment an
   observable contract rather than an accidental prefix. */
input long source_row double(worker firm deletion outcome frequency target_weight permkey)
1  1 1 11  1 1 1  5
2  1 1 12  3 2 2 10
3  1 2 21  0 1 2  2
4  1 2 22  2 2 2  7
5  2 1 31 -1 1 3 12
6  2 1 32  1 3 9  4
7  2 2 41  2 2 8 11
8  2 2 42 -2 1 4  1
9  9 9 91 10 1 1  3
10 9 10 92 11 1 1  6
11 10 9 93 12 1 1  9
12 10 10 94 13 1 1  8
end
sort permkey
local caller_rng_state `"`c(rngstate)'"'

varcomp_kss_rust clear
varcomp_kss_rust probe
assert r(core_ready_flags) == 255
assert r(support_flags) == 38

varcomp_kss_rust prepare worker firm deletion outcome frequency          ///
    target_weight, cleanup generate(rust_keep)
local handle = r(handle)
assert `handle' > 0
assert r(input_rows) == 12
assert r(retained_rows) == 8
assert r(workers) == 2
assert r(firms) == 2
assert r(cells) == 4
assert r(deletion_units) == 8
assert r(target_strata) == 5
assert rust_keep == (source_row <= 8)

/* Compare the original-row mask against the public Mata graph path. */
varcomp_kss outcome [fw=frequency], worker(worker) firm(firm)             ///
    deletion(match) deletionid(deletion) algorithm(exact)                ///
    targetweight(target_weight) nodisplay
assert e(N) == 13
assert e(N_retained) == 8
quietly count if rust_keep != e(sample)
assert r(N) == 0
matrix mata_public = e(results)

/* Run the retained Rust lifecycle with the production-candidate diagonal
   route.  Internal route code 2 is diagonal PCG, not public algorithm 2. */
varcomp_kss_rust solve `handle', seed(8675309) probes(5)                  ///
    leveragebatch(2) targetbatch(2) route(diagonal)                      ///
    tolerance(1e-13) maxiter(500)
varcomp_kss_rust result `handle'
matrix rust_result = r(result)
local rust_rows : rownames rust_result
local rust_columns : colnames rust_result
assert "`rust_rows'" ==                                             ///
    "plugin bias_correction corrected numerical_mcse"
assert "`rust_columns'" ==                                          ///
    "worker_variance firm_variance worker_firm_covariance total_variance"
assert r(seed) == 8675309
assert r(probes) == 5
assert r(requested_route) == 2
assert r(selected_route) == 2
assert r(solver_fallback) == 0
assert r(leverage_batch_width) == 2
assert r(target_batch_width) == 2
assert r(leverage_rhs_count) == 5
assert r(target_rhs_count) == 10
assert r(full_fit_complete_residual) <= r(full_residual_tolerance)
assert r(max_complete_residual) <= r(full_residual_tolerance)
assert r(accounting_residual) <= 1e-12
scalar rust_max_complete = r(max_complete_residual)
scalar rust_max_leverage = r(max_leverage)
forvalues component = 1/4 {
    /* The plug-in row contains no estimator RNG. */
    assert abs(rust_result[1,`component'] -                         ///
        mata_public[1,`component']) < 3e-12
}

mata:
void vckss_rust_mata_shared__run()
{
    struct vckss_scale_design scalar design
    struct vckss_scale_engine_result scalar out
    real colvector keep, worker, firm, deletion, outcome, frequency, target
    real colvector expected_vector
    real matrix leverage_atoms, target_atoms
    real matrix expected_components, expected_draws

    keep = st_data(., "rust_keep")
    worker = select(st_data(., "worker"),keep)
    firm = select(st_data(., "firm"),keep)
    deletion = select(st_data(., "deletion"),keep)
    outcome = select(st_data(., "outcome"),keep)
    frequency = select(st_data(., "frequency"),keep)
    target = select(st_data(., "target_weight"),keep)
    design = vckss_scale__prepare(
        worker,firm,deletion,frequency,outcome,target,1e-10)
    assert(design.status == "CONVERGED")
    assert(design.n_rows == 8 & design.n_physical == 13)
    assert(design.worker_levels == 2 & design.firm_levels == 2)
    assert(design.coefficient_cells == 4 & design.deletion_units == 8)
    assert(design.strata.count == 5 & design.target_weight_sum == 31)

    /* Freeze the exact coefficient-cell, deletion-unit, and target-stratum
       maps.  In particular, target mass is stored-row mass, not target*freq. */
    assert(max(abs(design.worker_key-(1\2))) == 0)
    assert(max(abs(design.firm_key-(1\2))) == 0)
    assert(max(abs(design.deletion_key-
        (11\12\21\22\31\32\41\42))) == 0)
    assert(max(abs(design.cell_worker-(1\1\2\2))) == 0)
    assert(max(abs(design.cell_firm-(1\2\1\2))) == 0)
    assert(max(abs(design.unit_cell-(1\1\2\2\3\3\4\4))) == 0)
    assert(max(abs(design.unit_frequency-(1\2\1\2\1\3\2\1))) == 0)
    assert(max(abs(design.strata.cell-(1\2\2\3\4))) == 0)
    assert(max(abs(design.strata.per_copy_mass-(1\1\2\3\4))) == 0)
    assert(max(abs(design.strata.physical_count-(3\2\1\4\3))) == 0)

    /* Counter-V1 leverage entity keys 1..8, trials (1,2,1,2,1,3,2,1),
       probes 0..4. */
    leverage_atoms = (
         1, 1,-1,-1, 1 \
         0, 0, 2, 0,-2 \
         1,-1, 1, 1,-1 \
         0, 0, 2, 0, 0 \
        -1, 1,-1,-1,-1 \
         3, 1,-1, 3,-1 \
        -2, 0, 0, 0, 2 \
         1,-1, 1, 1, 1)

    /* Counter-V1 target entity keys (1,4,3,5,7), trials (3,2,1,4,3),
       probes 0..4.  This is an independent RNG domain. */
    target_atoms = (
        -1,-1,-1,-1, 1 \
         0, 0, 0, 0, 0 \
         1,-1, 1,-1,-1 \
         2, 0, 0, 0, 4 \
        -1, 1, 1,-1, 1)

    assert(vckss_scale_eng__valid_atoms(
        leverage_atoms,design.unit_frequency))
    assert(vckss_scale_eng__valid_atoms(
        target_atoms,design.strata.physical_count))
    out = vckss_scale_eng__run_atoms(
        design,vckss__diagonal_backend(),leverage_atoms,target_atoms,
        2,2,1e-13,500,1e-10,1e-10)
    assert(out.status == "CONVERGED")
    assert(out.rng_contract == "TEST_MATRIX_ATOMS")
    assert(out.solver_route == "DIAGONAL")
    assert(rows(out.solver_rhs_diagnostics) == 16)
    assert(max(out.solver_rhs_diagnostics[.,5]) <= 1e-11)
    assert(min(out.solver_rhs_diagnostics[.,6]) == 1)

    expected_vector = (2.022222222222222\1.6444444444444444\
        .7333333333333334\.3555555555555555)
    assert(max(abs(out.fitted_cell-expected_vector)) < 3e-12)
    expected_vector = (.15226280407512646\.51045539033457255\
        .28895920442987899\.8789962186318323\.12774613506916191\
        .42547425474254735\.3729365768896612\.24567176992416653)
    assert(max(abs(out.unit_projection_share-expected_vector)) < 3e-12)
    expected_vector = (.84773719592487362\.48954460966542745\
        .7110407955701209\.12100378136816775\.87225386493083812\
        .57452574525745259\.6270634231103388\.75432823007583338)
    assert(max(abs(out.unit_residual_share-expected_vector)) < 3e-12)

    /* Coefficient-one finite-projection B and V. */
    expected_vector = (-.010459834917505805\.00085659124634756699\
        .0046906552413213442\.017757832298514471\
        -.014775372931922968\-.029847395362842533\
        -.014143015377574683\-.0089889346397955209)
    assert(max(abs(out.unit_finite_bias-expected_vector)) < 3e-12)
    expected_vector = (.0041094298625406946\.015803408041061603\
        .03213073067015533\.0046171514591239237\
        .0033317061705819663\.021332842411230485\
        .018713182354403637\.0072736191491370754)
    assert(max(abs(out.unit_finite_variance-expected_vector)) < 3e-12)
    expected_vector = (-1.0222222222222221\1.9555555555555557\
        -1.6444444444444444\.71111111111111125\
        -1.7333333333333334\.79999999999999982\
        3.2888888888888888\-2.3555555555555556)
    assert(max(abs(out.unit_residual_mass-expected_vector)) < 3e-12)
    expected_vector = (-1.1840511832724148\3.7382147311509284\
        -2.1810061676192767\4.8860410885733865\
        -1.9448254362996038\1.2301196977370579\
        4.8770005035819386\-3.045590340676231)
    assert(max(abs(out.unit_d-expected_vector)) < 3e-12)
    expected_vector = (21.245237203633152\19.544164354293546\
        5.6351845295107772\25.599182695680216)
    assert(max(abs(out.cell_correction_weight-expected_vector)) < 3e-12)

    expected_draws = (
        .000005919592148325,.14795414857041678,-.000359573162336864,.14724092183789136 \
        .3238274440379454,.03910659880532012,.04323740813370915,.44940885911068384 \
        .001030790584914836,.2875533494713059,.006614884114337031,.3018139082848948 \
        .1208096286496324,.07892603991118426,-.03751791839019003,.12469983178043659 \
        .32832100642234874,.8872532024075648,-.20737238677720882,.8008294352754959)
    assert(max(abs(out.target_draws-expected_draws)) < 3e-12)

    expected_components = (
        .2904135352834625,.03564188538173971,-.006080086329826184,.31389524800554985 \
        .15479895785739795,.28815866783315836,-.03907951721633791,.3647985912578805 \
        .13561457742606453,-.2525167824514186,.03299943088651172,-.050903343252330646 \
        .07329438550816288,.15562409207174877,.04398193783561902,.12373916487436575)
    assert(max(abs((out.plugin\out.correction\out.corrected\
        out.numerical_mcse)-expected_components)) < 3e-12)
    assert(abs(st_numscalar("rust_max_leverage")-
        out.max_leverage) < 3e-12)
    assert(out.target_identity_residual <= 1e-12)
    st_numscalar("mata_max_complete",out.max_complete_residual)
    st_matrix("mata_result",out.plugin\out.correction\out.corrected\
        out.numerical_mcse)
}

vckss_rust_mata_shared__run()
mata drop vckss_rust_mata_shared__run()
end

/* This is the actual shared-atom component differential. */
mata: assert(max(abs(st_matrix("rust_result")-st_matrix("mata_result"))) < 3e-12)
assert rust_max_complete <= 1e-11
assert mata_max_complete <= 1e-11

varcomp_kss_rust release `handle'
varcomp_kss_rust snapshot
assert r(state) == 0
assert `"`c(rngstate)'"' == `"`caller_rng_state'"'

di as result "PASS test_rust_mata_shared_atoms.do"
exit 0
