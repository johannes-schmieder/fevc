// SPDX-License-Identifier: GPL-3.0-only

//! Graph-certified preparation with an original-marked-row retention mask.
//!
//! `retained[row]` is aligned to the caller's compact marked-row sequence. The
//! C shim can therefore scatter it back into a preallocated Stata keep variable
//! without retaining pointers to Stata-managed data inside Rust.

use std::sync::Arc;
use std::time::{Duration, Instant};

use vckss_core::component_inference::{
    prepare_grouped_structured_component_inference,
    prepare_structured_component_inference_with_interrupt, ComponentInferenceOptions,
    ComponentInferenceUnit, ComponentVarianceSource, PreparedComponentInference,
};
use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::graph::{
    select_match_deletion_graph_with_implicit_match_and_interrupt,
    select_observation_deletion_graph_with_interrupt,
};
use vckss_core::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use vckss_core::jla::JlaPlan;
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::projection::{
    prepare_projection_with_interrupt, PreparedProjection, ProjectionEffect, ProjectionWeight,
};
use vckss_core::stayer_hybrid::{
    prepare_exact_stayer_hybrid_with_interrupt, PreparedExactStayerHybrid, StayerAugmentationInput,
};
use vckss_core::structured_variance::StructuredVarianceOptions;
use vckss_core::types::{DeletionMode, InputColumns};

