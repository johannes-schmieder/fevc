// SPDX-License-Identifier: GPL-3.0-only

//! Pre-RNG planning for the isolated diagonal estimator executor. No public
//! option, solver choice, RNG key, or registered acceptance threshold changes.
use super::*;
use crate::batch_plan::{
    BatchPhaseReceipt, BatchSelectionReason, BATCH_ARITHMETIC_CONTRACT, BATCH_PLAN_SCHEMA_VERSION,
};
use crate::model_solver::diagonal_queue::DiagonalQueuePlan;

pub(super) fn widths(request: BatchRequest, cap: usize, probes: usize) -> Result<Vec<usize>> {
    let mut values = Vec::new();
    reserve_exact(
        &mut values,
        usize::BITS as usize + 1,
        "diagonal batch candidates",
    )?;
    match request {
        BatchRequest::Explicit(0) => return Err(invalid("explicit diagonal batch width is zero")),
        BatchRequest::Explicit(width) => values.push(width.min(probes)),
        BatchRequest::Auto => {
            let mut width = 1;
            while width < cap {
                values.push(width);
                width = width
                    .checked_mul(2)
                    .ok_or_else(|| resource("diagonal batch ladder overflow"))?;
            }
            if values.last().copied() != Some(cap) {
                values.push(cap);
            }
        }
    }
    Ok(values)
}

pub(super) fn add_increment(memory: &mut MemoryForecast, bytes: u64) -> Result<()> {
    // Conservative coexistence bound. Do not subtract an old serial scratch
    // estimate: the generic caller also retains outputs across physical chunks.
    // Every unchanged statistical phase is bounded independently, then charged
    // the one shared queue/runtime increment (not once per model solver).
    for phase in [
        &mut memory.peak,
        &mut memory.canonicalization,
        &mut memory.setup,
        &mut memory.fit,
        &mut memory.geometry,
        &mut memory.leverage,
        &mut memory.target,
        &mut memory.maker,
        &mut memory.result,
        &mut memory.result_transition,
        &mut memory.result_export,
    ] {
        *phase = checked_sum(&[*phase, bytes])?;
    }
    if memory.projection > 0 {
        memory.projection = checked_sum(&[memory.projection, bytes])?;
    }
    Ok(())
}

