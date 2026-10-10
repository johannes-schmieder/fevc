// SPDX-License-Identifier: GPL-3.0-only
//! Inference units preserve the point executor's certified mixed partition.
use super::*;
use crate::structured_variance::{basis_row, normalized_midranks, StructuredVarianceModel};

// Preserve design ties across equivalent solver reductions. The explicitly
// registered mixed-model features use a 40-bit relative binary grid before
// midranking; this does not change leverages, makers, targets or rank gates.
fn midranks(values: &[f64], interrupt: &mut dyn InterruptCheck) -> Result<Vec<f64>> {
    let maximum = values.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
    if maximum == 0.0 {
        return normalized_midranks(values, interrupt);
    }
    let grid = 2.0_f64.powf(maximum.log2().floor() - 40.0);
    if !grid.is_finite() || grid <= 0.0 {
        return Err(nonfinite("mixed feature tie grid is not representable"));
    }
    let mut rounded = Vec::new();
    reserve_exact(&mut rounded, values.len(), "mixed feature ranks")?;
    for (index, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "mixed_feature_ties")?;
        rounded.push((value / grid).round());
    }
    normalized_midranks(&rounded, interrupt)
}

pub(super) fn basis(
    model: StructuredVarianceModel,
    leverage: &[f64],
    diagonals: &[Vec<f64>; PRIMITIVE_TARGETS],
    mass: &[f64],
    movers: usize,
    observations_per_term: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let n = leverage.len();
    if movers == 0 || movers >= n || mass.len() != n || diagonals.iter().any(|x| x.len() != n) {
        return Err(invalid("mixed variance basis dimensions disagree"));
    }
    let (mover_terms, stayer_terms) = match model {
        StructuredVarianceModel::Common => (21, 15),
        StructuredVarianceModel::LeverageOnly => (3, 3),
    };
    let terms = mover_terms + stayer_terms;
    let length = checked_matrix_length(n, terms, "mixed variance basis")?;
    let mut output = zeroed_f64_with_interrupt(
        length,
        "mixed variance basis",
        interrupt,
        "mixed_variance_basis",
    )?;
    for (start, end, count, offset) in [
        (0, movers, mover_terms, 0),
        (movers, n, stayer_terms, mover_terms),
    ] {
        let mut ranks = vec![midranks(&leverage[start..end], interrupt)?];
        for (target, diagonal) in diagonals.iter().enumerate() {
            // A fixed-offset stayer shock is absorbed entirely by its worker
            // effect. Its firm and worker-firm target diagonals are exactly
            // zero. Ranking their solver roundoff creates spurious regressors.
            if start != 0 && target > 0 {
                ranks.push(zeroed_f64_with_interrupt(
                    end - start,
                    "structural zero ranks",
                    interrupt,
                    "mixed_variance_basis",
                )?);
            } else {
                ranks.push(midranks(&diagonal[start..end], interrupt)?);
            }
        }
        if start == 0 {
            ranks.push(normalized_midranks(&mass[start..end], interrupt)?);
        }
        let mut local = Vec::new();
        reserve_exact(
            &mut local,
            checked_matrix_length(end - start, count, "type variance basis")?,
            "type variance basis",
        )?;
        for row in 0..end - start {
            checkpoint_chunk(interrupt, row, "mixed_variance_basis")?;
            let values = basis_row(model, &ranks, row);
            local.extend_from_slice(&values[..count]);
            let base = (start + row) * terms;
            output[base] = 1.0;
            output[base + offset..base + offset + count].copy_from_slice(&values[..count]);
        }
        let (active, _) = crate::residual_moments::basis::reduce(&mut local, count, interrupt)?;
        let required = active
            .len()
            .checked_mul(observations_per_term)
            .ok_or_else(|| resource("mixed variance support count overflow"))?;
        if end - start < required {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "mixed_variance_support",
                "each unit type must support its active variance terms",
            ));
        }
    }
    // Global intercept plus a stayer intercept contrast spans the two separate
    // intercepts and satisfies the existing fitter's constant-first-column API.
    Ok(output)
}

pub(super) struct Plan {
    pub plan: MatchPlan,
    pub mover_units: usize,
    // Stored row and literal-copy index in the existing observation maker.
    pub observations: Vec<(usize, usize)>,
}