use crate::context::{ContextHandle, ContextRegistry, ContextSnapshot};
use crate::session::{
    PreparationMemoryReceipt, PreparationReceipt, StayerAugmentationMemoryReceipt,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComponentInferencePolicy {
    Legacy,
    IndividualV2,
    UnifiedV3,
    DirectV4 { gram_probes: u32 },
}

#[derive(Clone, Debug)]
pub struct PreparedStayerAugmentation {
    pub core: PreparedExactStayerHybrid,
    pub memory: StayerAugmentationMemoryReceipt,
}

#[derive(Clone, Debug)]
pub struct ProjectionAugmentationReceipt {
    pub rows: u64,
    pub columns: u64,
    pub caller_copy_bytes: u64,
    pub augmentation_peak_forecast_bytes: u64,
    pub projection_persistent_bytes: u64,
    pub total_prepared_resident_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct PreparedProjectionAugmentation {
    pub core: PreparedProjection,
    pub receipt: ProjectionAugmentationReceipt,
}

#[derive(Clone, Debug)]
pub struct ComponentInferenceAugmentationReceipt {
    pub rows: u64,
    pub variance_source: ComponentVarianceSource,
    pub augmentation_peak_forecast_bytes: u64,
    pub component_persistent_bytes: u64,
    pub total_prepared_resident_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct PreparedComponentInferenceAugmentation {
    pub core: PreparedComponentInference,
    pub receipt: ComponentInferenceAugmentationReceipt,
}

/// Diagnostic-only native wall-clock phases. These values never participate
/// in request admission, plan selection, numerical work, RNG accounting, or
/// result reconciliation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativePhaseTimings {
    pub ingest_ns: u64,
    pub canonicalize_ns: u64,
    pub graph_ns: u64,
    pub compress_ns: u64,
    pub plan_ns: u64,
    pub stayer_augmentation_ns: u64,
    pub solve_ns: u64,
}

impl NativePhaseTimings {
    #[must_use]
    pub fn total_ns(self) -> u64 {
        self.ingest_ns
            .saturating_add(self.canonicalize_ns)
            .saturating_add(self.graph_ns)
            .saturating_add(self.compress_ns)
            .saturating_add(self.plan_ns)
            .saturating_add(self.stayer_augmentation_ns)
            .saturating_add(self.solve_ns)
    }
}

#[must_use]
pub(crate) fn duration_ns(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[derive(Clone, Debug)]
pub struct PreparedProblemWithMask {
    pub problem: CompressedProblem,
    pub plan: Option<JlaPlan>,
    pub deletion: DeletionMode,
    pub retained: Arc<Vec<bool>>,
    pub receipt: PreparationReceipt,
    pub stayer_augmentation: Option<PreparedStayerAugmentation>,
    pub projection: Option<PreparedProjectionAugmentation>,
    pub component_inference: Option<PreparedComponentInferenceAugmentation>,
    pub performance: NativePhaseTimings,
}

impl PreparedProblemWithMask {
    pub fn from_columns(columns: InputColumns) -> Result<Self> {
        Self::build(
            columns,
            None,
            DeletionMode::Match,
            false,
            PreparationMemoryReceipt::default(),
            &mut NeverInterrupt,
        )
    }

    pub fn from_columns_and_interrupt(
        columns: InputColumns,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Self::build(
            columns,
            None,
            DeletionMode::Match,
            false,
            PreparationMemoryReceipt::default(),
            interrupt,
        )
    }

    pub fn from_columns_with_memory(
        columns: InputColumns,
        memory: PreparationMemoryReceipt,
    ) -> Result<Self> {
        if memory.hard_limit_bytes == 0 || memory.preparation_peak_forecast_bytes == 0 {
            return Err(BackendError::invalid(
                "engine_memory",
                "admitted preparation memory receipt is incomplete",
            ));
        }
        Self::build(
            columns,
            None,
            DeletionMode::Match,
            false,
            memory,
            &mut NeverInterrupt,
        )
    }

    pub fn from_columns_with_memory_and_interrupt(
        columns: InputColumns,
        memory: PreparationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if memory.hard_limit_bytes == 0 || memory.preparation_peak_forecast_bytes == 0 {
            return Err(BackendError::invalid(
                "engine_memory",
                "admitted preparation memory receipt is incomplete",
            ));
        }
        Self::build(columns, None, DeletionMode::Match, false, memory, interrupt)
    }

    pub fn from_columns_with_mode_and_memory_and_interrupt(
        columns: InputColumns,
        deletion: DeletionMode,
        memory: PreparationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if memory.hard_limit_bytes == 0 || memory.preparation_peak_forecast_bytes == 0 {
            return Err(BackendError::invalid(
                "engine_memory",
                "admitted preparation memory receipt is incomplete",
            ));
        }
        Self::build(columns, None, deletion, false, memory, interrupt)
    }

    pub fn from_columns_with_probe_order_mode_memory_and_interrupt(
        columns: InputColumns,
        probe_order: Option<Vec<f64>>,
        deletion: DeletionMode,
        memory: PreparationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if memory.hard_limit_bytes == 0 || memory.preparation_peak_forecast_bytes == 0 {
            return Err(BackendError::invalid(
                "engine_memory",
                "admitted preparation memory receipt is incomplete",
            ));
        }
        Self::build(columns, probe_order, deletion, false, memory, interrupt)
    }

    pub fn from_columns_with_probe_order_mode_implicit_match_memory_and_interrupt(
        columns: InputColumns,
        probe_order: Option<Vec<f64>>,
        deletion: DeletionMode,
        implicit_match: bool,
        memory: PreparationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if memory.hard_limit_bytes == 0 || memory.preparation_peak_forecast_bytes == 0 {
            return Err(BackendError::invalid(
                "engine_memory",
                "admitted preparation memory receipt is incomplete",
            ));
        }
        if implicit_match && deletion != DeletionMode::Match {
            return Err(BackendError::invalid(
                "session_prepare",
                "implicit-match preparation requires match deletion",
            ));
        }
        Self::build(
            columns,
            probe_order,
            deletion,
            implicit_match,
            memory,
            interrupt,
        )
    }

    fn build(
        columns: InputColumns,
        probe_order: Option<Vec<f64>>,
        deletion: DeletionMode,
        implicit_match: bool,
        mut memory: PreparationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("session_prepare_entry")?;
        let mut performance = NativePhaseTimings::default();
        let input_rows = to_u64(columns.worker.len(), "input row count")?;
        if let Some(values) = &probe_order {
            if values.len() != columns.worker.len() {
                return Err(BackendError::invalid(
                    "session_prepare",
                    "probe-order column has the wrong row dimension",
                ));
            }
            for (row, &value) in values.iter().enumerate() {
                checkpoint_chunk(interrupt, row, "session_prepare_probe_order")?;
                if !value.is_finite() {
                    return Err(BackendError::invalid(
                        "session_prepare",
                        format!("probe-order value is nonfinite at zero-based row {row}"),
                    ));
                }
            }
        }
        let canonical_start = Instant::now();
        let canonical = CanonicalInput::from_validated_with_implicit_match_and_interrupt(
            columns.validate_with_interrupt(interrupt)?,
            implicit_match,
            interrupt,
        )?;
        performance.canonicalize_ns = duration_ns(canonical_start.elapsed());
        let graph_start = Instant::now();
        let selection = match deletion {
            DeletionMode::Match => select_match_deletion_graph_with_implicit_match_and_interrupt(
                &canonical,
                implicit_match,
                interrupt,
            )?,
            DeletionMode::Observation => {
                select_observation_deletion_graph_with_interrupt(&canonical, interrupt)?
            }
        };
        performance.graph_ns = duration_ns(graph_start.elapsed());
        let graph = selection.receipt;
        if selection.active.len()
            != usize::try_from(input_rows).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_prepare",
                    "input row count is not representable as usize",
                )
            })?
        {
            return Err(BackendError::invariant(
                "session_prepare",
                "graph-selection mask has the wrong row dimension",
            ));
        }
        let retained = Arc::new(selection.active);
        let compress_start = Instant::now();
        let mut problem = canonical.compress_with_implicit_match_and_interrupt(
            retained.as_slice(),
            implicit_match,
            interrupt,
        )?;
        if let Some(values) = probe_order {
            let mut retained_probe_order = Vec::with_capacity(problem.retained_rows.len());
            for (local, &source_row) in problem.retained_rows.iter().enumerate() {
                checkpoint_chunk(interrupt, local, "session_prepare_probe_order_retain")?;
                retained_probe_order.push(values[source_row]);
            }
            problem.probe_order = Some(retained_probe_order);
        }
        performance.compress_ns = duration_ns(compress_start.elapsed());
        let plan_start = Instant::now();
        let plan = if deletion == DeletionMode::Match && problem.controls.is_empty() {
            Some(
                JlaPlan::build_no_controls_with_certified_match_and_interrupt(
                    &problem,
                    implicit_match,
                    interrupt,
                )?,
            )
        } else {
            None
        };
        performance.plan_ns = duration_ns(plan_start.elapsed());
        let mut retained_rows = 0_usize;
        for (row, &value) in retained.iter().enumerate() {
            checkpoint_chunk(interrupt, row, "session_prepare_retained_reconcile")?;
            retained_rows += usize::from(value);
        }
        if retained_rows != problem.outcome.len() {
            return Err(BackendError::invariant(
                "session_prepare",
                "retention mask and compressed row count disagree",
            ));
        }
        if memory.hard_limit_bytes != 0 {
            interrupt.checkpoint("session_prepare_resident_reconcile")?;
            let retained_bytes = to_u64(
                bit_packed_capacity_bytes(retained.capacity()),
                "bit-packed retained-mask capacity",
            )?;
            let problem_bytes = match plan.as_ref() {
                Some(plan) => vckss_core::engine::prepared_problem_bytes(&problem, plan)?,
                None => vckss_core::engine::compressed_problem_bytes(&problem)?,
            };
            memory.prepared_resident_bytes =
                problem_bytes.checked_add(retained_bytes).ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "engine_memory",
                        "prepared resident byte count overflow",
                    )
                })?;
            let simultaneous = memory
                .caller_copy_bytes
                .checked_add(memory.prepared_resident_bytes)
                .ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "engine_memory",
                        "simultaneous caller/prepared byte count overflow",
                    )
                })?;
            if simultaneous > memory.hard_limit_bytes {
                return Err(BackendError::new(
                    ErrorCode::ResourceLimit,
                    "engine_memory",
                    format!(
                        "simultaneous caller copy and prepared resident allocation {simultaneous} bytes exceeds the declared limit {} bytes",
                        memory.hard_limit_bytes
                    ),
                ));
            }
        }
        let receipt = PreparationReceipt {
            input_rows,
            retained_rows: to_u64(retained_rows, "retained row count")?,
            workers: to_u64(problem.workers(), "worker count")?,
            firms: to_u64(problem.firms(), "firm count")?,
            cells: to_u64(problem.cells(), "cell count")?,
            deletion_units: match deletion {
                DeletionMode::Match => to_u64(problem.deletion_units(), "deletion-unit count")?,
                DeletionMode::Observation => problem.physical_total,
            },
            target_strata: to_u64(
                plan.as_ref().map_or_else(
                    || usize::try_from(problem.dimensions.target_strata).expect("target strata"),
                    JlaPlan::target_strata,
                ),
                "target-stratum count",
            )?,
            target_weight_sum: problem.target_total,
            graph,
            memory,
        };
        interrupt.checkpoint("session_prepare_final")?;
        Ok(Self {
            problem,
            plan,
            deletion,
            retained,
            receipt,
            stayer_augmentation: None,
            projection: None,
            component_inference: None,
            performance,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn augment_projection_with_interrupt(
        &mut self,
        project: Vec<Vec<f64>>,
        effect: ProjectionEffect,
        weight: ProjectionWeight,
        rank_tolerance: f64,
        caller_copy_bytes: u64,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        interrupt.checkpoint("session_projection_augmentation_entry")?;
        if self.projection.is_some() {
            return Err(BackendError::new(
                ErrorCode::ContextPoisoned,
                "session_projection_augmentation",
                "the prepared generation already owns a projection",
            ));
        }
        if self.component_inference.is_some() {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "session_projection_augmentation",
                "component inference and fixed-effect projection cannot share one generation",
            ));
        }
        let projection_problem = self
            .stayer_augmentation
            .as_ref()
            .map_or(&self.problem, |augmentation| &augmentation.core.problem);
        let rows = to_u64(projection_problem.outcome.len(), "projection retained rows")?;
        let project_count = project.len();
        let columns_usize = project_count.checked_add(1).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "session_projection_augmentation",
                "projection column count overflow",
            )
        })?;
        let columns = to_u64(columns_usize, "projection columns")?;
        let expected_caller_copy_bytes = rows
            .checked_mul(to_u64(project_count, "supplied projection columns")?)
            .and_then(|value| value.checked_mul(core::mem::size_of::<f64>() as u64))
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_projection_augmentation",
                    "projection caller-copy byte count overflow",
                )
            })?;
        if caller_copy_bytes != expected_caller_copy_bytes {
            return Err(BackendError::invalid(
                "session_projection_augmentation",
                "projection caller-copy bytes do not match the supplied dimensions",
            ));
        }
        let old_resident = self.stayer_augmentation.as_ref().map_or(
            self.receipt.memory.prepared_resident_bytes,
            |augmentation| augmentation.memory.total_prepared_resident_bytes,
        );
        let expected_persistent_bytes = to_u64(
            projection_problem
                .workers()
                .checked_add(projection_problem.firms())
                .and_then(|value| value.checked_mul(columns_usize))
                .ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "session_projection_augmentation",
                        "projection persistent dimension overflow",
                    )
                })?,
            "projection persistent values",
        )?
        .checked_mul(core::mem::size_of::<f64>() as u64)
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "session_projection_augmentation",
                "projection persistent byte count overflow",
            )
        })?;
        let projection_square_bytes = columns
            .checked_mul(columns)
            .and_then(|value| value.checked_mul(core::mem::size_of::<f64>() as u64))
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_projection_augmentation",
                    "projection dense-work byte count overflow",
                )
            })?;
        // The synchronous boundary owns the C caller copy and the Rust column
        // copy at the same time.  Four persistent-size blocks cover the
        // stable coefficient-space cross-products plus the final RHSs; eight
        // q-square blocks cover the Gram conversion, scaling, spectrum,
        // inverse, and small-vector headers conservatively.
        let augmentation_peak_forecast_bytes = old_resident
            .checked_add(caller_copy_bytes.checked_mul(2).ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_projection_augmentation",
                    "projection synchronous-copy byte count overflow",
                )
            })?)
            .and_then(|value| value.checked_add(expected_persistent_bytes.checked_mul(4)?))
            .and_then(|value| value.checked_add(projection_square_bytes.checked_mul(8)?))
            .and_then(|value| value.checked_add(4096))
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_projection_augmentation",
                    "projection augmentation peak overflow",
                )
            })?;
        let total_prepared_resident_bytes = old_resident
            .checked_add(expected_persistent_bytes)
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_projection_augmentation",
                    "projection prepared-resident byte count overflow",
                )
            })?;
        let hard_limit = self.receipt.memory.hard_limit_bytes;
        if hard_limit == 0
            || total_prepared_resident_bytes > hard_limit
            || augmentation_peak_forecast_bytes > hard_limit
        {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "session_projection_augmentation",
                "projection augmentation exceeds the declared whole-command memory limit",
            ));
        }
        let core = prepare_projection_with_interrupt(
            projection_problem,
            &project,
            effect,
            weight,
            rank_tolerance,
            interrupt,
        )?;
        drop(project);
        if core.persistent_bytes != expected_persistent_bytes {
            return Err(BackendError::invariant(
                "session_projection_augmentation",
                "projection persistent bytes do not reconcile with its dimensions",
            ));
        }
        self.receipt.memory.prepared_resident_bytes = total_prepared_resident_bytes;
        self.receipt.memory.preparation_peak_forecast_bytes = self
            .receipt
            .memory
            .preparation_peak_forecast_bytes
            .max(augmentation_peak_forecast_bytes);
        self.projection = Some(PreparedProjectionAugmentation {
            receipt: ProjectionAugmentationReceipt {
                rows,
                columns,
                caller_copy_bytes,
                augmentation_peak_forecast_bytes,
                projection_persistent_bytes: core.persistent_bytes,
                total_prepared_resident_bytes,
            },
            core,
        });
        interrupt.checkpoint("session_projection_augmentation_final")
    }

    pub fn augment_component_inference_with_interrupt(
        &mut self,
        inference_unit: ComponentInferenceUnit,
        variance_source: ComponentVarianceSource,
        options: ComponentInferenceOptions,
        structured_options: StructuredVarianceOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        self.augment_component_inference_with_reporting(
            inference_unit,
            variance_source,
            options,
            structured_options,
            ComponentInferencePolicy::Legacy,
            interrupt,
        )
    }

    pub fn augment_component_inference_v2_with_interrupt(
        &mut self,
        inference_unit: ComponentInferenceUnit,
        variance_source: ComponentVarianceSource,
        options: ComponentInferenceOptions,
        structured_options: StructuredVarianceOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        self.augment_component_inference_with_reporting(
            inference_unit,
            variance_source,
            options,
            structured_options,
            ComponentInferencePolicy::IndividualV2,
            interrupt,
        )
    }

    pub(crate) fn augment_component_inference_with_reporting(
        &mut self,
        inference_unit: ComponentInferenceUnit,
        variance_source: ComponentVarianceSource,
        options: ComponentInferenceOptions,
        structured_options: StructuredVarianceOptions,
        policy: ComponentInferencePolicy,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        interrupt.checkpoint("session_component_inference_augmentation_entry")?;
        if self.component_inference.is_some() {
            return Err(BackendError::new(
                ErrorCode::ContextPoisoned,
                "session_component_inference_augmentation",
                "the prepared generation already owns component inference",
            ));
        }
        if self.projection.is_some() || self.stayer_augmentation.is_some() {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "session_component_inference_augmentation",
                "structured component inference requires a mover-only generation without projection",
            ));
        }
        let expected_deletion = match inference_unit {
            ComponentInferenceUnit::Observation => DeletionMode::Observation,
            ComponentInferenceUnit::Match => DeletionMode::Match,
        };
        if self.deletion != expected_deletion {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "session_component_inference_augmentation",
                "component inference units disagree with the prepared deletion mode",
            ));
        }
        let rows = to_u64(
            self.problem.outcome.len(),
            "component inference retained rows",
        )?;
        let old_resident = self.receipt.memory.prepared_resident_bytes;
        // Preparation validates every retained row and publishes only a small
        // options object. The row-level fitted variance state belongs to the
        // already-conservative generic-JLA attachment peak, not this boundary.
        let augmentation_peak_forecast_bytes = old_resident.checked_add(4096).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "session_component_inference_augmentation",
                "component-inference augmentation peak overflow",
            )
        })?;
        let hard_limit = self.receipt.memory.hard_limit_bytes;
        if hard_limit == 0 || augmentation_peak_forecast_bytes > hard_limit {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "session_component_inference_augmentation",
                "component-inference augmentation exceeds the whole-command memory limit",
            ));
        }
        let individual = policy != ComponentInferencePolicy::Legacy;
        let mut core = if let ComponentInferencePolicy::DirectV4 { gram_probes } = policy {
            vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
                &self.problem,
                inference_unit,
                variance_source,
                options,
                structured_options,
                gram_probes,
                interrupt,
            )?
        } else if policy == ComponentInferencePolicy::UnifiedV3 {
            vckss_core::residual_moment_inference::prepare_unified_with_interrupt(
                &self.problem,
                inference_unit,
                variance_source,
                options,
                structured_options,
                interrupt,
            )?
        } else {
            match inference_unit {
                ComponentInferenceUnit::Observation if individual => {
                    vckss_core::residual_moment_inference::prepare_default_with_interrupt(
                        &self.problem,
                        variance_source,
                        options,
                        structured_options,
                        interrupt,
                    )?
                }
                ComponentInferenceUnit::Observation => {
                    prepare_structured_component_inference_with_interrupt(
                        &self.problem,
                        variance_source,
                        options,
                        structured_options,
                        interrupt,
                    )?
                }
                ComponentInferenceUnit::Match => prepare_grouped_structured_component_inference(
                    &self.problem,
                    variance_source,
                    options,
                    structured_options,
                )?,
            }
        };
        core.individual_intervals = individual;
        let total_prepared_resident_bytes = old_resident
            .checked_add(core.persistent_bytes)
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_component_inference_augmentation",
                    "component-inference prepared-resident byte count overflow",
                )
            })?;
        self.receipt.memory.prepared_resident_bytes = total_prepared_resident_bytes;
        self.receipt.memory.preparation_peak_forecast_bytes = self
            .receipt
            .memory
            .preparation_peak_forecast_bytes
            .max(augmentation_peak_forecast_bytes);
        self.component_inference = Some(PreparedComponentInferenceAugmentation {
            receipt: ComponentInferenceAugmentationReceipt {
                rows,
                variance_source,
                augmentation_peak_forecast_bytes,
                component_persistent_bytes: core.persistent_bytes,
                total_prepared_resident_bytes,
            },
            core,
        });
        interrupt.checkpoint("session_component_inference_augmentation_final")
    }

    pub fn augment_stayers_with_memory_and_interrupt(
        &mut self,
        input: StayerAugmentationInput,
        mut memory: StayerAugmentationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        interrupt.checkpoint("session_stayer_augmentation_entry")?;
        if self.component_inference.is_some() {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "session_stayer_augmentation",
                "structured component inference does not support stayer augmentation",
            ));
        }
        let augmentation_start = Instant::now();
        if self.deletion != DeletionMode::Match {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "session_stayer_augmentation",
                "the stayer hybrid requires match-deletion mover preparation",
            ));
        }
        if self.stayer_augmentation.is_some() {
            return Err(BackendError::new(
                ErrorCode::ContextPoisoned,
                "session_stayer_augmentation",
                "the prepared generation already owns a stayer augmentation",
            ));
        }
        if memory.hard_limit_bytes == 0
            || memory.hard_limit_bytes != self.receipt.memory.hard_limit_bytes
            || memory.total_prepared_resident_bytes != self.receipt.memory.prepared_resident_bytes
        {
            return Err(BackendError::invalid(
                "session_stayer_augmentation",
                "the stayer memory admission does not reconcile with mover preparation",
            ));
        }
        let core = prepare_exact_stayer_hybrid_with_interrupt(&self.problem, input, interrupt)?;
        let hybrid_problem_bytes = vckss_core::engine::compressed_problem_bytes(&core.problem)?;
        let stayer_mask_bytes = u64::try_from(core.plan.stayer_rows.capacity())
            .map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_stayer_augmentation",
                    "stayer-mask capacity is not representable",
                )
            })?
            .checked_mul(core::mem::size_of::<bool>() as u64)
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_stayer_augmentation",
                    "stayer-mask resident byte count overflow",
                )
            })?;
        memory.augmented_resident_bytes = hybrid_problem_bytes
            .checked_add(stayer_mask_bytes)
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_stayer_augmentation",
                    "augmented resident byte count overflow",
                )
            })?;
        memory.total_prepared_resident_bytes = self
            .receipt
            .memory
            .prepared_resident_bytes
            .checked_add(memory.augmented_resident_bytes)
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_stayer_augmentation",
                    "total prepared resident byte count overflow",
                )
            })?;
        let simultaneous = memory
            .caller_copy_bytes
            .checked_add(memory.total_prepared_resident_bytes)
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_stayer_augmentation",
                    "simultaneous stayer caller/resident byte count overflow",
                )
            })?;
        if simultaneous > memory.hard_limit_bytes {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "session_stayer_augmentation",
                format!(
                    "simultaneous stayer caller and prepared resident allocation {simultaneous} bytes exceeds the declared limit {} bytes",
                    memory.hard_limit_bytes
                ),
            ));
        }
        self.stayer_augmentation = Some(PreparedStayerAugmentation { core, memory });
        self.performance.stayer_augmentation_ns = duration_ns(augmentation_start.elapsed());
        interrupt.checkpoint("session_stayer_augmentation_final")
    }
}

