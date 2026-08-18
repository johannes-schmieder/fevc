version 18.0
clear all
set more off
set varabbrev off

// Independent dense and compressed oracles for the no-control match path.
// These helpers intentionally do not call the production KSS implementation.
mata:
real scalar kssscale_oracle__norm2(real matrix value)
{
    return(sqrt(sum(value:^2)))
}

real scalar kssscale_oracle__stable_sum(real colvector value)
{
    real scalar row, total, correction, updated, one

    total = 0
    correction = 0
    for (row=1; row<=rows(value); row++) {
        updated = total+value[row]
        if (abs(total) >= abs(value[row])) {
            one = (total-updated)+value[row]
        }
        else one = (value[row]-updated)+total
        correction = correction+one
        total = updated
    }
    return(total+correction)
}

real scalar kssscale_oracle__stable_dot(
    real colvector left,
    real colvector right)
{
    return(kssscale_oracle__stable_sum(left:*right))
}

real scalar kssscale_oracle__max_relres(
    real matrix residual,
    real matrix right_hand_side)
{
    real scalar column, one, scale, maximum

    maximum = 0
    for (column=1; column<=cols(right_hand_side); column++) {
        scale = kssscale_oracle__norm2(right_hand_side[.,column])
        one = kssscale_oracle__norm2(residual[.,column])
        if (scale > 0) one = one/scale
        maximum = max((maximum,one))
    }
    return(maximum)
}

