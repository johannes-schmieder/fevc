// SPDX-License-Identifier: GPL-3.0-only

//! Direct-solver storage is charged in addition to the generic statistical
//! engine's observation/match-specific live arrays. No memory policy changes.
use super::*;

#[path = "generic_cmg_attachments.rs"]
pub(super) mod attachments;

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_controlled<'a>(
    problem: &'a CompressedProblem,
    data: CanonicalModelData<'a>,
    routing: ModelRoutingOptions,
    plan: FullCmgPlanOptions,
    options: GenericJlaOptions,
    parameters: usize,
    hybrid: bool,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(
    crate::model_solver::PreparedGenericJlaSolvers<'a>,
    GenericJlaBatchExecutionReceipt,
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
    let route = route_memory_forecast(problem, &fe, data.controls.len())?;
    let batch = plan_generic_jla_batches(
        problem,
        options,
        parameters,
        hybrid,
        BatchRequest::Auto,
        BatchRequest::Auto,
        route,
        facts,
        interrupt,
    )?;
    let maximum = batch
        .target_active_width
        .checked_mul(2)
        .ok_or_else(|| resource("controlled target capacity overflow"))?
        .max(batch.leverage_active_width);
    fe.configure_full_cmg_capacity(maximum)?;
    let route = route_memory_forecast(problem, &fe, data.controls.len())?;
    let before = memory_forecast(
        problem,
        options,
        parameters,
        hybrid,
        route,
        facts,
        batch.leverage_active_width,
        batch.target_active_width,
    )?;
    if options
        .memory_budget
        .rejects(before.peak, options.memory_limit_bytes)
    {
        admit_memory(before.peak, options.memory_limit_bytes)?;
    }
    interrupt.checkpoint("generic_full_cmg_controls_admitted")?;
    fe.allocate_full_cmg_pools()?;
    // Rank-specific solves now run only after the pools and all coexisting
    // projection, control, queue and result storage have been admitted.
    let full =
        PreparedModelSolver::prepare_generic_jla_direct_controlled(data, routing, &fe, interrupt)?;
    if full.direct_control_retained_bytes()? > controlled_storage(problem)? {
        return Err(BackendError::invariant(
            "generic_full_cmg",
            "actual control capacities exceed the pre-allocation bound",
        ));
    }
    let after = route_memory_forecast(problem, &full, data.controls.len())?;
    let realized = memory_forecast(
        problem,
        options,
        parameters,
        hybrid,
        after,
        facts,
        batch.leverage_active_width,
        batch.target_active_width,
    )?;
    if realized.peak > before.peak {
        return Err(BackendError::invariant(
            "generic_full_cmg",
            "controlled retained storage exceeds admission",
        ));
    }
    Ok((
        crate::model_solver::PreparedGenericJlaSolvers {
            full,
            fe,
            fe_hierarchy_reused: true,
        },
        batch,
    ))
}

fn controlled_storage(problem: &CompressedProblem) -> Result<u64> {
    let q = problem.controls.len() as u64;
    checked_sum(&[
        checked_product(
            &[
                checked_sum(&[problem.workers() as u64, problem.firms() as u64])?,
                q,
                8,
            ],
            "strict FE control coefficients",
        )?,
        checked_product(&[q, q, 8], "direct control inverse")?,
        checked_product(
            &[
                q,
                std::mem::size_of::<crate::model_solver::ModelCoefficients>() as u64,
            ],
            "direct control projection headers",
        )?,
        checked_product(&[q, 8], "full control diagonal")?,
    ])
}

