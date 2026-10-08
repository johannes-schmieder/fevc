version 18.0
clear all
set more off
args package_dir
if `"`package_dir'"'=="" exit 198
adopath ++ `"`package_dir'"'
// Frozen Counter-V1 atoms from the independent 13-copy fixture; equal
// numeric seeds from the two production RNGs are never a parity oracle.
// The directly injected Mata atoms use the ordinary uncentered formula.
input long(source_row worker firm deletion) double(outcome frequency target)
1 1 1 11  1 1 1
2 1 1 12  3 2 2
3 1 2 21  0 1 2
4 1 2 22  2 2 2
5 2 1 31 -1 1 3
6 2 1 32  1 3 9
7 2 2 41  2 2 8
8 2 2 42 -2 1 4
end
local state `"`c(rngstate)'"'
quietly fevc outcome [fw=frequency], centering(none) worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion) targetweight(target) probeorder(source_row) ///
    stayers(movers) backend(rust) engine(auto) algorithm(jla) preconditioner(diagonal) ///
    probes(5) batch(2) seed(8675309) tolerance(1e-13) numericalmcse(all) nodisplay
matrix native_point=e(kss)
matrix native_cond=e(numerical_mccov_cond)
matrix native_leverage=e(numerical_mccov_leverage)
matrix native_raw=e(numerical_mccov_all_raw)
local status `e(numerical_mc_status)'
assert e(mc_replay_rhs)==5 & e(mc_replay_generator_work)==40
assert `"`c(rngstate)'"'==`"`state'"'
// The native preflight loads only numerical validation. The independent
// Mata comparison also needs the full solver structures.
quietly do `"`package_dir'/fevc.mata"'
quietly do `"`package_dir'/fevc_scale.mata"'
quietly do `"`package_dir'/fevc_scale_engine.mata"'
mata:
void fevc_test_nmc_shared_atoms()
{
    struct vckss_scale_design scalar design
    struct vckss_fe_design scalar base
    struct vckss_scale_engine_matrix_atoms scalar source
    struct vckss_scale_atom_provider scalar provider
    struct vckss_scale_engine_result scalar point
    struct vckss_nmc__attachment scalar attachment
    design=vckss_scale__prepare(st_data(.,"worker"),st_data(.,"firm"),
        st_data(.,"deletion"),st_data(.,"frequency"),st_data(.,"outcome"),
        st_data(.,"target"),1e-10)
    assert(design.status=="CONVERGED")
    source.leverage_unit=(1,1,-1,-1,1\0,0,2,0,-2\1,-1,1,1,-1\0,0,2,0,0\
        -1,1,-1,-1,-1\3,1,-1,3,-1\-2,0,0,0,2\1,-1,1,1,1)
    source.target_stratum=(-1,-1,-1,-1,1\0,0,0,0,0\1,-1,1,-1,-1\
        2,0,0,0,4\-1,1,1,-1,1)
    provider=vckss_scale_eng__mat_provider(&source)
    base=vckss__fe_prepare(design.cell_worker,design.cell_firm,design.cell_frequency,1e-10)
    point=vckss_scale_eng__run_prepared(design,base,vckss__diagonal_backend(),
        provider,5,2,2,1e-13,500,1e-10,1e-10,&attachment)
    assert(point.status=="CONVERGED" & rows(attachment.replay_rhs)==5)
    assert(vckss_nmc__relative_equal(point.corrected,st_matrix("native_point")))
    assert(vckss_nmc__relative_equal(attachment.covariance.conditional,st_matrix("native_cond")))
    assert(vckss_nmc__relative_equal(attachment.covariance.leverage,st_matrix("native_leverage")))
    assert(vckss_nmc__relative_equal(attachment.covariance.raw,st_matrix("native_raw")))
    assert(attachment.covariance.status==st_local("status"))
    assert(max(attachment.replay_rhs[.,3])<=1e-11)
}
fevc_test_nmc_shared_atoms()
mata drop fevc_test_nmc_shared_atoms()
end
assert `"`c(rngstate)'"'==`"`state'"'
fevc_rust snapshot
assert r(state)==0
di as result "FEVC ALL PROBE SHARED ATOMS PASS"
