// SPDX-License-Identifier: GPL-3.0-only

//! Internal attachment admission. The public point-only planner is unchanged.
use super::*;
use crate::batch_plan::{
    BatchPhaseReceipt, BatchSelectionReason, BATCH_ARITHMETIC_CONTRACT, BATCH_PLAN_SCHEMA_VERSION,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn forecast(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    parameters: usize,
    hybrid: bool,
    component: Option<&ComponentExecution<'_>>,
    route: RouteMemory,
    facts: MemoryFacts,
    leverage: usize,
    target: usize,
) -> Result<MemoryForecast> {
    let mut memory = memory_forecast(
        problem, options, parameters, hybrid, route, facts, leverage, target,
    )?;
    if let Some(prepared) = component {
        memory.component_inference = component_inference_peak_forecast(
            problem,
            prepared,
            parameters,
            route,
            Some(
                route
                    .full_cmg
                    .map_or(capacity(leverage, target)?, |setup| setup.maximum_batch_rhs),
            ),
            route
                .full_cmg
                .map_or(usize::MAX, |setup| setup.workspace_count),
        )?;
        memory.peak = checked_sum(&[memory.peak, memory.component_inference])?;
        memory.peak_phase = GenericJlaMemoryPeakPhase::ComponentInference;
    }
    Ok(memory)
}

fn capacity(leverage: usize, target: usize) -> Result<usize> {
    Ok(leverage.max(
        target
            .checked_mul(2)
            .ok_or_else(|| resource("CMG RHS capacity overflow"))?,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare<'a>(
    problem: &'a CompressedProblem,
    data: CanonicalModelData<'a>,
    routing: ModelRoutingOptions,
    plan: FullCmgPlanOptions,
    options: GenericJlaOptions,
    parameters: usize,
    hybrid: bool,
    component: Option<&ComponentExecution<'_>>,
    component_policy: ComponentBatchPolicy,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(
    crate::model_solver::PreparedGenericJlaSolvers<'a>,
    GenericJlaBatchExecutionReceipt,
    Option<ComponentBatchReceipt>,
)> {
    let mut fe = PreparedModelSolver::prepare_generic_jla_direct_fe(
        problem,
        data,
        routing,
        plan,
        options.memory_limit_bytes,
        interrupt,
    )?;
    let facts = memory_facts(problem, interrupt)?;
    let probes = options.probes as usize;
    let (left_cap, right_cap, _) = crate::full_cmg_batch_policy::caps(
        probes,
        plan.threads,
        crate::full_cmg_batch_policy::SELECTED_K,
    )?;
    let left = diagonal::widths(BatchRequest::Auto, left_cap, probes)?;
    let right = diagonal::widths(BatchRequest::Auto, right_cap, probes)?;
    let inference = component.map_or_else(
        || diagonal::widths(BatchRequest::Explicit(1), 1, 1),
        |value| value.candidates(component_policy, plan.threads),
    )?;
    let price = |l, t, i| {
        let setup = fe.forecast_full_cmg_capacity(component_batches::capacity(
            capacity(l, t)?,
            component_policy,
            i,
        ))?;
        let selected_component = component.map(|value| value.select(component_policy, i));
        forecast(
            problem,
            options,
            parameters,
            hybrid,
            selected_component.as_ref(),
            route_memory(problem, setup)?,
            facts,
            l,
            t,
        )
    };
    let budget = options.memory_budget;
    let mut selected = (1, 1, inference[0]);
    let mut best = None;
    if budget.limit(options.memory_limit_bytes).is_none() {
        selected = (left_cap, right_cap, *inference.last().unwrap());
    } else {
        // Point and automatic inference axes alter the same owned pool. Price
        // the complete attachment, not independently affordable phases.
        for &l in &left {
            for &t in &right {
                for &i in &inference {
                    interrupt.checkpoint("generic_full_cmg_attachment_plan")?;
                    let memory = price(l, t, i)?;
                    let score = (
                        l.checked_mul(t)
                            .and_then(|value| value.checked_mul(i))
                            .ok_or_else(|| resource("CMG batch score overflow"))?,
                        l,
                        t,
                    );
                    if budget.fits(memory.peak, options.memory_limit_bytes)
                        && best.is_none_or(|previous| score > previous)
                    {
                        selected = (l, t, i);
                        best = Some(score);
                    }
                }
            }
        }
    }
    let one = price(1, 1, inference[0])?;
    let before = price(selected.0, selected.1, selected.2)?;
    budget.admit(
        before.peak,
        options.memory_limit_bytes,
        "generic_jla_memory",
    )?;
    fe.configure_full_cmg_capacity(component_batches::capacity(
        capacity(selected.0, selected.1)?,
        component_policy,
        selected.2,
    ))?;
    // Attachment, callback, strict rank, pool, queue and output lifetimes have
    // all been admitted before the first solve worker allocation or rank RHS.
    interrupt.checkpoint("generic_full_cmg_attachments_admitted")?;
    fe.allocate_full_cmg_pools()?;
    let full =
        PreparedModelSolver::prepare_generic_jla_direct_controlled(data, routing, &fe, interrupt)?;
    if full.direct_control_retained_bytes()? > controlled_storage(problem)? {
        return Err(BackendError::invariant(
            "generic_full_cmg",
            "control capacities exceed admission",
        ));
    }
    fe.refresh_full_cmg_receipt()?;
    let selected_component = component.map(|value| value.select(component_policy, selected.2));
    let after = forecast(
        problem,
        options,
        parameters,
        hybrid,
        selected_component.as_ref(),
        route_memory_forecast(problem, &full, data.controls.len())?,
        facts,
        selected.0,
        selected.1,
    )?;
    if after.peak > before.peak {
        return Err(BackendError::invariant(
            "generic_full_cmg",
            "retained attachment storage exceeds admission",
        ));
    }
    let phase = |width, cap| BatchPhaseReceipt {
        requested: BatchRequest::Auto,
        selected_width: width,
        reason: if budget.limit(options.memory_limit_bytes).is_none() {
            BatchSelectionReason::NoBudgetPerformanceChoice
        } else if !budget.fits(before.peak, options.memory_limit_bytes) {
            BatchSelectionReason::MinimumMemoryOverBudget
        } else {
            BatchSelectionReason::LargestAdmissibleCandidate
        },
        probe_width_cap: probes,
        declared_threads: plan.threads,
        thread_width_cap: cap,
        route_width_cap: cap,
        effective_width_cap: cap,
        hard_memory_bytes: budget.receipt_limit(options.memory_limit_bytes),
        width_one_forecast_bytes: one.peak,
        selected_forecast_bytes: after.peak,
    };
    Ok((
        crate::model_solver::PreparedGenericJlaSolvers {
            full,
            fe,
            fe_hierarchy_reused: true,
        },
        GenericJlaBatchExecutionReceipt {
            leverage_requested: BatchRequest::Auto,
            target_requested: BatchRequest::Auto,
            leverage_active_width: selected.0,
            target_active_width: selected.1,
            automatic_ladder_cap: left_cap.max(right_cap),
            plan: BatchPlanReceipt {
                schema_version: BATCH_PLAN_SCHEMA_VERSION,
                deterministic: true,
                bitwise_estimator_width_invariance_required: true,
                arithmetic_contract: BATCH_ARITHMETIC_CONTRACT,
                non_batched_peak_bytes: one.peak,
                selected_command_peak_bytes: after.peak,
                whole_command_admitted: true,
                leverage: phase(selected.0, left_cap),
                target: phase(selected.1, right_cap),
            },
        },
        component
            .map(|prepared| {
                prepared.receipt(
                    component_policy,
                    selected.2,
                    plan.threads,
                    phase(selected.0, left_cap).reason,
                )
            })
            .transpose()?,
    ))
}