pub(super) fn route_memory(
    problem: &CompressedProblem,
    setup: FullCmgSetupReceipt,
) -> Result<RouteMemory> {
    // Retained hierarchy bytes are not a construction-peak bound: the terminal
    // dense factorization can hold temporary square matrices that disappear
    // before its retained receipt is sampled. Price that lifetime separately,
    // using the already-realized hybrid dimensions and the pinned solver's
    // checked estimate. Pools are deferred here; one RHS bounds setup scratch.
    let construction = cmg::CmgMemoryEstimate::conservative(
        cmg::CmgProblemSize {
            vertices: setup.vertices,
            input_edges: setup.edges,
            canonical_edges: setup.edges,
            right_hand_sides: 1,
        },
        cmg::CmgOptions::default(),
        cmg::ParallelOptions {
            threads: setup.threads,
            ..Default::default()
        },
    )
    .map_err(|error| resource(format!("direct construction forecast: {error}")))?;
    let construction = checked_sum(&[
        construction.build_peak_bytes() as u64,
        setup.graph_copy_bytes,
        checked_product(&[setup.vertices as u64, 48], "hybrid vertex setup")?,
        checked_product(&[setup.edges as u64, 40], "hybrid edge setup")?,
        checked_product(&[problem.workers() as u64, 4], "hybrid worker setup")?,
    ])?;
    let construction = checked_sum(&[construction, construction / 5])?;
    let operators = checked_product(
        &[
            checked_sum(&[
                problem.workers() as u64,
                checked_product(&[problem.firms() as u64, 4], "direct diagonals")?,
            ])?,
            8,
        ],
        "direct operator and generic diagonal storage",
    )?;
    let persistent = checked_sum(&[
        setup.actual_retained_bytes,
        setup.allocator_allowance_bytes,
        operators,
    ])?;
    let controls = problem.controls.len() as u64;
    let control_storage = controlled_storage(problem)?;
    let control_preparation = if controls == 0 {
        0
    } else {
        checked_sum(&[
            persistent,
            control_storage,
            checked_product(
                &[problem.outcome.len() as u64, controls, 3, 8],
                "strict rank residualized controls",
            )?,
            checked_product(
                &[
                    checked_sum(&[problem.workers() as u64, problem.firms() as u64])?,
                    controls,
                    6,
                    8,
                ],
                "strict projection RHS corrections and solutions",
            )?,
            checked_product(&[controls, controls, 20, 8], "strict rank dense work")?,
            checked_product(&[controls, 512], "strict rank receipt headers")?,
            solve_bytes(
                problem.workers() as u64,
                problem.firms() as u64,
                controls,
                controls,
                setup,
            )?,
        ])?
    };
    Ok(RouteMemory {
        full_cmg: Some(setup),
        shared_cmg_persistent: persistent,
        full_control_block_persistent: control_storage,
        setup_transient: construction
            .max(setup.pre_rng_forecast_bytes)
            .max(control_preparation),
        ..RouteMemory::default()
    })
}

pub(super) fn solve_bytes(
    workers: u64,
    firms: u64,
    controls: u64,
    columns: u64,
    setup: FullCmgSetupReceipt,
) -> Result<u64> {
    checked_sum(&[
        // The independent statistical queue is sequential with the solve
        // queue, but both envelopes are retained conservatively in admission.
        crate::ordered_work::metadata_bytes(
            usize::try_from(columns).map_err(|_| resource("statistical queue size overflow"))?,
        )?,
        crate::full_cmg::queue_metadata_bytes(
            usize::try_from(columns).map_err(|_| resource("queue size overflow"))?,
        )?,
        checked_product(
            &[setup.vertices as u64, columns, 4, 8],
            "direct refinement blocks",
        )?,
        checked_product(
            &[
                checked_sum(&[
                    checked_product(&[workers, 3], "returned worker vectors")?,
                    checked_product(&[firms, 4], "returned firm vectors")?,
                ])?,
                columns,
                8,
            ],
            "direct and generic returned solutions",
        )?,
        checked_product(
            &[
                checked_sum(&[workers, checked_product(&[firms, 4], "extraction firms")?])?,
                columns.min(setup.threads as u64),
                8,
            ],
            "direct extraction scratch",
        )?,
        checked_product(
            &[columns, 1024],
            "adapter receipts, certification slots and vector headers",
        )?,
        if controls == 0 {
            0
        } else {
            checked_sum(&[
                checked_product(
                    &[controls, columns, 8, 8],
                    "controlled results and Schur contractions",
                )?,
                checked_product(
                    &[checked_sum(&[workers, firms, controls])?, 8, 8],
                    "full-system scalar correction and certification",
                )?,
                checked_product(
                    &[setup.vertices as u64, 4, 8],
                    "controlled FE correction refinement blocks",
                )?,
                crate::full_cmg::queue_metadata_bytes(1)?,
            ])?
        },
    ])
}