pub(super) fn plan(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    parameters: usize,
    hybrid: bool,
    component: Option<&ComponentExecution<'_>>,
    component_policy: ComponentBatchPolicy,
    leverage: BatchRequest,
    target: BatchRequest,
    threads: usize,
    facts: MemoryFacts,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(
    GenericJlaBatchExecutionReceipt,
    DiagonalQueuePlan,
    Option<ComponentBatchReceipt>,
)> {
    let probes = options.probes as usize;
    let (leverage_cap, target_cap, _) = crate::full_cmg_batch_policy::caps(
        probes,
        threads,
        crate::full_cmg_batch_policy::SELECTED_K,
    )?;
    let left = widths(leverage, leverage_cap, probes)?;
    let right = widths(target, target_cap, probes)?;
    let inference = component.map_or_else(
        || widths(BatchRequest::Explicit(1), 1, 1),
        |value| value.candidates(component_policy, threads),
    )?;
    let forecast = |l: usize, t: usize, i: usize| -> Result<(MemoryForecast, DiagonalQueuePlan)> {
        let mut memory = memory_forecast(
            problem,
            options,
            parameters,
            hybrid,
            RouteMemory::default(),
            facts,
            l,
            t,
        )?;
        let capacity = component_batches::capacity(
            l.max(
                t.checked_mul(2)
                    .ok_or_else(|| resource("diagonal RHS capacity overflow"))?,
            ),
            component_policy,
            i,
        );
        if let Some(prepared) = component {
            let prepared = prepared.select(component_policy, i);
            memory.component_inference = component_inference_peak_forecast(
                problem,
                &prepared,
                parameters,
                RouteMemory::default(),
                Some(capacity),
                threads.min(capacity),
            )?;
            memory.peak = checked_sum(&[memory.peak, memory.component_inference])?;
            memory.peak_phase = GenericJlaMemoryPeakPhase::ComponentInference;
        }
        let queue = DiagonalQueuePlan::checked(
            problem.workers(),
            problem.firms(),
            problem.controls.len(),
            threads,
            capacity,
            memory.peak,
        )?;
        Ok((memory, queue))
    };
    let minimum_l = left[0];
    let minimum_t = right[0];
    let mut selected = (minimum_l, minimum_t, inference[0]);
    // Maximize the product on fixed ladders; ties prefer leverage, then target.
    // Inference is a third axis only when omission intent was supplied. Price
    // every overlapping lifetime and max(L, 2*Tgt, inference) pool together.
    let budget = options.memory_budget;
    let mut best = None;
    if budget.limit(options.memory_limit_bytes).is_none() {
        selected = (
            *left.last().unwrap(),
            *right.last().unwrap(),
            *inference.last().unwrap(),
        );
    } else {
        for &l in &left {
            for &t in &right {
                for &i in &inference {
                    interrupt.checkpoint("generic_diagonal_queue_plan")?;
                    let (_, queue) = forecast(l, t, i)?;
                    let score = (
                        l.checked_mul(t)
                            .and_then(|value| value.checked_mul(i))
                            .ok_or_else(|| resource("diagonal batch score overflow"))?,
                        l,
                        t,
                    );
                    if budget.fits(queue.command_peak_bytes, options.memory_limit_bytes)
                        && best.is_none_or(|previous| score > previous)
                    {
                        best = Some(score);
                        selected = (l, t, i);
                    }
                }
            }
        }
    }
    let (mut memory, queue) = forecast(selected.0, selected.1, selected.2)?;
    budget.admit(
        queue.command_peak_bytes,
        options.memory_limit_bytes,
        "generic_jla_memory",
    )?;
    add_increment(&mut memory, queue.incremental_peak_bytes)?;
    if memory.peak != queue.command_peak_bytes {
        return Err(BackendError::invariant(
            "generic_diagonal_queue",
            "joint memory forecast does not reconcile",
        ));
    }
    let (_, width_one) = forecast(1, 1, inference[0])?;
    let non_batched = memory
        .canonicalization
        .max(memory.setup)
        .max(memory.fit)
        .max(memory.geometry)
        .max(memory.maker)
        .max(memory.result)
        .max(memory.projection);
    let phase = |request, width, cap, selected_forecast| BatchPhaseReceipt {
        requested: request,
        selected_width: width,
        reason: match request {
            BatchRequest::Explicit(_) => BatchSelectionReason::ExplicitWidth,
            BatchRequest::Auto if budget.limit(options.memory_limit_bytes).is_none() => {
                BatchSelectionReason::NoBudgetPerformanceChoice
            }
            BatchRequest::Auto
                if !budget.fits(queue.command_peak_bytes, options.memory_limit_bytes) =>
            {
                BatchSelectionReason::MinimumMemoryOverBudget
            }
            BatchRequest::Auto => BatchSelectionReason::LargestAdmissibleCandidate,
        },
        probe_width_cap: probes,
        declared_threads: threads,
        thread_width_cap: cap,
        route_width_cap: cap,
        effective_width_cap: cap,
        hard_memory_bytes: budget.receipt_limit(options.memory_limit_bytes),
        width_one_forecast_bytes: width_one.command_peak_bytes,
        selected_forecast_bytes: selected_forecast,
    };
    let phase_cap = |request, automatic| match request {
        BatchRequest::Explicit(width) => width.min(probes),
        BatchRequest::Auto => automatic,
    };
    Ok((
        GenericJlaBatchExecutionReceipt {
            leverage_requested: leverage,
            target_requested: target,
            leverage_active_width: selected.0,
            target_active_width: selected.1,
            automatic_ladder_cap: leverage_cap.max(target_cap),
            plan: BatchPlanReceipt {
                schema_version: BATCH_PLAN_SCHEMA_VERSION,
                deterministic: true,
                bitwise_estimator_width_invariance_required: true,
                arithmetic_contract: BATCH_ARITHMETIC_CONTRACT,
                non_batched_peak_bytes: non_batched,
                selected_command_peak_bytes: memory.peak,
                whole_command_admitted: true,
                leverage: phase(
                    leverage,
                    selected.0,
                    phase_cap(leverage, leverage_cap),
                    memory.peak,
                ),
                target: phase(
                    target,
                    selected.1,
                    phase_cap(target, target_cap),
                    memory.peak,
                ),
            },
        },
        queue,
        component
            .map(|prepared| {
                prepared.receipt(
                    component_policy,
                    selected.2,
                    threads,
                    phase(BatchRequest::Auto, selected.0, leverage_cap, memory.peak).reason,
                )
            })
            .transpose()?,
    ))
}