pub(super) fn plan(
    problem: &CompressedProblem,
    movers: &MatchPlan,
    hybrid: &ExactStayerHybridPlan,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Plan> {
    let mut plan = movers.clone();
    let mut observations = Vec::new();
    let count = problem
        .frequency
        .iter()
        .zip(&hybrid.stayer_rows)
        .filter(|(_, stayer)| **stayer)
        .try_fold(0_usize, |count, (&frequency, _)| {
            count
                .checked_add(
                    usize::try_from(frequency).map_err(|_| resource("mixed physical count"))?,
                )
                .ok_or_else(|| resource("mixed physical count overflow"))
        })?;
    reserve_exact(&mut observations, count, "mixed observation addresses")?;
    reserve_exact(&mut plan.rows, count, "mixed singleton units")?;
    reserve_exact(&mut plan.cell, count, "mixed unit cells")?;
    reserve_exact(&mut plan.physical_count, count, "mixed unit masses")?;
    reserve_exact(&mut plan.entity, count, "mixed unit entities")?;
    let mut physical = 0_usize;
    for (row, &stayer) in hybrid.stayer_rows.iter().enumerate() {
        let frequency = usize::try_from(problem.frequency[row])
            .map_err(|_| resource("mixed component physical count"))?;
        if stayer {
            for copy in 0..frequency {
                checkpoint_chunk(interrupt, copy, "mixed_component_units")?;
                observations.push((row, physical + copy));
                let mut singleton = Vec::new();
                reserve_exact(&mut singleton, 1, "mixed singleton row")?;
                singleton.push(row);
                plan.rows.push(singleton);
                plan.cell.push(problem.row_cell[row]);
                plan.physical_count.push(1);
                plan.entity.push(0); // Inference addresses are assigned from design classes.
            }
        }
        physical = physical
            .checked_add(frequency)
            .ok_or_else(|| resource("mixed component physical count overflow"))?;
    }
    Ok(Plan {
        plan,
        mover_units: movers.rows.len(),
        observations,
    })
}

pub(super) fn geometry(
    plan: &Plan,
    movers: &mut DeletedAdjustment,
    stayers: &mut DeletedAdjustment,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut leverage = movers
        .inference_leverage
        .take()
        .ok_or_else(|| invalid("missing mover inference leverage"))?;
    let mut maker = movers
        .inference_maker_inverse
        .take()
        .ok_or_else(|| invalid("missing mover inference maker"))?;
    let (physical_leverage, physical_maker) = stayers
        .inference_physical
        .take()
        .ok_or_else(|| invalid("missing physical stayer inference geometry"))?;
    reserve_exact(&mut leverage, plan.observations.len(), "mixed leverage")?;
    reserve_exact(&mut maker, plan.observations.len(), "mixed makers")?;
    for (index, &(_, physical)) in plan.observations.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "mixed_geometry")?;
        leverage.push(physical_leverage[physical]);
        maker.push(physical_maker[physical]);
    }
    if leverage.iter().chain(&maker).any(|x| !x.is_finite()) {
        return Err(nonfinite("mixed component geometry is nonfinite"));
    }
    Ok((leverage, maker))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_data(
    plan: &Plan,
    data: &mut CollapsedMatchComponentData,
    outcome: &[f64],
    residual: &[f64],
    maker: &[f64],
    diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if data.outcome.len() != plan.mover_units || maker.len() != plan.plan.rows.len() {
        return Err(invalid("mixed component collapse counts disagree"));
    }
    for values in [
        &mut data.outcome,
        &mut data.residual,
        &mut data.deleted_adjusted,
        &mut data.mass,
    ] {
        reserve_exact(values, plan.observations.len(), "mixed component values")?;
    }
    for values in &mut data.target_diagonal {
        reserve_exact(values, plan.observations.len(), "mixed target diagonals")?;
    }
    for (index, &(row, _)) in plan.observations.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "mixed_component_values")?;
        data.outcome.push(outcome[row]);
        data.residual.push(residual[row]);
        data.deleted_adjusted
            .push(residual[row] * maker[plan.mover_units + index]);
        data.mass.push(1.0);
        for (target, values) in diagonal.iter().enumerate() {
            data.target_diagonal[target].push(values[row]);
        }
    }
    Ok(())
}
