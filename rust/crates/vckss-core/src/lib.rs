// SPDX-License-Identifier: GPL-3.0-only

//! Rust numerical backend for `vckss`.
//!
//! The crate is deliberately independent of Stata's ABI. The plugin crate owns
//! the caller-thread-only Stata bridge; this crate owns validated native data,
//! deterministic parallelism, graph and numerical algorithms, and receipts.

pub mod batch;
pub mod batch_plan;
pub mod cmg;
pub mod component_inference;
pub mod control_basis;
pub mod counter_accounting;
mod dense;
pub mod engine;
pub mod engine_plan;
pub mod error;
pub mod exact;
pub mod exact_estimator;
pub mod full_cmg;
mod full_cmg_batch_policy;
pub mod generic_batch;
pub mod generic_jla;
pub mod graph;
pub mod interrupt;
pub mod jla;
pub mod krylov;
pub mod memory;
pub mod model_operator;
pub mod model_solver;
pub mod operator;
pub mod parallel;
pub mod problem;
pub mod projection;
pub mod receipt;
#[doc(hidden)]
pub mod residual_moment_inference;
/// Internal observation variance candidate; no plugin or Stata capability.
#[doc(hidden)]
pub mod residual_moments;
pub mod rng;
pub mod solver;
pub mod stayer_hybrid;
pub mod structured_variance;
pub mod types;
pub mod wall_plan;

use cmg::HybridGraph;
use error::Result;
use exact::solve_two_way_exact;
use graph::select_match_deletion_graph;
use jla::JlaPlan;
use krylov::{solve_two_way_pcg, PcgOptions};
use operator::TwoWayOperator;
use parallel::{compensated_sum, DeterministicExecutor};
use problem::CanonicalInput;
use types::{BackendOptions, InputColumns};

pub const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const ABI_VERSION: u32 = 1;
pub const NUMERICAL_CONTRACT: &str = "VCKSS-RUST-NUMERICAL-V1";
pub const RECEIPT_SCHEMA: &str = "VCKSS-RUST-RECEIPT-V1";
pub const COUNTER_RNG_CONTRACT: &str = "VCKSS-COUNTER-V1";
pub const CMG_BASELINE: &str = "VCKSS-CMG-API7-IMPROVED-BASELINE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Capabilities {
    pub abi_version: u32,
    pub core_match_graph_ready: bool,
    pub core_exact_ready: bool,
    pub core_diagonal_pcg_ready: bool,
    pub core_batched_pcg_ready: bool,
    pub core_cmg_graph_ready: bool,
    pub core_solver_router_ready: bool,
    pub core_counter_rng_ready: bool,
    pub core_jla_plan_ready: bool,
    pub supports_exact: bool,
    pub supports_jla: bool,
    pub supports_match_deletion: bool,
    pub supports_observation_deletion: bool,
    pub supports_controls: bool,
    pub supports_diagonal: bool,
    pub supports_cmg: bool,
    pub deterministic_parallelism: bool,
}

impl Capabilities {
    #[must_use]
    pub const fn current() -> Self {
        Self {
            abi_version: ABI_VERSION,
            core_match_graph_ready: true,
            core_exact_ready: true,
            core_diagonal_pcg_ready: true,
            core_batched_pcg_ready: true,
            core_cmg_graph_ready: true,
            core_solver_router_ready: true,
            core_counter_rng_ready: true,
            core_jla_plan_ready: true,
            supports_exact: true,
            supports_jla: true,
            supports_match_deletion: true,
            supports_observation_deletion: true,
            supports_controls: true,
            supports_diagonal: true,
            supports_cmg: false,
            deterministic_parallelism: true,
        }
    }

    #[must_use]
    pub fn to_json(self) -> String {
        format!(
            concat!(
                "{{\"abi_version\":{},",
                "\"backend_version\":\"{}\",",
                "\"numerical_contract\":\"{}\",",
                "\"receipt_schema\":\"{}\",",
                "\"counter_rng_contract\":\"{}\",",
                "\"cmg_baseline\":\"{}\",",
                "\"core_match_graph_ready\":{},",
                "\"core_exact_ready\":{},",
                "\"core_diagonal_pcg_ready\":{},",
                "\"core_batched_pcg_ready\":{},",
                "\"core_cmg_graph_ready\":{},",
                "\"core_solver_router_ready\":{},",
                "\"core_counter_rng_ready\":{},",
                "\"core_jla_plan_ready\":{},",
                "\"supports_exact\":{},",
                "\"supports_jla\":{},",
                "\"supports_match_deletion\":{},",
                "\"supports_observation_deletion\":{},",
                "\"supports_controls\":{},",
                "\"supports_diagonal\":{},",
                "\"supports_cmg\":{},",
                "\"deterministic_parallelism\":{}}}"
            ),
            self.abi_version,
            BACKEND_VERSION,
            NUMERICAL_CONTRACT,
            RECEIPT_SCHEMA,
            COUNTER_RNG_CONTRACT,
            CMG_BASELINE,
            self.core_match_graph_ready,
            self.core_exact_ready,
            self.core_diagonal_pcg_ready,
            self.core_batched_pcg_ready,
            self.core_cmg_graph_ready,
            self.core_solver_router_ready,
            self.core_counter_rng_ready,
            self.core_jla_plan_ready,
            self.supports_exact,
            self.supports_jla,
            self.supports_match_deletion,
            self.supports_observation_deletion,
            self.supports_controls,
            self.supports_diagonal,
            self.supports_cmg,
            self.deterministic_parallelism,
        )
    }
}