#[allow(clippy::too_many_arguments)]
pub(super) fn plan_batches(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    parameters: usize,
    hybrid: bool,
    route: RouteMemory,
    facts: MemoryFacts,
    setup: FullCmgSetupReceipt,
    leverage_request: BatchRequest,
    target_request: BatchRequest,
) -> Result<GenericJlaBatchExecutionReceipt> {
    if leverage_request != BatchRequest::Auto || target_request != BatchRequest::Auto {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "generic_full_cmg",
            "explicit direct batches are unsupported",
        ));
    }
    let probes = options.probes as usize;
    let (l, t, _) = crate::full_cmg_batch_policy::caps(
        probes,
        setup.threads,
        crate::full_cmg_batch_policy::SELECTED_K,
    )?;
    let one = memory_forecast(problem, options, parameters, hybrid, route, facts, 1, 1)?;
    let non_batched = one
        .canonicalization
        .max(one.setup)
        .max(one.fit)
        .max(one.geometry)
        .max(one.maker)
        .max(one.result);
    let caps = |width| BatchPlannerCaps {
        probes,
        declared_threads: 1,
        columns_per_thread: width,
        route_width_cap: width,
        non_batched_peak_bytes: non_batched,
        hard_memory_bytes: options.memory_limit_bytes,
        memory_budget: options.memory_budget,
    };
    let left = crate::batch_plan::plan_full_cmg_batches_with_forecasts(
        BatchRequest::Auto,
        BatchRequest::Explicit(1),
        caps(l),
        |width| {
            Ok(
                memory_forecast(problem, options, parameters, hybrid, route, facts, width, 1)?
                    .leverage,
            )
        },
        |_| Ok(non_batched),
    )?;
    let right = crate::batch_plan::plan_full_cmg_batches_with_forecasts(
        BatchRequest::Explicit(1),
        BatchRequest::Auto,
        caps(t),
        |_| Ok(non_batched),
        |width| {
            Ok(
                memory_forecast(problem, options, parameters, hybrid, route, facts, 1, width)?
                    .target,
            )
        },
    )?;
    let mut plan = left;
    plan.target = right.target;
    plan.selected_command_peak_bytes = non_batched
        .max(plan.leverage.selected_forecast_bytes)
        .max(plan.target.selected_forecast_bytes);
    Ok(GenericJlaBatchExecutionReceipt {
        leverage_requested: leverage_request,
        target_requested: target_request,
        leverage_active_width: plan.leverage.selected_width,
        target_active_width: plan.target.selected_width,
        automatic_ladder_cap: l.max(t),
        plan,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn refresh_batch(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    parameters: usize,
    hybrid: bool,
    route: RouteMemory,
    facts: MemoryFacts,
    batch: &mut GenericJlaBatchExecutionReceipt,
) -> Result<()> {
    let one = memory_forecast(problem, options, parameters, hybrid, route, facts, 1, 1)?;
    let final_memory = memory_forecast(
        problem,
        options,
        parameters,
        hybrid,
        route,
        facts,
        batch.leverage_active_width,
        batch.target_active_width,
    )?;
    batch.plan.non_batched_peak_bytes = one
        .canonicalization
        .max(one.setup)
        .max(one.fit)
        .max(one.geometry)
        .max(one.maker)
        .max(one.result);
    let non_batched = batch.plan.non_batched_peak_bytes;
    batch.plan.leverage.width_one_forecast_bytes = one.leverage.max(non_batched);
    batch.plan.target.width_one_forecast_bytes = one.target.max(non_batched);
    batch.plan.leverage.selected_forecast_bytes = final_memory.leverage.max(non_batched);
    batch.plan.target.selected_forecast_bytes = final_memory.target.max(non_batched);
    batch.plan.selected_command_peak_bytes = final_memory.peak;
    Ok(())
}