pub(crate) const fn bit_packed_capacity_bytes(bit_capacity: usize) -> usize {
    bit_capacity.div_ceil(8)
}

#[cfg(test)]
mod tests {
    use super::bit_packed_capacity_bytes;

    #[test]
    fn bit_packed_capacity_rounds_only_at_byte_boundaries() {
        assert_eq!(bit_packed_capacity_bytes(0), 0);
        assert_eq!(bit_packed_capacity_bytes(1), 1);
        assert_eq!(bit_packed_capacity_bytes(7), 1);
        assert_eq!(bit_packed_capacity_bytes(8), 1);
        assert_eq!(bit_packed_capacity_bytes(9), 2);
        assert_eq!(
            bit_packed_capacity_bytes(usize::MAX),
            usize::MAX / 8 + usize::from(usize::MAX % 8 != 0)
        );
    }
}

#[derive(Debug)]
pub struct RetainedNativeSession<Output> {
    registry: ContextRegistry<PreparedProblemWithMask, Output>,
}

impl<Output> Default for RetainedNativeSession<Output> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Output> RetainedNativeSession<Output> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            registry: ContextRegistry::new(),
        }
    }

    pub fn prepare(&mut self, columns: InputColumns) -> Result<ContextHandle> {
        self.registry
            .prepare(PreparedProblemWithMask::from_columns(columns)?)
    }

    pub fn solve<F>(&mut self, handle: ContextHandle, solve: F) -> Result<()>
    where
        F: FnOnce(PreparedProblemWithMask) -> Result<Output>,
    {
        self.registry.solve(handle, solve)
    }

    pub fn result(&self, handle: ContextHandle) -> Result<&Output> {
        self.registry.result(handle)
    }

    pub fn release(&mut self, handle: ContextHandle) -> Result<bool> {
        self.registry.release(handle)
    }

    pub fn clear_abandoned(&mut self) -> Option<ContextHandle> {
        self.registry.clear_abandoned()
    }

    #[must_use]
    pub fn snapshot(&self) -> ContextSnapshot {
        self.registry.snapshot()
    }
}

fn to_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "session_prepare",
            format!("{label} is not representable as u64"),
        )
    })
}
