version 18.0

/* Independent quad oracle for the bounded panel kernel. The panels include
   unequal widths, a subset/permutation of input rows, and severe cancellation. */
mata:
void fevc_test_nmc_reductions()
{
    real matrix values, panel, reference, actual, gradients, moments, plan
    real colvector row_order, index, p, m
    real scalar g, n
    struct vckss_nmc__state scalar state
    struct vckss_nmc__finite scalar finite
    n = 70000
    values = J(3*n,2,0)
    values[1..n,.] = J(n,1,1e16),J(n,1,-1e16)
    values[n+1..2*n,.] = J(n,1,1),J(n,1,-3)
    values[2*n+1..3*n,.] = J(n,1,-1e16),J(n,1,1e16)
    row_order = vec(((1..n)',(n+1..2*n)',(2*n+1..3*n)')')
    panel = ((1..3*n)'[selectindex(mod((1..3*n)',3):==1)],
             (1..3*n)'[selectindex(mod((1..3*n)',3):==0)])
    actual = vckss_nmc__panelsum(values,panel,row_order)
    assert(all(actual[.,1]:==1) & all(actual[.,2]:==-3))
    plan = vckss_nmc__panel_plan(panel,row_order)
    actual = vckss_nmc__planned_sum(values,panel,plan)
    assert(all(actual[.,1]:==1) & all(actual[.,2]:==-3))
    values = (1e16,2\1,3\-1e16,4\7,5\-2,6\3,7\9,8)
    row_order = (4\2\5\1\3\6)
    panel = (1,1\2,3\4,6)
    reference = J(rows(panel),2,.)
    for (g=1; g<=rows(panel); g++) {
        index = row_order[|panel[g,1]\panel[g,2]|]
        reference[g,.] = quadcolsum(values[index,.],1)
    }
    assert(all(vckss_nmc__panelsum(values,panel,row_order):==reference))
    plan = vckss_nmc__panel_plan(panel,(1..rows(row_order))')
    assert(all(vckss_nmc__planned_sum(values[row_order,.],panel,plan):==reference))
    // A few very wide panels use quad reduction without an offset-length loop.
    values = J(20003,1,0)
    values[(1\2\20001\20002\20003)] = (1e16\1\-1e16\2\3)
    panel = (1,20001\20002,20003)
    assert(all(vckss_nmc__panelsum(values,panel):==(1\5)))
    plan = vckss_nmc__panel_plan(panel,(1..rows(values))')
    assert(cols(plan)==0)
    assert(all(vckss_nmc__planned_sum(values,panel,plan):==(1\5)))
    values[2] = .
    assert(hasmissing(vckss_nmc__panelsum(values,panel)))
    // Five moment terms must retain cancellation and propagate nonfiniteness.
    gradients = (1e16,1,-1e16,0,0,0\1,-2,3,-4,5,6)
    p = (1\.4); m = (1\-.7)
    moments = (p:^2,m:^2,p:^4,m:^4,p:^2:*m:^2)
    reference = J(2,1,.)
    for (g=1; g<=2; g++) reference[g] =
        quadsum(gradients[g,1..5]:*moments[g,.],1)-gradients[g,6]
    assert(all(vckss_nmc__block_score(gradients,p,m):==reference))
    gradients[2,3] = .
    assert(missing(vckss_nmc__block_score(gradients,p,m)[2]))
    finite = vckss_nmc__finite_derivative(J(0,5,.),33)
    assert(finite.status=="nonfinite_derivative")
    // A nonsmooth copy refuses batching and retains the scalar status/margins.
    state = vckss_nmc__new(2,3,0,33)
    vckss_nmc__observation_prepare(state,(1\2),(1,2\3,3),(1\1\2),
        33:*(.2\.3),33:*(-.2\.1),J(3,1,0),J(3,1,0),J(2,1,0))
    assert(state.status=="nonsmooth_adjustment")
    assert(abs(state.minimum_constrained-1.4)<1e-15)
    assert(abs(state.minimum_margin-1.3/1.6)<1e-15)
    assert(all(state.copy[1..2,.]:==0) & !hasmissing(state.copy[3,.]))
    // A quad crossproduct must not silently omit an overflowing influence.
    state = vckss_nmc__new(1,1,0,33)
    state.observation_index = state.physical_row = 1
    state.row = (1,1,0); state.copy = (1,1)
    state.observation_fold = state.copy_fold = J(1,6,1)
    assert(hasmissing(vckss_nmc__score(state,1e100,1,1,J(0,1,.),J(0,2,.))))
}
fevc_test_nmc_reductions()
end

di as result "PASS test_all_probe_reductions.do"