pub fn selftest() -> Result<()> {
    BackendOptions::default().validate()?;
    let input = InputColumns {
        worker: vec![1, 1, 2, 2],
        firm: vec![1, 2, 1, 2],
        deletion: vec![1, 2, 3, 4],
        outcome: vec![1.5, 0.5, -0.5, -1.5],
        frequency: vec![1, 1, 1, 1],
        target_weight: vec![1.0, 1.0, 1.0, 1.0],
        controls: Vec::new(),
    }
    .validate()?;

    let executor = DeterministicExecutor::new(2)?;
    let partial = executor.map_partitions(input.rows(), |range| {
        Ok(compensated_sum(&input.columns.outcome[range]))
    })?;
    if compensated_sum(&partial) != 0.0 {
        return Err(error::BackendError::invariant(
            "selftest",
            "deterministic parallel sum failed",
        ));
    }

    let canonical = CanonicalInput::from_validated(input)?;
    let selection = select_match_deletion_graph(&canonical)?;
    let problem = canonical.compress(&selection.active)?;
    let plan = JlaPlan::build_no_controls(&problem)?;
    if plan.deletion_units() != problem.deletion_units() || plan.target_strata() == 0 {
        return Err(error::BackendError::invariant(
            "selftest",
            "JLA semantic plan has inconsistent dimensions",
        ));
    }
    let operator = TwoWayOperator::new(&problem)?;
    let hybrid = HybridGraph::from_problem(&problem)?;
    let firm_test = vec![0.5, -0.5];
    let mut schur_test = vec![0.0; problem.firms()];
    operator.apply_full_schur(&firm_test, &mut schur_test)?;
    let hybrid_test = hybrid.firm_schur_action(&firm_test)?;
    if hybrid_test
        .iter()
        .zip(&schur_test)
        .any(|(&left, &right)| (left - right).abs() > 1.0e-12)
    {
        return Err(error::BackendError::invariant(
            "selftest",
            "hybrid graph does not reproduce the firm Schur action",
        ));
    }

    let (worker_rhs, firm_rhs) = operator.outcome_rhs()?;
    let exact = solve_two_way_exact(&operator, &worker_rhs, &firm_rhs, 1.0e-12)?;
    let iterative = solve_two_way_pcg(
        &operator,
        &worker_rhs,
        &firm_rhs,
        PcgOptions {
            tolerance: 1.0e-12,
            maximum_iterations: 100,
            residual_replacement_interval: 10,
        },
        1.0e-11,
    )?;
    if exact.solution.residual.relative_norm > 1.0e-12
        || iterative.solution.residual.relative_norm > 1.0e-11
    {
        return Err(error::BackendError::invariant(
            "selftest",
            "exact or iterative full residual failed",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_json_is_stable_and_valid_shape() {
        let json = Capabilities::current().to_json();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("\"abi_version\":1"));
        assert!(json.contains(CMG_BASELINE));
        assert!(json.contains(COUNTER_RNG_CONTRACT));
        assert!(json.contains("\"core_exact_ready\":true"));
        assert!(json.contains("\"core_batched_pcg_ready\":true"));
        assert!(json.contains("\"core_cmg_graph_ready\":true"));
        assert!(json.contains("\"core_solver_router_ready\":true"));
        assert!(json.contains("\"core_counter_rng_ready\":true"));
        assert!(json.contains("\"core_jla_plan_ready\":true"));
        assert!(json.contains("\"supports_exact\":true"));
        assert!(json.contains("\"supports_observation_deletion\":true"));
        assert!(json.contains("\"supports_controls\":true"));
    }

    #[test]
    fn backend_selftest_passes() {
        selftest().expect("selftest");
    }
}