real rowvector kssscale_oracle__dense_block(
    real colvector residual,
    real colvector frequency,
    real scalar projection_share,
    real scalar finite_bias,
    real scalar finite_variance,
    real scalar block_tolerance)
{
    real colvector square_root_frequency, common, eigen
    real matrix maker, right_hand_side, actions
    real colvector transformed, inverse_common, deleted_adjusted
    real scalar minimum, common_transformed, common_inverse, relres

    square_root_frequency = sqrt(frequency)
    common = square_root_frequency:/sqrt(
        kssscale_oracle__stable_sum(frequency))
    maker = I(rows(frequency))-
        projection_share:*common*common'
    maker = .5:*(maker+maker')
    eigen = Re(eigenvalues(maker))
    minimum = min(eigen)
    if (minimum <= block_tolerance) return((0,.,.,minimum))
    right_hand_side = square_root_frequency:*residual,common
    actions = invsym(maker)*right_hand_side
    relres = kssscale_oracle__max_relres(
        maker*actions-right_hand_side,right_hand_side)
    transformed = actions[.,1]
    inverse_common = actions[.,2]
    common_transformed = (common'*transformed)[1,1]
    common_inverse = (common'*inverse_common)[1,1]
    deleted_adjusted = transformed+
        finite_bias:*inverse_common:*common_transformed-
        finite_variance:*inverse_common:*common_inverse:*
            common_transformed
    return((1,
        (square_root_frequency'*deleted_adjusted)[1,1],
        relres,minimum))
}

real rowvector kssscale_oracle__compressed(
    real colvector residual,
    real colvector frequency,
    real scalar projection_share,
    real scalar finite_bias,
    real scalar finite_variance,
    real scalar block_tolerance)
{
    real scalar residual_mass, residual_share, multiplier, reciprocal_relres

    residual_mass = kssscale_oracle__stable_dot(frequency,residual)
    residual_share = 1-projection_share
    if (residual_share <= block_tolerance) {
        return((0,.,residual_mass,residual_share,.))
    }
    reciprocal_relres = abs(residual_share*(1/residual_share)-1)
    multiplier = 1/residual_share+
        finite_bias/residual_share^2-
        finite_variance/residual_share^3
    return((1,residual_mass*multiplier,residual_mass,
        residual_share,reciprocal_relres))
}

real matrix kssscale_oracle__literal_expand(
    real colvector residual,
    real colvector frequency,
    real colvector outcome)
{
    real scalar row, begin, finish, total
    real matrix expanded

    total = sum(frequency)
    expanded = J(total,3,.)
    begin = 1
    for (row=1; row<=rows(frequency); row++) {
        finish = begin+frequency[row]-1
        expanded[|begin,1\finish,1|] = J(frequency[row],1,residual[row])
        expanded[|begin,2\finish,2|] = J(frequency[row],1,1)
        expanded[|begin,3\finish,3|] = J(frequency[row],1,outcome[row])
        begin = finish+1
    }
    return(expanded)
}

void kssscale_oracle__assert_one(
    real colvector residual,
    real colvector frequency,
    real scalar projection_share,
    real scalar finite_bias,
    real scalar finite_variance)
{
    real rowvector dense, compressed
    real scalar scale

    dense = kssscale_oracle__dense_block(
        residual,frequency,projection_share,
        finite_bias,finite_variance,1e-10)
    compressed = kssscale_oracle__compressed(
        residual,frequency,projection_share,
        finite_bias,finite_variance,1e-10)
    assert(dense[1] == 1)
    assert(compressed[1] == 1)
    scale = max((1,abs(dense[2]),abs(compressed[2])))
    assert(abs(dense[2]-compressed[2]) <= 3e-12*scale)
    assert(dense[3] < 3e-14)
    assert(compressed[5] < 3e-16)
}

// Several seeded, repeated-row, literal-frequency blocks.
seeds = (7,8675309,20260816,20260823)
for (seed_index=1; seed_index<=cols(seeds); seed_index++) {
    rseed(seeds[seed_index])
    for (group=1; group<=12; group++) {
        rows_in_group = 2+floor(5*runiform(1,1))
        frequency = ceil(5:*runiform(rows_in_group,1))
        residual = rnormal(rows_in_group,1,0,1)
        projection_share = .02+.90*runiform(1,1)
        finite_bias = .004:*(runiform(1,1)-.5)
        finite_variance = .0005:*runiform(1,1)
        kssscale_oracle__assert_one(
            residual,frequency,projection_share,
            finite_bias,finite_variance)
    }
}

// Literal expansion must preserve the deletion-unit contraction.
frequency = (3\1\4)
residual = (.75\-1.25\.375)
outcome = (1\-.5\2.25)
expanded = kssscale_oracle__literal_expand(residual,frequency,outcome)
dense_stored = kssscale_oracle__dense_block(
    residual,frequency,.37,-.004,.0007,1e-10)
dense_expanded = kssscale_oracle__dense_block(
    expanded[.,1],expanded[.,2],.37,-.004,.0007,1e-10)
compressed_stored = kssscale_oracle__compressed(
    residual,frequency,.37,-.004,.0007,1e-10)
compressed_expanded = kssscale_oracle__compressed(
    expanded[.,1],expanded[.,2],.37,-.004,.0007,1e-10)
assert(abs(dense_stored[2]-dense_expanded[2]) < 2e-12)
assert(compressed_stored[2] == compressed_expanded[2])
assert(compressed_stored[3] == compressed_expanded[3])

// Two deletion IDs share coefficient cell 1.  They retain distinct p, B,
// and V values until their Y_g D_g terms are accumulated into K_c.
cell = (1\1\2)
frequency_1 = (2\1)
residual_1 = (1\-.25)
outcome_1 = (2\-1)
frequency_2 = (1\3\2)
residual_2 = (-.5\.75\1.25)
outcome_2 = (.5\3\-2)
frequency_3 = (1\1)
residual_3 = (.2\-.4)
outcome_3 = (1\2)
dense_1 = kssscale_oracle__dense_block(
    residual_1,frequency_1,.21,-.003,.0002,1e-10)
dense_2 = kssscale_oracle__dense_block(
    residual_2,frequency_2,.73,.006,.0008,1e-10)
dense_3 = kssscale_oracle__dense_block(
    residual_3,frequency_3,.12,-.001,.0001,1e-10)
compressed_1 = kssscale_oracle__compressed(
    residual_1,frequency_1,.21,-.003,.0002,1e-10)
compressed_2 = kssscale_oracle__compressed(
    residual_2,frequency_2,.73,.006,.0008,1e-10)
compressed_3 = kssscale_oracle__compressed(
    residual_3,frequency_3,.12,-.001,.0001,1e-10)
y_mass = (
    kssscale_oracle__stable_dot(frequency_1,outcome_1) \
    kssscale_oracle__stable_dot(frequency_2,outcome_2) \
    kssscale_oracle__stable_dot(frequency_3,outcome_3))
K = J(2,1,0)
K[1] = kssscale_oracle__stable_sum((
    y_mass[1]*compressed_1[2] \
    y_mass[2]*compressed_2[2]))
K[2] = y_mass[3]*compressed_3[2]
prediction = ((.3,-1.2,.7)\(.8,.25,-.4))
for (probe=1; probe<=cols(prediction); probe++) {
    dense_draw = kssscale_oracle__stable_sum((
        y_mass[1]*dense_1[2]*prediction[cell[1],probe]^2 \
        y_mass[2]*dense_2[2]*prediction[cell[2],probe]^2 \
        y_mass[3]*dense_3[2]*prediction[cell[3],probe]^2))
    compressed_draw = kssscale_oracle__stable_sum(
        K:*prediction[.,probe]:^2)
    assert(abs(dense_draw-compressed_draw) <=
        3e-12*max((1,abs(dense_draw),abs(compressed_draw))))
}
assert(rows(cell) == 3 & rows(K) == 2)

// The scalar path and the dense eigenvalue path use the same strict gate.
block_tolerance = 1e-8
frequency = (1\3\2)
residual = (.4\-.2\.7)
dense_near = kssscale_oracle__dense_block(
    residual,frequency,1-2*block_tolerance,
    2e-10,1e-18,block_tolerance)
compressed_near = kssscale_oracle__compressed(
    residual,frequency,1-2*block_tolerance,
    2e-10,1e-18,block_tolerance)
assert(dense_near[1] == 1 & compressed_near[1] == 1)
assert(abs(dense_near[2]-compressed_near[2]) <=
    2e-8*max((1,abs(dense_near[2]),abs(compressed_near[2]))))
for (gate_case=1; gate_case<=2; gate_case++) {
    // Keep the rejected fixtures more than a few ulps below the gate.  The
    // decimal expression 1-tolerance can put 1-p and a computed eigenvalue
    // on opposite sides of the exact floating-point threshold.
    residual_share = gate_case == 1 ? .9*block_tolerance : .5*block_tolerance
    dense_rejected = kssscale_oracle__dense_block(
        residual,frequency,1-residual_share,
        2e-10,1e-18,block_tolerance)
    compressed_rejected = kssscale_oracle__compressed(
        residual,frequency,1-residual_share,
        2e-10,1e-18,block_tolerance)
    assert(dense_rejected[1] == 0)
    assert(compressed_rejected[1] == 0)
}

// Stable accumulation retains the low-order term under severe cancellation
// and is invariant to these alternate group orders.
cancellation = (1e16\1\-1e16)
assert(kssscale_oracle__stable_sum(cancellation) == 1)
assert(kssscale_oracle__stable_sum(cancellation[(3\1\2)]) == 1)

mata drop kssscale_oracle__*()
end

di as result "PASS test_scale_match_formula.do"
exit 0
