version 18.0
clear all
set more off
set varabbrev off

local oldpwd `"`c(pwd)'"'
capture confirm file "kss_bc/kss_bc_scale_engine.mata"
if _rc {
    capture confirm file "../../kss_bc_scale_engine.mata"
    if _rc {
        di as error "run from the repository root or kss_bc/tests/stata"
        exit 601
    }
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/kss_bc"'

quietly do `"`pkgroot'/kss_bc.mata"'
quietly do `"`pkgroot'/kss_bc_scale.mata"'
quietly do `"`pkgroot'/kss_bc_rng.mata"'
quietly do `"`pkgroot'/kss_bc_scale_engine.mata"'

mata:
real scalar kssered__reldif(real matrix left, real matrix right)
{
    return(max(abs(left-right))/max((1,max(abs(left)),max(abs(right)))))
}

real scalar kssered__median3(real colvector values)
{
    real colvector ordered

    ordered = sort(values,1)
    return(ordered[2])
}

/* Scalar reference for the retired O(G*P*5) engine loop.  Each group keeps
   the same logical probe order and uses the same Neumaier recurrence. */
real matrix kssered__legacy_moments(
    real matrix projection,
    real matrix residual)
{
    real scalar column, correction, group, moment, one, subtotal, updated
    real rowvector values
    real matrix compensation, out

    out = J(rows(projection),5,0)
    compensation = J(rows(projection),5,0)
    for (column=1; column<=cols(projection); column++) {
        for (group=1; group<=rows(projection); group++) {
            values = (projection[group,column]^2,
                residual[group,column]^2,
                projection[group,column]^4,
                residual[group,column]^4,
                projection[group,column]^2*residual[group,column]^2)
            for (moment=1; moment<=5; moment++) {
                subtotal = out[group,moment]
                one = values[moment]
                updated = subtotal+one
                if (abs(subtotal) >= abs(one)) {
                    correction = (subtotal-updated)+one
                }
                else correction = (one-updated)+subtotal
                out[group,moment] = updated
                compensation[group,moment] =
                    compensation[group,moment]+correction
            }
        }
    }
    return(out+compensation)
}

/* Scalar reference for the retired three-dot target loop. */
real matrix kssered__legacy_target(
    real colvector cell_weight,
    real matrix prediction)
{
    real scalar batch, batches
    real colvector firm, total, worker
    real matrix out

    batches = cols(prediction)/2
    out = J(batches,4,.)
    for (batch=1; batch<=batches; batch++) {
        worker = prediction[.,2*batch-1]
        firm = prediction[.,2*batch]
        total = worker+firm
        out[batch,1] = kssbc_scale_engine__dot(
            cell_weight,worker:^2)
        out[batch,2] = kssbc_scale_engine__dot(
            cell_weight,firm:^2)
        out[batch,4] = kssbc_scale_engine__dot(
            cell_weight,total:^2)
        out[batch,3] = .5*(out[batch,4]-out[batch,1]-out[batch,2])
    }
    return(out)
}

/* Scalar reference for the retired O(rows*columns) atom validator. */
real scalar kssered__legacy_valid_atoms(
    real matrix atoms,
    real colvector trials)
{
    real scalar column, row, value

    if (rows(atoms) == 0 | cols(atoms) == 0 |
        rows(trials) != rows(atoms) | cols(trials) != 1 |
        hasmissing(atoms) | hasmissing(trials) | min(trials) < 1 |
        max(abs(trials-floor(trials))) != 0) return(0)
    for (column=1; column<=cols(atoms); column++) {
        for (row=1; row<=rows(atoms); row++) {
            value = atoms[row,column]
            if (value != floor(value) | abs(value) > trials[row] |
                mod(value+trials[row],2) != 0) return(0)
        }
    }
    return(1)
}

void kssered__run()
{
    struct kssbc_maker_result scalar generic_maker
    struct kssbc_result scalar ordinary
    struct kssbc_scale_design scalar design
    struct kssbc_scale_engine_result scalar timed
    struct kssbc_scale_unit_adjust scalar adjustment
    struct kssbc_solver_backend scalar backend
    real scalar batches, boundary_case, cells, first, groups, probes
    real scalar repetition
    real scalar scalar_moment_time, scalar_target_time
    real scalar scalar_atom_time, vector_atom_time
    real scalar vector_moment_time, vector_target_time
    real matrix atoms, compensation, legacy, moments, partitioned
    real matrix prediction, target_atoms, unit_atoms
    real matrix projection, residual, subtotal, target_legacy, target_new
    real matrix timing, whole
    real colvector cell_index, cell_weight, common_direction, group_index
    real colvector deletion, firm, fixture_target, frequency, maker_rhs
    real colvector outcome, trials, worker
    real rowvector probe_index, residual_targets
    real scalar block_tolerance, projection_share, rank_tolerance

    assert(kssbc_scale_eng__moment_width() == 8)
    assert(kssbc_scale_eng__target_rows() == 4096)
    assert(kssbc_scale_eng__atom_rows() == 4096)
    assert(kssbc_scale_eng__roundoff_gate() ==
        4096*2.2204460492503131e-16)

    /* Equality, tiling, and solver-batch partition invariance. */
    groups = 137
    probes = 37
    group_index = 1::groups
    probe_index = 1..probes
    projection = sin(group_index*probe_index:/97) :*
        (1:+mod(group_index,11):/17)
    residual = cos(group_index*(probe_index:+3):/89) :-
        projection:/7
    legacy = kssered__legacy_moments(projection,residual)
    whole = kssbc_scale_eng__moment_reduce(projection,residual)
    assert(rows(whole) == groups & cols(whole) == 5)
    assert(kssered__reldif(whole,legacy) <=
        4*kssbc_scale_eng__roundoff_gate())

    subtotal = J(groups,5,0)
    compensation = J(groups,5,0)
    for (first=1; first<=probes; first=first+11) {
        moments = kssbc_scale_eng__moment_reduce(
            projection[.,first..min((probes,first+10))],
            residual[.,first..min((probes,first+10))])
        kssbc_scale_eng__merge(&subtotal,&compensation,moments)
    }
    partitioned = subtotal+compensation
    assert(kssered__reldif(partitioned,legacy) <=
        4*kssbc_scale_eng__roundoff_gate())

    /* Fused worker/firm/cross contractions reproduce the retired three-dot
       algebra while deriving total from one accounting identity. */
    cells = 10037
    batches = 19
    cell_index = 1::cells
    cell_weight = sin(cell_index:/113):*(1:+mod(cell_index,7):/5)
    prediction = J(cells,2*batches,.)
    for (first=1; first<=batches; first++) {
        prediction[.,2*first-1] =
            sin(cell_index*(first+2):/701):+cell_index:/cells
        prediction[.,2*first] =
            cos(cell_index*(first+5):/809):-cell_index:/(3*cells)
    }
    target_legacy = kssered__legacy_target(cell_weight,prediction)
    target_new = kssbc_scale_eng__target_contract(
        cell_weight,prediction)
    assert(rows(target_new) == batches & cols(target_new) == 4)
    assert(kssered__reldif(target_new,target_legacy) <=
        8*kssbc_scale_eng__roundoff_gate())
    assert(kssbc_scale_eng__identity_resid(target_new) <=
        kssbc_scale_eng__roundoff_gate())

    /* Matrix/tiled atom validation preserves literal sign-sum support. */
    trials = 1:+mod(group_index,31)
    atoms = J(groups,probes,.)
    for (first=1; first<=probes; first++) {
        atoms[.,first] = 2:*mod(group_index:*first,trials):-trials
    }
    assert(kssered__legacy_valid_atoms(atoms,trials))
    assert(kssbc_scale_eng__valid_atoms(atoms,trials))
    atoms[1,1] = trials[1]+2
    assert(!kssered__legacy_valid_atoms(atoms,trials))
    assert(!kssbc_scale_eng__valid_atoms(atoms,trials))
    atoms[1,1] = -trials[1]
    atoms[2,2] = atoms[2,2]+1
    assert(!kssered__legacy_valid_atoms(atoms,trials))
    assert(!kssbc_scale_eng__valid_atoms(atoms,trials))
    atoms[2,2] = -trials[2]
    atoms[3,3] = atoms[3,3]+.5
    assert(!kssered__legacy_valid_atoms(atoms,trials))
    assert(!kssbc_scale_eng__valid_atoms(atoms,trials))
    atoms[3,3] = -trials[3]
    atoms[4,4] = .
    assert(!kssered__legacy_valid_atoms(atoms,trials))
    assert(!kssbc_scale_eng__valid_atoms(atoms,trials))

    /* Aggregate and identity acceptance are fixed roundoff contracts.  They
       no longer inherit the caller's statistical rank tolerance. */
    assert(kssbc_scale_eng__aggregate_ok(
        (1e8+1e-6\-3), (1e8\-3), (1e8\7)))
    assert(!kssbc_scale_eng__aggregate_ok(
        (1e8+.01\-3), (1e8\-3), (1e8\7)))

    /* The scalar specialization inherits the generic no-control match
       conditioning gates.  Tiny negative finite-variance estimates are
       clipped at the existing -100*rank_tolerance boundary; material
       negatives retain the generic typed failure. */
    rank_tolerance = 1e-10
    block_tolerance = 1e-8
    adjustment = kssbc_scale_eng__unit_adjust(
        .4,.6,0,-50*rank_tolerance,1,
        rank_tolerance,block_tolerance)
    assert(adjustment.status == "CONVERGED")
    assert(adjustment.finite_variance == 0)
    adjustment = kssbc_scale_eng__unit_adjust(
        .4,.6,0,-101*rank_tolerance,1,
        rank_tolerance,block_tolerance)
    assert(adjustment.status == "JLA_MOMENT_FAILED")

    /* Compare both sides of the block-tolerance boundary to the generic
       low-rank maker.  The values stay away from the exact floating-point
       boundary so the regression tests semantics rather than eigensolver
       rounding at one representable number. */
    common_direction = J(3,1,1/sqrt(3))
    maker_rhs = (.2\-.3\.4)
    residual_targets = (.5*block_tolerance,2*block_tolerance)
    for (boundary_case=1; boundary_case<=cols(residual_targets);
        boundary_case++) {
        projection_share = 1-residual_targets[boundary_case]
        generic_maker = kssbc__low_rank_maker(
            sqrt(projection_share):*common_direction,
            maker_rhs,rank_tolerance,block_tolerance)
        adjustment = kssbc_scale_eng__unit_adjust(
            projection_share,residual_targets[boundary_case],0,0,1,
            rank_tolerance,block_tolerance)
        assert(adjustment.status == generic_maker.status)
        if (generic_maker.status == "CONVERGED") {
            assert(generic_maker.relres <=
                max((1e-10,100*rank_tolerance)))
            assert(adjustment.reciprocal_residual <=
                max((1e-10,100*rank_tolerance)))
        }
    }

    /* Nested stage attribution remains diagnostic: RNG and correction
       kernels are already contained in leverage/target and therefore are
       neither added to total_seconds nor conflated with solver time. */
    worker =   (1\1\1\1\1\2\2\2\3\3\3\3)
    firm =     (1\1\1\2\2\1\2\2\1\1\2\2)
    deletion = (101\101\102\103\103\104\105\105\106\106\107\108)
    frequency =(2\1\3\2\1\2\2\1\1\2\3\1)
    outcome =  (1.2\.8\1.5\2.1\1.9\-.4\.3\.7\1.1\.9\-.2\.2)
    fixture_target = (2\2\3\2\.5\2\4\1\1\2\3\1)
    design = kssbc_scale__prepare(
        worker,firm,deletion,frequency,outcome,fixture_target,1e-12)
    assert(design.status == "CONVERGED")
    probes = 16
    group_index = 1::design.deletion_units
    unit_atoms = J(design.deletion_units,probes,.)
    for (first=1; first<=probes; first++) {
        unit_atoms[.,first] = 2:*mod(
            group_index:*first,design.unit_frequency:+1):-
            design.unit_frequency
    }
    group_index = 1::design.strata.count
    target_atoms = J(design.strata.count,probes,.)
    for (first=1; first<=probes; first++) {
        target_atoms[.,first] = 2:*mod(
            group_index:*first,design.strata.physical_count:+1):-
            design.strata.physical_count
    }
    backend = kssbc__diagonal_backend()
    timed = kssbc_scale_eng__run_atoms(
        design,backend,unit_atoms,target_atoms,5,7,
        1e-12,10000,1e-12,1e-10)
    assert(timed.status == "CONVERGED")
    assert(timed.rng_seconds >= 0)
    assert(timed.correction_seconds >= 0)
    assert(timed.rng_seconds <=
        timed.leverage_seconds+timed.target_seconds)
    assert(timed.correction_seconds <=
        timed.leverage_seconds+timed.target_seconds)
    assert(timed.total_seconds ==
        timed.fit_seconds+timed.leverage_seconds+timed.target_seconds)
    ordinary = kssbc_scale_eng__as_result(timed,.25)
    assert(ordinary.status == "CONVERGED")
    assert(ordinary.correction_seconds == timed.correction_seconds)
    st_matrix("scale_nested_timing",(
        timed.fit_seconds,timed.rng_seconds,timed.leverage_seconds,
        timed.correction_seconds,timed.target_seconds,timed.total_seconds))

    /* Production-shaped helper timings.  The output and extra tile widths
       stay bounded as P and C grow: Gx5 moments, at most Gx8 moment-square
       tiles, Bx4 contractions, and at most 4096xB target tiles. */
    groups = 12000
    probes = 64
    group_index = 1::groups
    probe_index = 1..probes
    projection = sin(group_index*probe_index:/1009)
    residual = cos(group_index*(probe_index:+7):/1013):-projection:/9

    /* Warm both compiled paths before paired measurements. */
    legacy = kssered__legacy_moments(
        projection[1..20,.],residual[1..20,.])
    moments = kssbc_scale_eng__moment_reduce(
        projection[1..20,.],residual[1..20,.])
    timing = J(3,6,.)
    for (repetition=1; repetition<=3; repetition++) {
        timer_clear(81)
        timer_on(81)
        legacy = kssered__legacy_moments(projection,residual)
        timer_off(81)
        timing[repetition,1] = timer_value(81)[1]

        timer_clear(82)
        timer_on(82)
        moments = kssbc_scale_eng__moment_reduce(projection,residual)
        timer_off(82)
        timing[repetition,2] = timer_value(82)[1]
        assert(kssered__reldif(moments,legacy) <=
            4*kssbc_scale_eng__roundoff_gate())
    }

    trials = 1:+mod(group_index,101)
    atoms = J(groups,probes,.)
    for (first=1; first<=probes; first++) {
        atoms[.,first] = 2:*mod(group_index:*first,trials):-trials
    }
    for (repetition=1; repetition<=3; repetition++) {
        timer_clear(85)
        timer_on(85)
        assert(kssered__legacy_valid_atoms(atoms,trials))
        timer_off(85)
        timing[repetition,5] = timer_value(85)[1]

        timer_clear(86)
        timer_on(86)
        assert(kssbc_scale_eng__valid_atoms(atoms,trials))
        timer_off(86)
        timing[repetition,6] = timer_value(86)[1]
    }

    cells = 24000
    batches = 24
    cell_index = 1::cells
    cell_weight = cos(cell_index:/127):*(1:+mod(cell_index,13):/11)
    prediction = J(cells,2*batches,.)
    for (first=1; first<=batches; first++) {
        prediction[.,2*first-1] =
            sin(cell_index*(first+1):/811)
        prediction[.,2*first] =
            cos(cell_index*(first+3):/821)
    }
    target_legacy = kssered__legacy_target(
        cell_weight[1..20],prediction[1..20,.])
    target_new = kssbc_scale_eng__target_contract(
        cell_weight[1..20],prediction[1..20,.])
    for (repetition=1; repetition<=3; repetition++) {
        timer_clear(83)
        timer_on(83)
        target_legacy = kssered__legacy_target(cell_weight,prediction)
        timer_off(83)
        timing[repetition,3] = timer_value(83)[1]

        timer_clear(84)
        timer_on(84)
        target_new = kssbc_scale_eng__target_contract(
            cell_weight,prediction)
        timer_off(84)
        timing[repetition,4] = timer_value(84)[1]
        assert(kssered__reldif(target_new,target_legacy) <=
            8*kssbc_scale_eng__roundoff_gate())
    }
    scalar_moment_time = kssered__median3(timing[.,1])
    vector_moment_time = kssered__median3(timing[.,2])
    scalar_target_time = kssered__median3(timing[.,3])
    vector_target_time = kssered__median3(timing[.,4])
    scalar_atom_time = kssered__median3(timing[.,5])
    vector_atom_time = kssered__median3(timing[.,6])
    assert(vector_moment_time < scalar_moment_time)
    assert(vector_target_time < scalar_target_time)
    assert(vector_atom_time < scalar_atom_time)
    assert(rows(moments) == groups & cols(moments) == 5)
    assert(rows(target_new) == batches & cols(target_new) == 4)
    st_matrix("scale_reduction_timing",timing)
    st_matrix("scale_reduction_shape",(
        groups,probes,kssbc_scale_eng__moment_width(),rows(moments) \
        cells,batches,kssbc_scale_eng__target_rows(),rows(target_new) \
        groups,probes,kssbc_scale_eng__atom_rows(),1))
}

kssered__run()
mata drop kssered__*()
end

matrix colnames scale_reduction_timing = moment_scalar moment_vector ///
    target_scalar target_vector atom_scalar atom_vector
matrix list scale_reduction_timing
matrix colnames scale_reduction_shape = leading_dimension width ///
    tile_limit output_rows
matrix rownames scale_reduction_shape = leverage target atom_validation
matrix list scale_reduction_shape

matrix colnames scale_nested_timing = fit rng leverage correction target total
matrix list scale_nested_timing

quietly cd `"`oldpwd'"'
di as result "PASS test_scale_engine_reductions.do"
exit 0
