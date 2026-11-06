//! Certified quotient-space preconditioned conjugate gradients.

use crate::cancel::{Meter, chunked};
use crate::cmg_profile::CycleRecorder;
use crate::components::ComponentWorkspace;
use crate::execution::Team;
use crate::graph::compensated_add;
#[cfg(feature = "parallel")]
use crate::graph::compensated_sum;
use crate::preconditioner::{One, with_meter};
use crate::{
    ApplyContext, Cancellation, CmgError, CmgPreconditioner, CmgWorkspace, Laplacian, PcgOptions,
};

crate::storage::storage_fields!(
    PcgWorkspace,
    projected_rhs,
    solution,
    residual,
    preconditioned,
    direction,
    matrix_direction,
    component,
    cmg
);
#[cfg(not(feature = "parallel"))]
crate::storage::storage_fields!(PcgBatchWorkspace, pcg, rhs, guess);
#[cfg(feature = "parallel")]
crate::storage::storage_fields!(PcgBatchWorkspace, pcg, rhs, guess, outcome);
#[cfg(feature = "parallel")]
use crate::ParallelCmgPlan;
#[cfg(feature = "profiling")]
use std::time::Instant;

/// Reusable vectors for repeated PCG solves with one preconditioner.
#[derive(Debug, Clone)]
pub struct PcgWorkspace {
    projected_rhs: Vec<f64>,
    solution: Vec<f64>,
    residual: Vec<f64>,
    preconditioned: Vec<f64>,
    direction: Vec<f64>,
    matrix_direction: Vec<f64>,
    component: ComponentWorkspace,
    cmg: CmgWorkspace,
}

/// Solution-free diagnostics returned by caller-buffer PCG entry points.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PcgDiagnostics {
    iterations: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    relative_residual: f64,
    backward_error: f64,
    tolerance: f64,
    restarts: usize,
    rhs_projection_norm: f64,
}

/// Untimed-use batch phase attribution for caller-buffer solves.
#[cfg(feature = "profiling")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PcgBatchPhaseProfile {
    validation_nanoseconds: u128,
    gather_nanoseconds: u128,
    solve_nanoseconds: u128,
    scatter_nanoseconds: u128,
    result_construction_nanoseconds: u128,
    total_nanoseconds: u128,
    cycle: (),
}

#[cfg(feature = "profiling")]
impl PcgBatchPhaseProfile {
    /// Return batch view, dimension, compatibility, and workspace validation time.
    #[must_use]
    pub const fn validation_nanoseconds(self) -> u128 {
        self.validation_nanoseconds
    }

    /// Return time spent gathering noncontiguous RHS and guesses.
    #[must_use]
    pub const fn gather_nanoseconds(self) -> u128 {
        self.gather_nanoseconds
    }

    /// Return time spent in the shared certified numerical core.
    #[must_use]
    pub const fn solve_nanoseconds(self) -> u128 {
        self.solve_nanoseconds
    }

    /// Return time spent scattering solutions to caller layouts.
    #[must_use]
    pub const fn scatter_nanoseconds(self) -> u128 {
        self.scatter_nanoseconds
    }

    /// Return owned-result construction time, zero for this caller-buffer path.
    #[must_use]
    pub const fn result_construction_nanoseconds(self) -> u128 {
        self.result_construction_nanoseconds
    }

    /// Return complete profiled call wall time.
    #[must_use]
    pub const fn total_nanoseconds(self) -> u128 {
        self.total_nanoseconds
    }
}

/// Reusable single-RHS PCG workspace plus strided batch staging buffers.
#[derive(Debug, Clone)]
pub struct PcgBatchWorkspace {
    pcg: PcgWorkspace,
    rhs: Vec<f64>,
    guess: Vec<f64>,
    #[cfg(feature = "parallel")]
    outcome: Option<Result<PcgDiagnostics, CmgError>>,
}

impl PcgBatchWorkspace {
    /// Requested heap layouts for one new batch workspace, including all nested
    /// descriptor banks. Excludes the inline root and the borrowed preconditioner.
    pub fn allocation_bytes(preconditioner: &CmgPreconditioner) -> Result<usize, CmgError> {
        Self::allocation_bytes_metered(preconditioner, &mut crate::cancel::Unmetered)
    }

    fn allocation_bytes_metered<M: Meter>(
        preconditioner: &CmgPreconditioner,
        meter: &mut M,
    ) -> Result<usize, CmgError> {
        use crate::storage::{add, bytes};
        let n = preconditioner.hierarchy().levels()[0]
            .graph()
            .vertex_count();
        add(
            PcgWorkspace::allocation_bytes_metered(preconditioner, meter)?,
            bytes::<[f64; 2]>(n)?,
        )
    }

    /// Admit all requested layouts before allocating a workspace. The budget is
    /// incremental: callers reserve simultaneous parents/old banks separately.
    /// Allocator overhead is excluded; allocation failure is returned normally.
    pub fn new_with_budget(
        preconditioner: &CmgPreconditioner,
        budget_bytes: usize,
    ) -> Result<Self, CmgError> {
        crate::storage::admit(Self::allocation_bytes(preconditioner)?, budget_bytes)?;
        Self::new(preconditioner)
    }

    /// Requested layouts for a bank, including its outer descriptor allocation.
    pub fn bank_allocation_bytes(
        preconditioner: &CmgPreconditioner,
        count: usize,
    ) -> Result<usize, CmgError> {
        Self::bank_allocation_bytes_with(preconditioner, count, None)
    }

    /// [`Self::bank_allocation_bytes`] with an optional cancellation query:
    /// the same value and errors, polling between bounded layout steps and
    /// once before returning. A true query returns [`CmgError::Cancelled`].
    pub fn bank_allocation_bytes_with(
        preconditioner: &CmgPreconditioner,
        count: usize,
        cancel: Option<crate::Cancellation<'_>>,
    ) -> Result<usize, CmgError> {
        with_meter!(cancel, meter => Self::bank_allocation_bytes_metered(preconditioner, count, meter))
    }

    fn bank_allocation_bytes_metered<M: Meter>(
        preconditioner: &CmgPreconditioner,
        count: usize,
        meter: &mut M,
    ) -> Result<usize, CmgError> {
        let payload = Self::allocation_bytes_metered(preconditioner, meter)?
            .checked_mul(count)
            .ok_or(CmgError::AllocationFailed {
                context: "PCG bank size overflows",
            })?;
        crate::storage::add(payload, crate::storage::bytes::<Self>(count)?)
    }

    /// Prepare a complete fixed bank fallibly after admission, without lazy
    /// growth or replacing an existing bank. On failure partial banks are dropped.
    pub fn new_bank_with_budget(
        preconditioner: &CmgPreconditioner,
        count: usize,
        budget_bytes: usize,
    ) -> Result<Vec<Self>, CmgError> {
        Self::new_bank_with(
            preconditioner,
            count,
            crate::BuildContext {
                budget_bytes: Some(budget_bytes),
                ..crate::BuildContext::default()
            },
        )
    }

    /// Prepare a fixed bank under a context; its team is unused. A budget
    /// admits [`Self::bank_allocation_bytes`] before the first allocation.
    /// With a cancellation query every vector, component, centering and CMG
    /// level bank polls before allocating and while it is zero-filled, and
    /// the query is polled once more before the bank is returned. A true query
    /// drops every partial workspace and returns [`CmgError::Cancelled`].
    /// Layout, contents and other errors never depend on the context.
    pub fn new_bank_with(
        preconditioner: &CmgPreconditioner,
        count: usize,
        context: crate::BuildContext<'_>,
    ) -> Result<Vec<Self>, CmgError> {
        with_meter!(context.cancel, meter => (|| {
            if let Some(budget) = context.budget_bytes {
                crate::storage::admit(
                    Self::bank_allocation_bytes_metered(preconditioner, count, meter)?,
                    budget,
                )?;
            }
            let mut bank = crate::cancel::try_reserved(meter, count, "PCG batch workspace bank")?;
            for _ in 0..count {
                meter.work("PCG batch workspace bank", 16)?;
                bank.push(Self::new_metered(preconditioner, meter)?);
            }
            Ok(bank)
        })())
    }

    /// Allocate a workspace for contiguous or strided batch solves.
    pub fn new(preconditioner: &CmgPreconditioner) -> Result<Self, CmgError> {
        Self::new_metered(preconditioner, &mut crate::cancel::Unmetered)
    }

    fn new_metered<M: Meter>(
        preconditioner: &CmgPreconditioner,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        let dimension = preconditioner.hierarchy().levels()[0]
            .graph()
            .vertex_count();
        let rhs = crate::cancel::try_filled(meter, dimension, 0.0, "PCG batch RHS staging")?;
        let guess = crate::cancel::try_filled(meter, dimension, 0.0, "PCG batch guess staging")?;
        Ok(Self {
            pcg: PcgWorkspace::try_new_metered(preconditioner, meter)?,
            rhs,
            guess,
            #[cfg(feature = "parallel")]
            outcome: None,
        })
    }

    /// Return the system dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.pcg.dimension()
    }

    /// Return principal retained bytes for numerical and staging arrays.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.pcg
            .byte_len()
            .saturating_add(self.rhs.capacity().saturating_mul(8))
            .saturating_add(self.guess.capacity().saturating_mul(8))
    }

    pub(crate) fn validate(&self, preconditioner: &CmgPreconditioner) -> Result<(), CmgError> {
        self.pcg.validate(preconditioner)?;
        for (context, actual) in [
            ("PcgBatchWorkspace RHS staging", self.rhs.len()),
            ("PcgBatchWorkspace guess staging", self.guess.len()),
        ] {
            if actual != self.pcg.dimension() {
                return Err(CmgError::dimension(context, self.pcg.dimension(), actual));
            }
        }
        Ok(())
    }
}

impl PcgDiagnostics {
    /// Return completed PCG iterations.
    #[must_use]
    pub const fn iterations(self) -> usize {
        self.iterations
    }

    /// Return the Euclidean residual norm at initialization.
    #[must_use]
    pub const fn initial_residual_norm(self) -> f64 {
        self.initial_residual_norm
    }

    /// Return the freshly recomputed residual norm against the submitted RHS.
    #[must_use]
    pub const fn residual_norm(self) -> f64 {
        self.residual_norm
    }

    /// Return `||r|| / ||b||`, with zero denominator handled explicitly.
    #[must_use]
    pub const fn relative_residual(self) -> f64 {
        self.relative_residual
    }

    /// Return `||r|| / (||b|| + ||A||_bound ||x||)`.
    #[must_use]
    pub const fn backward_error(self) -> f64 {
        self.backward_error
    }

    /// Return the absolute residual threshold used for certification.
    #[must_use]
    pub const fn tolerance(self) -> f64 {
        self.tolerance
    }

    /// Return the number of explicit residual-replacement restarts.
    #[must_use]
    pub const fn restarts(self) -> usize {
        self.restarts
    }

    /// Return the norm of component-nullspace roundoff removed from the RHS.
    #[must_use]
    pub const fn rhs_projection_norm(self) -> f64 {
        self.rhs_projection_norm
    }
}

/// Borrowed read-only batch storage with explicit RHS and value strides.
#[derive(Debug, Clone, Copy)]
pub struct PcgBatchRef<'a> {
    data: &'a [f64],
    rhs_count: usize,
    dimension: usize,
    rhs_stride: usize,
    value_stride: usize,
}

impl<'a> PcgBatchRef<'a> {
    /// Borrow a tightly packed RHS-major batch.
    pub fn contiguous(
        data: &'a [f64],
        rhs_count: usize,
        dimension: usize,
    ) -> Result<Self, CmgError> {
        let expected = rhs_count
            .checked_mul(dimension)
            .ok_or(CmgError::InvalidHierarchy {
                context: "PCG batch dimensions overflow",
            })?;
        if data.len() != expected {
            return Err(CmgError::dimension(
                "PcgBatchRef contiguous storage",
                expected,
                data.len(),
            ));
        }
        Self::strided(data, rhs_count, dimension, dimension.max(1), 1)
    }

    /// Borrow a batch whose logical value `(rhs, vertex)` is stored at
    /// `rhs * rhs_stride + vertex * value_stride`.
    pub fn strided(
        data: &'a [f64],
        rhs_count: usize,
        dimension: usize,
        rhs_stride: usize,
        value_stride: usize,
    ) -> Result<Self, CmgError> {
        let required = batch_required_len(rhs_count, dimension, rhs_stride, value_stride)?;
        if data.len() < required {
            return Err(CmgError::dimension(
                "PcgBatchRef strided storage",
                required,
                data.len(),
            ));
        }
        Ok(Self {
            data,
            rhs_count,
            dimension,
            rhs_stride,
            value_stride,
        })
    }

    /// Return the number of logical right-hand sides.
    #[must_use]
    pub const fn rhs_count(self) -> usize {
        self.rhs_count
    }

    /// Return the logical vector dimension.
    #[must_use]
    pub const fn dimension(self) -> usize {
        self.dimension
    }

    /// Return the storage stride between adjacent logical right-hand sides.
    #[must_use]
    pub const fn rhs_stride(self) -> usize {
        self.rhs_stride
    }

    /// Return the storage stride between adjacent vertices within one RHS.
    #[must_use]
    pub const fn value_stride(self) -> usize {
        self.value_stride
    }

    /// One right-hand side: the borrowed slice when values are contiguous,
    /// else a counted copy into `staging`.
    fn gather<'s, M: Meter>(
        self,
        rhs: usize,
        staging: &'s mut [f64],
        meter: &mut M,
    ) -> Result<&'s [f64], CmgError>
    where
        'a: 's,
    {
        debug_assert!(rhs < self.rhs_count);
        if self.value_stride == 1 {
            if self.dimension == 0 {
                return Ok(&self.data[0..0]);
            }
            let start = rhs * self.rhs_stride;
            return Ok(&self.data[start..start + self.dimension]);
        }
        debug_assert_eq!(staging.len(), self.dimension);
        chunked(meter, "PCG batch gather", self.dimension, 1, |range| {
            for vertex in range {
                staging[vertex] = self.data[rhs * self.rhs_stride + vertex * self.value_stride];
            }
            Ok(())
        })?;
        Ok(staging)
    }
}

/// Borrowed writable batch storage with checked, nonoverlapping strides.
#[derive(Debug)]
pub struct PcgBatchMut<'a> {
    data: &'a mut [f64],
    rhs_count: usize,
    dimension: usize,
    rhs_stride: usize,
    value_stride: usize,
}

impl<'a> PcgBatchMut<'a> {
    /// Borrow a tightly packed RHS-major output batch.
    pub fn contiguous(
        data: &'a mut [f64],
        rhs_count: usize,
        dimension: usize,
    ) -> Result<Self, CmgError> {
        let expected = rhs_count
            .checked_mul(dimension)
            .ok_or(CmgError::InvalidHierarchy {
                context: "PCG batch dimensions overflow",
            })?;
        if data.len() != expected {
            return Err(CmgError::dimension(
                "PcgBatchMut contiguous storage",
                expected,
                data.len(),
            ));
        }
        Self::strided(data, rhs_count, dimension, dimension.max(1), 1)
    }

    /// Borrow a batch whose logical value `(rhs, vertex)` is stored at
    /// `rhs * rhs_stride + vertex * value_stride`.
    ///
    /// The layout must be provably nonoverlapping in RHS-major or
    /// vertex-major order.
    pub fn strided(
        data: &'a mut [f64],
        rhs_count: usize,
        dimension: usize,
        rhs_stride: usize,
        value_stride: usize,
    ) -> Result<Self, CmgError> {
        let required = batch_required_len(rhs_count, dimension, rhs_stride, value_stride)?;
        if data.len() < required {
            return Err(CmgError::dimension(
                "PcgBatchMut strided storage",
                required,
                data.len(),
            ));
        }
        let rhs_span = dimension
            .saturating_sub(1)
            .checked_mul(value_stride)
            .and_then(|value| value.checked_add(1))
            .ok_or(CmgError::InvalidHierarchy {
                context: "PCG batch RHS span overflows",
            })?;
        let vertex_span = rhs_count
            .saturating_sub(1)
            .checked_mul(rhs_stride)
            .and_then(|value| value.checked_add(1))
            .ok_or(CmgError::InvalidHierarchy {
                context: "PCG batch vertex span overflows",
            })?;
        if rhs_count > 1 && dimension > 1 && rhs_stride < rhs_span && value_stride < vertex_span {
            return Err(CmgError::InvalidHierarchy {
                context: "mutable PCG batch layout may overlap",
            });
        }
        Ok(Self {
            data,
            rhs_count,
            dimension,
            rhs_stride,
            value_stride,
        })
    }

    /// Return the number of logical output vectors.
    #[must_use]
    pub const fn rhs_count(&self) -> usize {
        self.rhs_count
    }

    /// Return the logical vector dimension.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// Return the storage stride between adjacent logical output vectors.
    #[must_use]
    pub const fn rhs_stride(&self) -> usize {
        self.rhs_stride
    }

    /// Return the storage stride between adjacent vertices within one output.
    #[must_use]
    pub const fn value_stride(&self) -> usize {
        self.value_stride
    }

    fn scatter<M: Meter>(
        &mut self,
        rhs: usize,
        input: &[f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        debug_assert!(rhs < self.rhs_count);
        debug_assert_eq!(input.len(), self.dimension);
        chunked(meter, "PCG batch scatter", self.dimension, 1, |range| {
            for vertex in range {
                self.data[rhs * self.rhs_stride + vertex * self.value_stride] = input[vertex];
            }
            Ok(())
        })
    }
}

fn batch_required_len(
    rhs_count: usize,
    dimension: usize,
    rhs_stride: usize,
    value_stride: usize,
) -> Result<usize, CmgError> {
    if rhs_count == 0 || dimension == 0 {
        return Ok(0);
    }
    if rhs_stride == 0 || value_stride == 0 {
        return Err(CmgError::InvalidHierarchy {
            context: "nonempty PCG batch strides must be positive",
        });
    }
    let rhs_offset = (rhs_count - 1)
        .checked_mul(rhs_stride)
        .ok_or(CmgError::InvalidHierarchy {
            context: "PCG batch RHS offset overflows",
        })?;
    let value_offset =
        (dimension - 1)
            .checked_mul(value_stride)
            .ok_or(CmgError::InvalidHierarchy {
                context: "PCG batch value offset overflows",
            })?;
    rhs_offset
        .checked_add(value_offset)
        .and_then(|value| value.checked_add(1))
        .ok_or(CmgError::InvalidHierarchy {
            context: "PCG batch storage span overflows",
        })
}

impl PcgWorkspace {
    /// Requested heap layouts including CMG/component scratch and descriptor banks.
    pub fn allocation_bytes(preconditioner: &CmgPreconditioner) -> Result<usize, CmgError> {
        Self::allocation_bytes_metered(preconditioner, &mut crate::cancel::Unmetered)
    }

    fn allocation_bytes_metered<M: Meter>(
        preconditioner: &CmgPreconditioner,
        meter: &mut M,
    ) -> Result<usize, CmgError> {
        use crate::storage::{add, bytes};
        let n = preconditioner.hierarchy().levels()[0]
            .graph()
            .vertex_count();
        add(
            add(
                bytes::<[f64; 6]>(n)?,
                ComponentWorkspace::allocation_bytes(preconditioner.finest_components().count())?,
            )?,
            preconditioner.workspace_allocation_bytes_metered(meter)?,
        )
    }
    #[cfg(feature = "parallel")]
    pub(crate) fn required_bytes(preconditioner: &CmgPreconditioner) -> usize {
        let dimension = preconditioner.hierarchy().levels()[0]
            .graph()
            .vertex_count();
        dimension
            .saturating_mul(core::mem::size_of::<f64>())
            .saturating_mul(6)
            .saturating_add(preconditioner.finest_components().workspace_bytes())
            .saturating_add(preconditioner.workspace_bytes())
    }

    /// Allocate a solver workspace for a fixed preconditioner.
    #[must_use]
    pub fn new(preconditioner: &CmgPreconditioner) -> Self {
        let dimension = preconditioner.hierarchy().levels()[0]
            .graph()
            .vertex_count();
        Self {
            projected_rhs: vec![0.0; dimension],
            solution: vec![0.0; dimension],
            residual: vec![0.0; dimension],
            preconditioned: vec![0.0; dimension],
            direction: vec![0.0; dimension],
            matrix_direction: vec![0.0; dimension],
            component: preconditioner.finest_components().workspace(),
            cmg: preconditioner.workspace(),
        }
    }

    /// Fallibly allocate a solver workspace for a fixed preconditioner.
    pub fn try_new(preconditioner: &CmgPreconditioner) -> Result<Self, CmgError> {
        Self::try_new_metered(preconditioner, &mut crate::cancel::Unmetered)
    }

    /// The one fallible preparation; a polling meter polls before every
    /// allocation and while zero-filling, layout and contents unchanged.
    fn try_new_metered<M: Meter>(
        preconditioner: &CmgPreconditioner,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        let dimension = preconditioner.hierarchy().levels()[0]
            .graph()
            .vertex_count();
        let mut zero = |context| crate::cancel::try_filled(meter, dimension, 0.0, context);
        Ok(Self {
            projected_rhs: zero("PCG projected RHS")?,
            solution: zero("PCG solution")?,
            residual: zero("PCG residual")?,
            preconditioned: zero("PCG preconditioned vector")?,
            direction: zero("PCG direction")?,
            matrix_direction: zero("PCG matrix direction")?,
            component: preconditioner.finest_components().try_workspace(meter)?,
            cmg: preconditioner.try_workspace_metered(meter)?,
        })
    }

    /// Return the system dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.solution.len()
    }

    /// The solution of the most recent successful solve.
    pub(crate) fn solution(&self) -> &[f64] {
        &self.solution
    }

    /// Return the number of heap bytes reserved by the principal work arrays.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.dimension()
            .saturating_mul(8)
            .saturating_mul(6)
            .saturating_add(self.component.byte_len())
            .saturating_add(self.cmg.byte_len())
    }

    fn validate(&self, preconditioner: &CmgPreconditioner) -> Result<(), CmgError> {
        let dimension = preconditioner.hierarchy().levels()[0]
            .graph()
            .vertex_count();
        for (context, actual) in [
            ("PcgWorkspace projected rhs", self.projected_rhs.len()),
            ("PcgWorkspace solution", self.solution.len()),
            ("PcgWorkspace residual", self.residual.len()),
            ("PcgWorkspace preconditioned", self.preconditioned.len()),
            ("PcgWorkspace direction", self.direction.len()),
            ("PcgWorkspace matrix direction", self.matrix_direction.len()),
        ] {
            if actual != dimension {
                return Err(CmgError::dimension(context, dimension, actual));
            }
        }
        preconditioner
            .finest_components()
            .validate_workspace(&self.component)?;
        preconditioner.validate_workspace(&self.cmg)
    }
}

/// A successfully certified PCG solution and its diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct PcgResult {
    solution: Vec<f64>,
    iterations: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    relative_residual: f64,
    backward_error: f64,
    tolerance: f64,
    restarts: usize,
    rhs_projection_norm: f64,
}

crate::storage::storage_fields!(PcgResult, solution);

impl PcgResult {
    /// Return the certified solution.
    #[must_use]
    pub fn solution(&self) -> &[f64] {
        &self.solution
    }

    /// Consume the result and return its solution vector.
    #[must_use]
    pub fn into_solution(self) -> Vec<f64> {
        self.solution
    }

    /// Return completed PCG iterations.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    /// Return the initial Euclidean residual norm for the submitted RHS.
    #[must_use]
    pub const fn initial_residual_norm(&self) -> f64 {
        self.initial_residual_norm
    }

    /// Return the freshly recomputed residual norm against the submitted RHS.
    #[must_use]
    pub const fn residual_norm(&self) -> f64 {
        self.residual_norm
    }

    /// Return `||r|| / ||b||`, with zero denominator handled explicitly.
    #[must_use]
    pub const fn relative_residual(&self) -> f64 {
        self.relative_residual
    }

    /// Return `||r|| / (||b|| + ||A||_bound ||x||)`.
    #[must_use]
    pub const fn backward_error(&self) -> f64 {
        self.backward_error
    }

    /// Return the absolute residual threshold used for certification.
    #[must_use]
    pub const fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// Return the number of explicit residual-replacement restarts.
    #[must_use]
    pub const fn restarts(&self) -> usize {
        self.restarts
    }

    /// Return the norm of accepted component-nullspace roundoff removed from
    /// the submitted RHS before iteration.
    #[must_use]
    pub const fn rhs_projection_norm(&self) -> f64 {
        self.rhs_projection_norm
    }

    /// Return solution-free diagnostics for this owned result.
    #[must_use]
    pub const fn diagnostics(&self) -> PcgDiagnostics {
        PcgDiagnostics {
            iterations: self.iterations,
            initial_residual_norm: self.initial_residual_norm,
            residual_norm: self.residual_norm,
            relative_residual: self.relative_residual,
            backward_error: self.backward_error,
            tolerance: self.tolerance,
            restarts: self.restarts,
            rhs_projection_norm: self.rhs_projection_norm,
        }
    }
}

/// Execution context of a certified PCG solve or batch.
///
/// The default solves serially, without cancellation, against the
/// preconditioner's own finest graph. A [`ParallelCmgPlan`] selects row
/// operators for the cycle and for products with the preconditioner's own
/// graph; a team runs planned kernels and, at two or more workers, fixed-chunk
/// reductions and single-component centering (the existing planned
/// arithmetic, which depends on the worker count only through that choice).
/// With `cancel`, every vector kernel, reduction, product and cycle polls the
/// sticky query after a bounded number of processed items and parallel lanes
/// poll it between bounded sub-ranges; [`CmgError::Cancelled`] is returned only
/// after all lanes join, and outputs are then unspecified.
#[derive(Clone, Copy, Default)]
pub struct SolveContext<'a> {
    /// Row operators retained for the preconditioner's large levels.
    #[cfg(feature = "parallel")]
    pub plan: Option<&'a ParallelCmgPlan>,
    /// The borrowed host team.
    #[cfg(feature = "parallel")]
    pub team: Option<Team<'a>>,
    /// For batches: solve right-hand sides concurrently on `team`, each on one
    /// lane of it ([`Team::single_lane`], the team's reduction arithmetic) with
    /// its own workspace. Requires a team and no plan.
    #[cfg(feature = "parallel")]
    pub concurrent_right_hand_sides: bool,
    /// Precondition with an earlier hierarchy whose finest graph shares the
    /// solved graph's prepared topology. All operator products and
    /// certificates use the solved graph; a plan serves only the cycle.
    pub retained_preconditioner: bool,
    /// The caller's sticky cancellation query.
    pub cancel: Option<Cancellation<'a>>,
}

impl core::fmt::Debug for SolveContext<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug = formatter.debug_struct("SolveContext");
        #[cfg(feature = "parallel")]
        debug
            .field("plan", &self.plan.map(ParallelCmgPlan::operator_count))
            .field("team", &self.team)
            .field(
                "concurrent_right_hand_sides",
                &self.concurrent_right_hand_sides,
            );
        debug
            .field("retained_preconditioner", &self.retained_preconditioner)
            .field("cancel", &self.cancel.is_some())
            .finish()
    }
}

/// Solve a compatible graph-Laplacian system with a newly allocated workspace.
pub fn solve_pcg(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    rhs: &[f64],
    options: PcgOptions,
) -> Result<PcgResult, CmgError> {
    let mut workspace = PcgWorkspace::new(preconditioner);
    solve_pcg_with(
        graph,
        preconditioner,
        rhs,
        options,
        &mut workspace,
        SolveContext::default(),
    )
}

/// Solve with a reusable workspace under `context`, returning an owned result.
pub fn solve_pcg_with(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    rhs: &[f64],
    options: PcgOptions,
    workspace: &mut PcgWorkspace,
    context: SolveContext<'_>,
) -> Result<PcgResult, CmgError> {
    let solve = Pcg::new(graph, preconditioner, context);
    // The owned solution is a fallible, counted copy; the final checkpoint
    // follows it, so a cancellation during or after the copy returns nothing.
    with_meter!(context.cancel, meter => {
        solve
            .run(rhs, None, options, workspace, meter, &mut ())
            .and_then(|diagnostics| {
                let solution = meter.copy_slice(&workspace.solution, "PCG owned solution")?;
                Ok(result_from_diagnostics(solution, diagnostics))
            })
    })
}

/// Solve right-hand sides one after another with one reusable workspace.
pub fn solve_pcg_batch(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    right_hand_sides: &[Vec<f64>],
    options: PcgOptions,
) -> Result<Vec<PcgResult>, CmgError> {
    let mut workspace = PcgWorkspace::new(preconditioner);
    right_hand_sides
        .iter()
        .map(|rhs| {
            solve_pcg_with(
                graph,
                preconditioner,
                rhs,
                options,
                &mut workspace,
                SolveContext::default(),
            )
        })
        .collect()
}

/// Solve independent right-hand sides concurrently on `team`, each on one lane
/// of it (the team's reduction arithmetic) with a private reusable workspace,
/// returning owned results
/// in input order. The team's workspace-memory budget limits the number of
/// simultaneous solves.
#[cfg(feature = "parallel")]
pub fn solve_pcg_batch_concurrent(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    right_hand_sides: &[Vec<f64>],
    options: PcgOptions,
    team: Team<'_>,
) -> Result<Vec<PcgResult>, CmgError> {
    if right_hand_sides.is_empty() {
        return Ok(Vec::new());
    }
    if !preconditioner.matches_graph(graph) {
        return Err(CmgError::InvalidHierarchy {
            context: "parallel PCG graph differs from the preconditioner's finest graph",
        });
    }
    for rhs in right_hand_sides {
        if rhs.len() != graph.vertex_count() {
            return Err(CmgError::dimension(
                "solve_pcg_batch_parallel rhs",
                graph.vertex_count(),
                rhs.len(),
            ));
        }
    }
    let first_workspace = PcgWorkspace::new(preconditioner);
    let concurrency = team.batch_concurrency(first_workspace.byte_len(), right_hand_sides.len())?;
    let mut workspaces = Vec::with_capacity(concurrency);
    workspaces.push(first_workspace);
    workspaces.extend((1..concurrency).map(|_| PcgWorkspace::new(preconditioner)));
    solve_owned_concurrently(
        graph,
        preconditioner,
        right_hand_sides,
        options,
        team,
        &mut workspaces,
    )
}

/// The one owned-result across-RHS loop: groups of `workspaces.len()`
/// serial solves run on the team and join before the next group.
#[cfg(feature = "parallel")]
pub(crate) fn solve_owned_concurrently(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    right_hand_sides: &[Vec<f64>],
    options: PcgOptions,
    team: Team<'_>,
    workspaces: &mut [PcgWorkspace],
) -> Result<Vec<PcgResult>, CmgError> {
    let mut results = Vec::with_capacity(right_hand_sides.len());
    for rhs_chunk in right_hand_sides.chunks(workspaces.len()) {
        // At most `worker_ceiling` lanes claim the group's solves.
        let mut slots: Vec<(&mut PcgWorkspace, Option<Result<PcgResult, CmgError>>)> = workspaces
            [..rhs_chunk.len()]
            .iter_mut()
            .map(|workspace| (workspace, None))
            .collect();
        team.chunks_mut(&mut slots, 1, None, |index, slot, _| {
            let (workspace, outcome) = &mut slot[0];
            *outcome = Some(solve_pcg_with(
                graph,
                preconditioner,
                &rhs_chunk[index],
                options,
                workspace,
                SolveContext {
                    team: Some(team.single_lane()),
                    ..SolveContext::default()
                },
            ));
            Ok(())
        })?;
        for (_, outcome) in slots {
            results.push(outcome.ok_or(CmgError::InvalidHierarchy {
                context: "parallel PCG group has no outcome",
            })??);
        }
    }
    Ok(results)
}

/// Solve into caller-owned storage, optionally starting from a supplied guess.
///
/// Component-sum defects accepted by `compatibility_tolerance` are projected
/// onto the exact Laplacian range. Final certification is performed against
/// the submitted, unprojected RHS and the projection norm is reported. The
/// workspace owns all numerical scratch; no solution result is allocated.
#[allow(clippy::too_many_arguments)]
pub fn solve_pcg_into(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    rhs: &[f64],
    initial_guess: Option<&[f64]>,
    solution: &mut [f64],
    options: PcgOptions,
    workspace: &mut PcgWorkspace,
    context: SolveContext<'_>,
) -> Result<PcgDiagnostics, CmgError> {
    if solution.len() != graph.vertex_count() {
        return Err(CmgError::dimension(
            "solve_pcg_into solution",
            graph.vertex_count(),
            solution.len(),
        ));
    }
    let solve = Pcg::new(graph, preconditioner, context);
    with_meter!(context.cancel, meter => {
        let diagnostics = solve.run(rhs, initial_guess, options, workspace, meter, &mut ())?;
        copy(meter, "PCG solution", solution, &workspace.solution)?;
        Ok(diagnostics)
    })
}

/// [`solve_pcg_into`] leaving the solution in the workspace.
pub(crate) fn solve_pcg_in_workspace(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    rhs: &[f64],
    initial_guess: Option<&[f64]>,
    options: PcgOptions,
    workspace: &mut PcgWorkspace,
    context: SolveContext<'_>,
) -> Result<PcgDiagnostics, CmgError> {
    let solve = Pcg::new(graph, preconditioner, context);
    with_meter!(context.cancel, meter => {
        solve.run(rhs, initial_guess, options, workspace, meter, &mut ())
    })
}

/// Solve borrowed contiguous or strided right-hand sides into caller buffers.
///
/// Right-hand sides run one after another in `workspaces[0]`, each with the
/// context's plan and team, or concurrently across RHS on the team (see
/// [`SolveContext::concurrent_right_hand_sides`]), using at most
/// `workspaces.len()` simultaneous solves limited by the team's workspace
/// budget. Results keep input order; the first failing RHS in input order is
/// reported after its concurrent group joins. No CMG-owned allocation occurs
/// during a successfully validated call.
#[allow(clippy::too_many_arguments)]
pub fn solve_pcg_batch_into(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    right_hand_sides: PcgBatchRef<'_>,
    initial_guesses: Option<PcgBatchRef<'_>>,
    mut solutions: PcgBatchMut<'_>,
    diagnostics: &mut [PcgDiagnostics],
    options: PcgOptions,
    workspaces: &mut [PcgBatchWorkspace],
    context: SolveContext<'_>,
) -> Result<(), CmgError> {
    let solve = Pcg::new(graph, preconditioner, context);
    validate_batch_buffers(
        &solve,
        right_hand_sides,
        initial_guesses,
        &solutions,
        diagnostics.len(),
        workspaces,
    )?;
    #[cfg(feature = "parallel")]
    if context.concurrent_right_hand_sides {
        return solve_concurrently(
            &solve,
            right_hand_sides,
            initial_guesses,
            &mut solutions,
            diagnostics,
            options,
            workspaces,
            context,
        );
    }
    with_meter!(context.cancel, meter => {
        solve_sequentially(
            &solve,
            right_hand_sides,
            initial_guesses,
            &mut solutions,
            diagnostics,
            options,
            workspaces.first_mut(),
            meter,
            &mut (),
        )
    })
}

/// Profile validation, gather, certified solve and scatter on the sequential
/// caller-buffer path. The timers are intended for separate trace runs, not
/// benchmark timings.
#[cfg(feature = "profiling")]
#[allow(clippy::too_many_arguments)]
pub fn profile_pcg_batch_into(
    graph: &Laplacian,
    preconditioner: &CmgPreconditioner,
    right_hand_sides: PcgBatchRef<'_>,
    initial_guesses: Option<PcgBatchRef<'_>>,
    mut solutions: PcgBatchMut<'_>,
    diagnostics: &mut [PcgDiagnostics],
    options: PcgOptions,
    workspace: &mut PcgBatchWorkspace,
    context: SolveContext<'_>,
) -> Result<PcgBatchPhaseProfile, CmgError> {
    let total_start = Instant::now();
    let mut profile = PcgBatchPhaseProfile::default();
    let solve = Pcg::new(graph, preconditioner, context);
    let validation_start = Instant::now();
    validate_batch_buffers(
        &solve,
        right_hand_sides,
        initial_guesses,
        &solutions,
        diagnostics.len(),
        core::slice::from_ref(workspace),
    )?;
    profile.validation_nanoseconds = validation_start.elapsed().as_nanos();
    with_meter!(context.cancel, meter => {
        solve_sequentially(
            &solve,
            right_hand_sides,
            initial_guesses,
            &mut solutions,
            diagnostics,
            options,
            Some(workspace),
            meter,
            &mut profile,
        )
    })?;
    profile.total_nanoseconds = total_start.elapsed().as_nanos();
    Ok(profile)
}

#[allow(clippy::too_many_arguments)]
fn solve_sequentially<M: Meter, R: PcgRecorder>(
    solve: &Pcg<'_>,
    right_hand_sides: PcgBatchRef<'_>,
    initial_guesses: Option<PcgBatchRef<'_>>,
    solutions: &mut PcgBatchMut<'_>,
    diagnostics: &mut [PcgDiagnostics],
    options: PcgOptions,
    workspace: Option<&mut PcgBatchWorkspace>,
    meter: &mut M,
    recorder: &mut R,
) -> Result<(), CmgError> {
    let Some(workspace) = workspace else {
        return Ok(());
    };
    for (rhs_index, diagnostic) in diagnostics.iter_mut().enumerate() {
        let PcgBatchWorkspace {
            pcg,
            rhs: rhs_staging,
            guess: guess_staging,
            ..
        } = workspace;
        let stamp = R::start();
        let rhs = right_hand_sides.gather(rhs_index, rhs_staging, meter)?;
        let guess = match initial_guesses {
            Some(guesses) => Some(guesses.gather(rhs_index, guess_staging, meter)?),
            None => None,
        };
        recorder.finish(PcgPhase::Gather, stamp);
        let stamp = R::start();
        *diagnostic = solve.run(rhs, guess, options, pcg, meter, recorder)?;
        recorder.finish(PcgPhase::Solve, stamp);
        let stamp = R::start();
        solutions.scatter(rhs_index, &pcg.solution, meter)?;
        recorder.finish(PcgPhase::Scatter, stamp);
    }
    Ok(())
}

#[cfg(feature = "parallel")]
#[allow(clippy::too_many_arguments)]
fn solve_concurrently(
    solve: &Pcg<'_>,
    right_hand_sides: PcgBatchRef<'_>,
    initial_guesses: Option<PcgBatchRef<'_>>,
    solutions: &mut PcgBatchMut<'_>,
    diagnostics: &mut [PcgDiagnostics],
    options: PcgOptions,
    workspaces: &mut [PcgBatchWorkspace],
    context: SolveContext<'_>,
) -> Result<(), CmgError> {
    let team = context.team.ok_or(CmgError::InvalidHierarchy {
        context: "concurrent PCG right-hand sides require a team",
    })?;
    if context.plan.is_some() {
        return Err(CmgError::InvalidHierarchy {
            context: "concurrent PCG right-hand sides use the serial recurrence without a plan",
        });
    }
    if right_hand_sides.rhs_count == 0 {
        return Ok(());
    }
    // Each concurrent solve runs on one lane with its own workspace; the
    // one-lane team keeps the full team's reduction and centering arithmetic.
    let lane = Pcg {
        team: Some(team.single_lane()),
        ..*solve
    };
    let concurrency = team
        .batch_concurrency(workspaces[0].byte_len(), right_hand_sides.rhs_count)?
        .min(workspaces.len());
    let cancel = context.cancel;
    with_meter!(cancel, meter => (|| {
        for start in (0..right_hand_sides.rhs_count).step_by(concurrency) {
            let count = concurrency.min(right_hand_sides.rhs_count - start);
            // At most `worker_ceiling` lanes claim the group's solves; each
            // solve meters itself and all lanes join before any publication.
            team.chunks_mut(&mut workspaces[..count], 1, None, |local_index, slot, _| {
                let rhs_index = start + local_index;
                let PcgBatchWorkspace {
                    pcg,
                    rhs: rhs_staging,
                    guess: guess_staging,
                    outcome,
                } = &mut slot[0];
                *outcome = Some(with_meter!(cancel, meter => (|| {
                    let rhs = right_hand_sides.gather(rhs_index, rhs_staging, meter)?;
                    let guess = match initial_guesses {
                        Some(guesses) => Some(guesses.gather(rhs_index, guess_staging, meter)?),
                        None => None,
                    };
                    lane.run(rhs, guess, options, pcg, meter, &mut ())
                })()));
                Ok(())
            })?;
            // A lane that stopped on the sticky query makes cancellation the
            // group's outcome; otherwise the first failing RHS in input order.
            meter.joined("concurrent PCG join")?;
            let mut first_error = None;
            for (local_index, workspace) in workspaces[..count].iter_mut().enumerate() {
                match workspace.outcome.take() {
                    Some(Ok(result)) => diagnostics[start + local_index] = result,
                    Some(Err(error)) => {
                        first_error.get_or_insert(error);
                    }
                    None => {
                        first_error.get_or_insert(CmgError::InvalidHierarchy {
                            context: "parallel PCG batch workspace has no outcome",
                        });
                    }
                }
            }
            if let Some(error) = first_error {
                return Err(error);
            }
            // Publication is bounded and polled; the final poll precedes success.
            for (local_index, workspace) in workspaces[..count].iter().enumerate() {
                solutions.scatter(start + local_index, &workspace.pcg.solution, meter)?;
            }
        }
        Ok(())
    })())
}

fn validate_batch_buffers(
    solve: &Pcg<'_>,
    right_hand_sides: PcgBatchRef<'_>,
    initial_guesses: Option<PcgBatchRef<'_>>,
    solutions: &PcgBatchMut<'_>,
    diagnostic_count: usize,
    workspaces: &[PcgBatchWorkspace],
) -> Result<(), CmgError> {
    if !solve.compatible() {
        return Err(CmgError::InvalidHierarchy {
            context: "PCG batch graph and preconditioner are incompatible",
        });
    }
    let dimension = solve.graph.vertex_count();
    for (context, actual) in [
        ("PCG batch RHS dimension", right_hand_sides.dimension),
        ("PCG batch solution dimension", solutions.dimension),
    ] {
        if actual != dimension {
            return Err(CmgError::dimension(context, dimension, actual));
        }
    }
    if solutions.rhs_count != right_hand_sides.rhs_count {
        return Err(CmgError::dimension(
            "PCG batch solution count",
            right_hand_sides.rhs_count,
            solutions.rhs_count,
        ));
    }
    if diagnostic_count != right_hand_sides.rhs_count {
        return Err(CmgError::dimension(
            "PCG batch diagnostic count",
            right_hand_sides.rhs_count,
            diagnostic_count,
        ));
    }
    if let Some(guesses) = initial_guesses {
        if guesses.dimension != dimension {
            return Err(CmgError::dimension(
                "PCG batch guess dimension",
                dimension,
                guesses.dimension,
            ));
        }
        if guesses.rhs_count != right_hand_sides.rhs_count {
            return Err(CmgError::dimension(
                "PCG batch guess count",
                right_hand_sides.rhs_count,
                guesses.rhs_count,
            ));
        }
    }
    if right_hand_sides.rhs_count > 0 && workspaces.is_empty() {
        return Err(CmgError::dimension("PCG batch workspace pool", 1, 0));
    }
    for workspace in workspaces {
        workspace.validate(solve.cmg)?;
    }
    solve.validate_plan()
}

/// One phase of a profiled solve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PcgPhase {
    Setup,
    Preconditioner,
    Matvec,
    DotProducts,
    VectorUpdates,
    Centering,
    Norms,
    ResidualRecompute,
    Certification,
    Gather,
    Solve,
    Scatter,
}

/// Observer of the one PCG recurrence; `()` records nothing and compiles away.
pub(crate) trait PcgRecorder {
    type Stamp;
    type Cycle: CycleRecorder;
    fn start() -> Self::Stamp;
    fn finish(&mut self, phase: PcgPhase, start: Self::Stamp);
    fn cycle(&mut self) -> &mut Self::Cycle;
}

impl PcgRecorder for () {
    type Stamp = ();
    type Cycle = ();
    #[inline(always)]
    fn start() {}
    #[inline(always)]
    fn finish(&mut self, _: PcgPhase, (): ()) {}
    #[inline(always)]
    fn cycle(&mut self) -> &mut () {
        self
    }
}

#[cfg(feature = "profiling")]
impl PcgRecorder for PcgBatchPhaseProfile {
    type Stamp = Instant;
    type Cycle = ();
    fn start() -> Instant {
        Instant::now()
    }
    fn finish(&mut self, phase: PcgPhase, start: Instant) {
        let slot = match phase {
            PcgPhase::Gather => &mut self.gather_nanoseconds,
            PcgPhase::Solve => &mut self.solve_nanoseconds,
            PcgPhase::Scatter => &mut self.scatter_nanoseconds,
            _ => return,
        };
        *slot += start.elapsed().as_nanos();
    }
    fn cycle(&mut self) -> &mut () {
        // Batch profiles do not attribute inside the cycle.
        &mut self.cycle
    }
}

/// Run `$operation` and record it as `$phase`.
macro_rules! timed {
    ($recorder:ident: $R:ident, $phase:ident, $operation:expr) => {{
        let stamp = $R::start();
        let value = $operation;
        $recorder.finish(PcgPhase::$phase, stamp);
        value
    }};
}

/// The one certified PCG recurrence and the solve's execution resources.
#[derive(Clone, Copy)]
pub(crate) struct Pcg<'a> {
    graph: &'a Laplacian,
    cmg: &'a CmgPreconditioner,
    retained: bool,
    #[cfg(feature = "parallel")]
    plan: Option<&'a ParallelCmgPlan>,
    #[cfg(feature = "parallel")]
    team: Option<Team<'a>>,
}

impl<'a> Pcg<'a> {
    pub(crate) fn new(
        graph: &'a Laplacian,
        cmg: &'a CmgPreconditioner,
        context: SolveContext<'a>,
    ) -> Self {
        Self {
            graph,
            cmg,
            retained: context.retained_preconditioner,
            #[cfg(feature = "parallel")]
            plan: context.plan,
            #[cfg(feature = "parallel")]
            team: context.team,
        }
    }

    fn compatible(&self) -> bool {
        if self.retained {
            self.cmg.matches_prepared_topology(self.graph)
        } else {
            self.cmg.matches_graph(self.graph)
        }
    }

    fn validate_plan(&self) -> Result<(), CmgError> {
        #[cfg(feature = "parallel")]
        if let Some(plan) = self.plan {
            plan.validate(self.cmg)?;
        }
        Ok(())
    }

    fn planned(&self) -> bool {
        #[cfg(feature = "parallel")]
        {
            self.plan.is_some()
        }
        #[cfg(not(feature = "parallel"))]
        {
            false
        }
    }

    /// Teams of two or more workers select fixed-chunk reductions.
    #[cfg(feature = "parallel")]
    fn reduction_team(&self) -> Option<Team<'a>> {
        // Any team, one lane included: the fixed-chunk tree depends only on
        // lengths and the team's thresholds, never on its lane count.
        self.team
    }

    #[cfg(not(feature = "parallel"))]
    #[cfg_attr(
        not(feature = "experimental-components"),
        allow(clippy::unused_self, dead_code)
    )]
    fn reduction_team(&self) -> Option<Team<'a>> {
        None
    }

    fn team(&self) -> Option<Team<'a>> {
        #[cfg(feature = "parallel")]
        {
            self.team
        }
        #[cfg(not(feature = "parallel"))]
        {
            None
        }
    }

    pub(crate) fn run<M: Meter, R: PcgRecorder>(
        &self,
        rhs: &[f64],
        initial_guess: Option<&[f64]>,
        options: PcgOptions,
        workspace: &mut PcgWorkspace,
        meter: &mut M,
        recorder: &mut R,
    ) -> Result<PcgDiagnostics, CmgError> {
        // Choose one monomorphized loop at entry; connected solves keep the
        // original centering and dot-product call sequence throughout.
        #[cfg(feature = "experimental-components")]
        if self.cmg.finest_components().count() > 1 && self.reduction_team().is_none() {
            return self.kernel::<true, M, R>(
                rhs,
                initial_guess,
                options,
                workspace,
                meter,
                recorder,
            );
        }
        self.kernel::<false, M, R>(rhs, initial_guess, options, workspace, meter, recorder)
    }

    #[allow(clippy::too_many_lines)]
    fn kernel<const FUSED_CENTERING: bool, M: Meter, R: PcgRecorder>(
        &self,
        rhs: &[f64],
        initial_guess: Option<&[f64]>,
        options: PcgOptions,
        workspace: &mut PcgWorkspace,
        meter: &mut M,
        recorder: &mut R,
    ) -> Result<PcgDiagnostics, CmgError> {
        let options = options.validate()?;
        let graph = self.graph;
        let dimension = graph.vertex_count();
        if !self.compatible() {
            return Err(CmgError::InvalidHierarchy {
                context: match (self.retained, self.planned()) {
                    (false, _) => "PCG graph differs from the preconditioner's finest graph",
                    (true, false) => {
                        "retained PCG preconditioner has a different prepared topology"
                    }
                    (true, true) => {
                        "planned retained PCG preconditioner has a different prepared topology"
                    }
                },
            });
        }
        if rhs.len() != dimension {
            return Err(CmgError::dimension("solve_pcg rhs", dimension, rhs.len()));
        }
        workspace.validate(self.cmg)?;
        self.validate_plan()?;
        if let Some(guess) = initial_guess {
            if guess.len() != dimension {
                return Err(CmgError::dimension(
                    if self.planned() {
                        "planned PCG initial guess"
                    } else {
                        "solve_pcg initial guess"
                    },
                    dimension,
                    guess.len(),
                ));
            }
        }

        let components = self.cmg.finest_components();
        let stamp = R::start();
        copy(
            meter,
            "PCG projected RHS",
            &mut workspace.projected_rhs,
            rhs,
        )?;
        let rhs_projection_norm = components.project_rhs_in_place_with_workspace(
            &mut workspace.projected_rhs,
            options.validation,
            &mut workspace.component,
            meter,
        )?;
        if let Some(guess) = initial_guess {
            copy(meter, "PCG initial guess", &mut workspace.solution, guess)?;
            components.center_with_team(
                &mut workspace.solution,
                &mut workspace.component,
                self.team(),
                meter,
            )?;
            self.recompute_residual(
                &workspace.projected_rhs,
                &workspace.solution,
                &mut workspace.residual,
                meter,
            )?;
        } else {
            crate::cancel::zero(meter, "PCG solution", &mut workspace.solution)?;
            copy(
                meter,
                "PCG residual",
                &mut workspace.residual,
                &workspace.projected_rhs,
            )?;
        }
        recorder.finish(PcgPhase::Setup, stamp);

        let rhs_norm = timed!(recorder: R, Norms, self.norm(rhs, meter)?);
        let initial_residual_norm = if initial_guess.is_some() {
            original_residual_norm(rhs, &workspace.projected_rhs, &workspace.residual, meter)?
        } else {
            rhs_norm
        };
        let projected_initial_norm = timed!(
            recorder: R,
            Norms,
            if initial_guess.is_some() {
                self.norm(&workspace.residual, meter)?
            } else {
                self.norm(&workspace.projected_rhs, meter)?
            }
        );
        let operator_bound = graph.operator_norm_bound();
        let initial_solution_norm = if initial_guess.is_some() {
            timed!(recorder: R, Norms, self.norm(&workspace.solution, meter)?)
        } else {
            0.0
        };
        let initial_tolerance =
            allowed_residual(options, rhs_norm, operator_bound, initial_solution_norm);
        if initial_residual_norm <= initial_tolerance {
            return timed!(
                recorder: R,
                Certification,
                diagnostics(
                    &workspace.solution,
                    0,
                    initial_residual_norm,
                    initial_residual_norm,
                    rhs_norm,
                    initial_tolerance,
                    operator_bound,
                    0,
                    rhs_projection_norm,
                    meter,
                )
            );
        }
        if projected_initial_norm == 0.0 {
            return Err(CmgError::ResidualVerificationFailed {
                iteration: 0,
                residual_norm: initial_residual_norm,
                tolerance: initial_tolerance,
            });
        }

        timed!(
            recorder: R,
            Preconditioner,
            self.apply(
                &workspace.residual,
                &mut workspace.preconditioned,
                &mut workspace.cmg,
                meter,
                recorder.cycle(),
            )?
        );
        let mut rho = self.rho::<FUSED_CENTERING, M, R>(workspace, meter, recorder)?;
        validate_positive_pcg(0, "r^T M r", rho)?;
        timed!(
            recorder: R,
            VectorUpdates,
            copy(
                meter,
                "PCG direction",
                &mut workspace.direction,
                &workspace.preconditioned,
            )?
        );

        let mut restarts = 0_usize;
        let mut last_tolerance = initial_tolerance;

        for iteration in 1..=options.max_iterations {
            timed!(
                recorder: R,
                Matvec,
                self.matvec(
                    &workspace.direction,
                    &mut workspace.matrix_direction,
                    meter
                )?
            );
            let direction_curvature = timed!(
                recorder: R,
                DotProducts,
                self.dot(&workspace.direction, &workspace.matrix_direction, meter)?
            );
            validate_positive_pcg(iteration, "p^T A p", direction_curvature)?;
            let alpha = rho / direction_curvature;
            validate_finite_pcg(iteration, "alpha", alpha)?;

            timed!(recorder: R, VectorUpdates, {
                let PcgWorkspace {
                    solution,
                    residual,
                    direction,
                    matrix_direction,
                    ..
                } = &mut *workspace;
                chunked(meter, "PCG update", dimension, 2, |range| {
                    for (((x, &p), r), &ap) in solution[range.clone()]
                        .iter_mut()
                        .zip(&direction[range.clone()])
                        .zip(&mut residual[range.clone()])
                        .zip(&matrix_direction[range])
                    {
                        *x += alpha * p;
                        *r -= alpha * ap;
                    }
                    Ok(())
                })?
            });
            timed!(
                recorder: R,
                Centering,
                components.center_with_team(
                    &mut workspace.solution,
                    &mut workspace.component,
                    self.team(),
                    meter,
                )?
            );

            let (solution_norm, recursive_residual_norm) = timed!(
                recorder: R,
                Norms,
                self.paired_norms(&workspace.solution, &workspace.residual, meter)?
            );
            last_tolerance = allowed_residual(options, rhs_norm, operator_bound, solution_norm);
            let candidate = recursive_residual_norm <= last_tolerance;
            let scheduled_recompute = iteration % options.residual_recompute_interval == 0;
            let mut restarted = false;

            if candidate || scheduled_recompute {
                let projected_fresh_norm = timed!(
                    recorder: R,
                    ResidualRecompute,
                    self.recompute_residual(
                        &workspace.projected_rhs,
                        &workspace.solution,
                        &mut workspace.matrix_direction,
                        meter,
                    )?
                );
                timed!(
                    recorder: R,
                    VectorUpdates,
                    copy(
                        meter,
                        "PCG residual",
                        &mut workspace.residual,
                        &workspace.matrix_direction,
                    )?
                );
                restarted = true;
                restarts += 1;
                if projected_fresh_norm <= last_tolerance {
                    let original_norm = timed!(
                        recorder: R,
                        Certification,
                        original_residual_norm(
                            rhs,
                            &workspace.projected_rhs,
                            &workspace.matrix_direction,
                            meter,
                        )?
                    );
                    if original_norm <= last_tolerance {
                        return timed!(
                            recorder: R,
                            Certification,
                            diagnostics(
                                &workspace.solution,
                                iteration,
                                initial_residual_norm,
                                original_norm,
                                rhs_norm,
                                last_tolerance,
                                operator_bound,
                                restarts,
                                rhs_projection_norm,
                                meter,
                            )
                        );
                    }
                    if iteration == options.max_iterations {
                        return Err(CmgError::ResidualVerificationFailed {
                            iteration,
                            residual_norm: original_norm,
                            tolerance: last_tolerance,
                        });
                    }
                }
            }

            if iteration == options.max_iterations {
                break;
            }

            // The submitted RHS was projected once. Remove only the
            // component-nullspace roundoff accumulated by Krylov updates
            // before reusing the compatible stationary core.
            timed!(
                recorder: R,
                Centering,
                components.center_with_team(
                    &mut workspace.residual,
                    &mut workspace.component,
                    self.team(),
                    meter,
                )?
            );
            timed!(
                recorder: R,
                Preconditioner,
                self.apply(
                    &workspace.residual,
                    &mut workspace.preconditioned,
                    &mut workspace.cmg,
                    meter,
                    recorder.cycle(),
                )?
            );
            let new_rho = self.rho::<FUSED_CENTERING, M, R>(workspace, meter, recorder)?;
            validate_positive_pcg(iteration, "new r^T M r", new_rho)?;

            if restarted {
                timed!(
                    recorder: R,
                    VectorUpdates,
                    copy(
                        meter,
                        "PCG direction",
                        &mut workspace.direction,
                        &workspace.preconditioned,
                    )?
                );
            } else {
                let beta = new_rho / rho;
                validate_finite_pcg(iteration, "beta", beta)?;
                timed!(recorder: R, VectorUpdates, {
                    let PcgWorkspace {
                        direction,
                        preconditioned,
                        ..
                    } = &mut *workspace;
                    chunked(meter, "PCG direction", dimension, 1, |range| {
                        for (direction, &preconditioned) in direction[range.clone()]
                            .iter_mut()
                            .zip(&preconditioned[range])
                        {
                            *direction = preconditioned + beta * *direction;
                        }
                        Ok(())
                    })?
                });
            }
            rho = new_rho;
        }

        timed!(
            recorder: R,
            ResidualRecompute,
            self.recompute_residual(
                &workspace.projected_rhs,
                &workspace.solution,
                &mut workspace.matrix_direction,
                meter,
            )?
        );
        let residual_norm = timed!(
            recorder: R,
            Certification,
            original_residual_norm(
                rhs,
                &workspace.projected_rhs,
                &workspace.matrix_direction,
                meter,
            )?
        );
        Err(CmgError::MaximumIterations {
            iterations: options.max_iterations,
            residual_norm,
            tolerance: last_tolerance,
        })
    }

    /// `M r`, centered, and its inner product with `r`.
    fn rho<const FUSED_CENTERING: bool, M: Meter, R: PcgRecorder>(
        &self,
        workspace: &mut PcgWorkspace,
        meter: &mut M,
        recorder: &mut R,
    ) -> Result<f64, CmgError> {
        let components = self.cmg.finest_components();
        #[cfg(feature = "experimental-components")]
        if FUSED_CENTERING {
            return timed!(
                recorder: R,
                Centering,
                components.center_and_dot_with_workspace(
                    &mut workspace.preconditioned,
                    &workspace.residual,
                    &mut workspace.component,
                    meter,
                )
            );
        }
        timed!(
            recorder: R,
            Centering,
            components.center_with_team(
                &mut workspace.preconditioned,
                &mut workspace.component,
                self.team(),
                meter,
            )?
        );
        Ok(timed!(
            recorder: R,
            DotProducts,
            self.dot(&workspace.residual, &workspace.preconditioned, meter)?
        ))
    }

    fn apply<M: Meter, C: CycleRecorder>(
        &self,
        residual: &[f64],
        preconditioned: &mut [f64],
        workspace: &mut CmgWorkspace,
        meter: &mut M,
        recorder: &mut C,
    ) -> Result<(), CmgError> {
        self.cmg.apply_compatible_metered(
            residual,
            preconditioned,
            workspace,
            ApplyContext {
                #[cfg(feature = "parallel")]
                plan: self.plan,
                #[cfg(feature = "parallel")]
                team: self.team,
                cancel: None,
            },
            meter,
            recorder,
        )
    }

    /// The solved graph's product; the preconditioner's own graph uses the
    /// plan's finest row operator when it retained one.
    fn matvec<M: Meter>(
        &self,
        input: &[f64],
        output: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let graph = self.graph;
        for (context, actual) in [
            ("Laplacian::matvec input", input.len()),
            ("Laplacian::matvec output", output.len()),
        ] {
            if actual != graph.vertex_count() {
                return Err(CmgError::dimension(context, graph.vertex_count(), actual));
            }
        }
        #[cfg(feature = "parallel")]
        if let Some(plan) = self.plan.filter(|_| !self.retained) {
            return plan.finest_matvec(graph, input, output, self.team, meter);
        }
        graph.matvec_lanes(input, output, One, meter)
    }

    fn recompute_residual<M: Meter>(
        &self,
        rhs: &[f64],
        solution: &[f64],
        residual: &mut [f64],
        meter: &mut M,
    ) -> Result<f64, CmgError> {
        self.matvec(solution, residual, meter)?;
        chunked(meter, "PCG residual", residual.len(), 1, |range| {
            for (residual, &rhs) in residual[range.clone()].iter_mut().zip(&rhs[range]) {
                *residual = rhs - *residual;
            }
            Ok(())
        })?;
        self.norm(residual, meter)
    }

    fn dot<M: Meter>(&self, left: &[f64], right: &[f64], meter: &mut M) -> Result<f64, CmgError> {
        debug_assert_eq!(left.len(), right.len());
        #[cfg(feature = "parallel")]
        if let Some(team) = self
            .reduction_team()
            .filter(|team| left.len() >= team.options().reduction_floor())
        {
            let query = meter.query();
            let chunk_size = team.options().reduction_chunk_size;
            let value = fixed_chunk_dot(left, right, chunk_size, team, query);
            meter.joined("PCG dot product")?;
            return value;
        }
        let mut sum = Compensated::default();
        chunked(meter, "PCG dot product", left.len(), 1, |range| {
            for index in range {
                sum.add(left[index] * right[index]);
            }
            Ok(())
        })?;
        Ok(sum.value())
    }

    /// Scaled Euclidean norm: a parallel maximum and fixed-chunk sum of
    /// scaled squares above the team's thresholds, else serial.
    fn norm<M: Meter>(&self, values: &[f64], meter: &mut M) -> Result<f64, CmgError> {
        #[cfg(feature = "parallel")]
        if let Some(team) = self.reduction_team() {
            let options = team.options();
            let parallel_floor = options
                .reduction_chunk_size
                .saturating_mul(options.threads)
                .saturating_mul(2);
            let scale = if team.should_parallel(values.len()) && values.len() >= parallel_floor {
                let chunk_size = options.reduction_chunk_size;
                // A maximum is exact in any order: lanes reduce fixed chunks.
                let scale = fixed_chunk_scale(values, chunk_size, team, meter.query());
                meter.joined("PCG norm scale")?;
                scale?
            } else {
                serial_scale(values, meter)?
            };
            if scale == 0.0 {
                return Ok(0.0);
            }
            let sum = if values.len() >= options.reduction_floor() {
                let sum = fixed_chunk_scaled_square_sum(
                    values,
                    scale,
                    options.reduction_chunk_size,
                    team,
                    meter.query(),
                );
                meter.joined("PCG norm sum")?;
                sum?
            } else {
                serial_scaled_square_sum(values, scale, meter)?
            };
            return Ok(scale * sum.sqrt());
        }
        serial_norm(values, meter)
    }

    /// Solution and residual norms; the serial recurrence interleaves both
    /// arithmetic chains in one traversal.
    fn paired_norms<M: Meter>(
        &self,
        left: &[f64],
        right: &[f64],
        meter: &mut M,
    ) -> Result<(f64, f64), CmgError> {
        #[cfg(feature = "experimental-components")]
        if self.reduction_team().is_none() {
            return paired_serial_norms(left, right, meter);
        }
        Ok((self.norm(left, meter)?, self.norm(right, meter)?))
    }
}

/// A Neumaier-compensated running sum.
#[derive(Default)]
struct Compensated {
    sum: f64,
    correction: f64,
}

impl Compensated {
    #[inline(always)]
    fn add(&mut self, value: f64) {
        compensated_add(&mut self.sum, &mut self.correction, value);
    }

    #[inline(always)]
    fn value(&self) -> f64 {
        self.sum + self.correction
    }
}

fn copy<M: Meter>(
    meter: &mut M,
    phase: &'static str,
    target: &mut [f64],
    source: &[f64],
) -> Result<(), CmgError> {
    chunked(meter, phase, target.len(), 1, |range| {
        target[range.clone()].copy_from_slice(&source[range]);
        Ok(())
    })
}

fn serial_scale<M: Meter>(values: &[f64], meter: &mut M) -> Result<f64, CmgError> {
    let mut scale = 0.0_f64;
    chunked(meter, "PCG norm scale", values.len(), 1, |range| {
        for &value in &values[range] {
            scale = scale.max(value.abs());
        }
        Ok(())
    })?;
    Ok(scale)
}

fn serial_scaled_square_sum<M: Meter>(
    values: &[f64],
    scale: f64,
    meter: &mut M,
) -> Result<f64, CmgError> {
    let mut sum = Compensated::default();
    chunked(meter, "PCG norm sum", values.len(), 1, |range| {
        for &value in &values[range] {
            let scaled = value / scale;
            sum.add(scaled * scaled);
        }
        Ok(())
    })?;
    Ok(sum.value())
}

/// The serial scaled Euclidean norm.
pub(crate) fn serial_norm<M: Meter>(values: &[f64], meter: &mut M) -> Result<f64, CmgError> {
    let scale = serial_scale(values, meter)?;
    if scale == 0.0 {
        Ok(0.0)
    } else {
        Ok(scale * serial_scaled_square_sum(values, scale, meter)?.sqrt())
    }
}

/// Compute two independent scaled norms with interleaved arithmetic chains.
/// Each maximum and compensated sum retains its original element order.
#[cfg(feature = "experimental-components")]
fn paired_serial_norms<M: Meter>(
    left: &[f64],
    right: &[f64],
    meter: &mut M,
) -> Result<(f64, f64), CmgError> {
    debug_assert_eq!(left.len(), right.len());
    let mut left_scale = 0.0_f64;
    let mut right_scale = 0.0_f64;
    chunked(meter, "PCG paired norm scale", left.len(), 2, |range| {
        for index in range {
            left_scale = left_scale.max(left[index].abs());
            right_scale = right_scale.max(right[index].abs());
        }
        Ok(())
    })?;
    if left_scale == 0.0 || right_scale == 0.0 {
        let norm = |values: &[f64], scale: f64, meter: &mut M| {
            if scale == 0.0 {
                Ok(0.0)
            } else {
                Ok(scale * serial_scaled_square_sum(values, scale, meter)?.sqrt())
            }
        };
        return Ok((
            norm(left, left_scale, meter)?,
            norm(right, right_scale, meter)?,
        ));
    }
    let mut left_sum = Compensated::default();
    let mut right_sum = Compensated::default();
    chunked(meter, "PCG paired norm sum", left.len(), 2, |range| {
        for index in range {
            let left = left[index] / left_scale;
            let right = right[index] / right_scale;
            left_sum.add(left * left);
            right_sum.add(right * right);
        }
        Ok(())
    })?;
    Ok((
        left_scale * left_sum.value().sqrt(),
        right_scale * right_sum.value().sqrt(),
    ))
}

fn original_residual_norm<M: Meter>(
    original_rhs: &[f64],
    projected_rhs: &[f64],
    projected_residual: &[f64],
    meter: &mut M,
) -> Result<f64, CmgError> {
    let value =
        |index: usize| projected_residual[index] + (original_rhs[index] - projected_rhs[index]);
    let mut scale = 0.0_f64;
    chunked(meter, "PCG certificate", original_rhs.len(), 1, |range| {
        for index in range {
            scale = scale.max(value(index).abs());
        }
        Ok(())
    })?;
    if scale == 0.0 {
        return Ok(0.0);
    }
    let mut sum = Compensated::default();
    chunked(meter, "PCG certificate", original_rhs.len(), 1, |range| {
        for index in range {
            let scaled = value(index) / scale;
            sum.add(scaled * scaled);
        }
        Ok(())
    })?;
    Ok(scale * sum.value().sqrt())
}

/// One leaf of a fixed-chunk compensated reduction: the leaf's terms in index
/// order, counted in segments through the lane's poll. The running sum and
/// correction cross segments unchanged, so the leaf equals `compensated_sum`
/// over the whole chunk. Cancellation propagates and stops the lane.
#[cfg(feature = "parallel")]
fn compensated_leaf(
    poll: &mut crate::cancel::LanePoll<'_>,
    phase: &'static str,
    range: core::ops::Range<usize>,
    term: impl Fn(usize) -> f64,
) -> Result<f64, CmgError> {
    let mut sum = 0.0;
    let mut correction = 0.0;
    let start = range.start;
    chunked(poll, phase, range.len(), 1, |segment| {
        for index in start + segment.start..start + segment.end {
            compensated_add(&mut sum, &mut correction, term(index));
        }
        Ok(())
    })?;
    Ok(sum + correction)
}

/// The largest absolute value, reduced by lanes over fixed chunks; a maximum
/// is exact in any order.
#[cfg(feature = "parallel")]
fn fixed_chunk_scale(
    values: &[f64],
    chunk_size: usize,
    team: Team<'_>,
    query: Option<Cancellation<'_>>,
) -> Result<f64, CmgError> {
    team.reduce(
        values.len().div_ceil(chunk_size),
        query,
        |chunk, poll| {
            let start = chunk * chunk_size;
            let end = values.len().min(start + chunk_size);
            let mut scale = 0.0_f64;
            chunked(poll, "PCG norm scale", end - start, 1, |range| {
                for value in &values[start + range.start..start + range.end] {
                    #[cfg(test)]
                    crate::execution::visits::visit(values.as_ptr());
                    scale = scale.max(value.abs());
                }
                Ok(())
            })?;
            Ok(scale)
        },
        f64::max,
    )
}

#[cfg(feature = "parallel")]
fn fixed_chunk_dot(
    left: &[f64],
    right: &[f64],
    chunk_size: usize,
    team: Team<'_>,
    query: Option<Cancellation<'_>>,
) -> Result<f64, CmgError> {
    debug_assert_eq!(left.len(), right.len());
    let chunk_count = left.len().div_ceil(chunk_size);
    if chunk_count == 0 {
        return Ok(0.0);
    }
    team.reduce(
        chunk_count,
        query,
        |chunk, poll| {
            let start = chunk * chunk_size;
            let end = left.len().min(start + chunk_size);
            compensated_leaf(poll, "PCG dot product", start..end, |index| {
                #[cfg(test)]
                crate::execution::visits::visit(left.as_ptr());
                left[index] * right[index]
            })
        },
        |left_sum, right_sum| compensated_sum([left_sum, right_sum]),
    )
}

#[cfg(feature = "parallel")]
fn fixed_chunk_scaled_square_sum(
    values: &[f64],
    scale: f64,
    chunk_size: usize,
    team: Team<'_>,
    query: Option<Cancellation<'_>>,
) -> Result<f64, CmgError> {
    let chunk_count = values.len().div_ceil(chunk_size);
    if chunk_count == 0 {
        return Ok(0.0);
    }
    team.reduce(
        chunk_count,
        query,
        |chunk, poll| {
            let start = chunk * chunk_size;
            let end = values.len().min(start + chunk_size);
            compensated_leaf(poll, "PCG norm sum", start..end, |index| {
                #[cfg(test)]
                crate::execution::visits::visit(values.as_ptr());
                let scaled = values[index] / scale;
                scaled * scaled
            })
        },
        |left_sum, right_sum| compensated_sum([left_sum, right_sum]),
    )
}

#[allow(clippy::too_many_arguments)]
fn diagnostics<M: Meter>(
    solution: &[f64],
    iterations: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    rhs_norm: f64,
    tolerance: f64,
    operator_bound: f64,
    restarts: usize,
    rhs_projection_norm: f64,
    meter: &mut M,
) -> Result<PcgDiagnostics, CmgError> {
    let solution_norm = serial_norm(solution, meter)?;
    Ok(diagnostics_with_norm(
        solution_norm,
        iterations,
        initial_residual_norm,
        residual_norm,
        rhs_norm,
        tolerance,
        operator_bound,
        restarts,
        rhs_projection_norm,
    ))
}

#[allow(clippy::too_many_arguments)]
fn diagnostics_with_norm(
    solution_norm: f64,
    iterations: usize,
    initial_residual_norm: f64,
    residual_norm: f64,
    rhs_norm: f64,
    tolerance: f64,
    operator_bound: f64,
    restarts: usize,
    rhs_projection_norm: f64,
) -> PcgDiagnostics {
    let denominator = rhs_norm + operator_bound * solution_norm;
    let relative_residual = if rhs_norm > 0.0 {
        residual_norm / rhs_norm
    } else {
        residual_norm
    };
    let backward_error = if denominator > 0.0 {
        residual_norm / denominator
    } else {
        0.0
    };
    PcgDiagnostics {
        iterations,
        initial_residual_norm,
        residual_norm,
        relative_residual,
        backward_error,
        tolerance,
        restarts,
        rhs_projection_norm,
    }
}

pub(crate) fn result_from_diagnostics(
    solution: Vec<f64>,
    diagnostics: PcgDiagnostics,
) -> PcgResult {
    PcgResult {
        solution,
        iterations: diagnostics.iterations,
        initial_residual_norm: diagnostics.initial_residual_norm,
        residual_norm: diagnostics.residual_norm,
        relative_residual: diagnostics.relative_residual,
        backward_error: diagnostics.backward_error,
        tolerance: diagnostics.tolerance,
        restarts: diagnostics.restarts,
        rhs_projection_norm: diagnostics.rhs_projection_norm,
    }
}

pub(crate) fn allowed_residual(
    options: PcgOptions,
    rhs_norm: f64,
    operator_bound: f64,
    solution_norm: f64,
) -> f64 {
    options.absolute_tolerance
        + options.relative_tolerance * (rhs_norm + operator_bound * solution_norm)
}

fn validate_positive_pcg(
    iteration: usize,
    quantity: &'static str,
    value: f64,
) -> Result<(), CmgError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(CmgError::PcgBreakdown {
            iteration,
            quantity,
            value,
        })
    }
}

fn validate_finite_pcg(
    iteration: usize,
    quantity: &'static str,
    value: f64,
) -> Result<(), CmgError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(CmgError::PcgBreakdown {
            iteration,
            quantity,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CmgOptions;
    #[cfg(feature = "experimental-components")]
    use crate::Components;
    #[cfg(any(feature = "parallel", feature = "experimental-components"))]
    use crate::cancel::Unmetered;

    fn fixture() -> (Laplacian, CmgPreconditioner, Vec<f64>) {
        let graph =
            Laplacian::from_edges(128, (0..127).map(|vertex| (vertex, vertex + 1, 1.0))).unwrap();
        let preconditioner = CmgPreconditioner::build(
            &graph,
            CmgOptions {
                direct_threshold: 2,
                ..CmgOptions::default()
            },
        )
        .unwrap();
        let known: Vec<f64> = (0..128).map(|index| (index as f64 / 7.0).cos()).collect();
        let rhs = graph.matvec(&known).unwrap();
        (graph, preconditioner, rhs)
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn required_bytes_matches_allocated_workspace() {
        let (_, preconditioner, _) = fixture();
        assert_eq!(
            PcgWorkspace::required_bytes(&preconditioner),
            PcgWorkspace::new(&preconditioner).byte_len()
        );
    }

    #[test]
    fn solve_overwrites_stale_hot_vectors() {
        let (graph, preconditioner, rhs) = fixture();
        let mut workspace = PcgWorkspace::new(&preconditioner);
        let solve = |workspace: &mut PcgWorkspace| {
            let mut solution = vec![0.0; graph.vertex_count()];
            let diagnostics = solve_pcg_into(
                &graph,
                &preconditioner,
                &rhs,
                None,
                &mut solution,
                PcgOptions::default(),
                workspace,
                SolveContext::default(),
            )
            .unwrap();
            (diagnostics, solution)
        };
        let expected = solve(&mut workspace);
        workspace.preconditioned.fill(f64::NAN);
        workspace.direction.fill(f64::NAN);
        workspace.matrix_direction.fill(f64::NAN);
        assert_eq!(expected, solve(&mut workspace));
    }

    #[cfg(feature = "experimental-components")]
    #[test]
    fn fused_and_original_scalar_loops_match_with_warm_starts_and_restarts() {
        for disconnected in [false, true] {
            let path_len = if disconnected { 96 } else { 130 };
            let mut edges: Vec<_> = (0..path_len - 1)
                .map(|v| (v, v + 1, 0.5 + (v % 7) as f64 / 4.0))
                .collect();
            if disconnected {
                edges.extend((96..128).step_by(2).map(|v| (v, v + 1, 1.0)));
            }
            let graph = Laplacian::from_edges(130, edges).unwrap();
            let preconditioner = CmgPreconditioner::build(
                &graph,
                CmgOptions {
                    direct_threshold: 16,
                    ..CmgOptions::default()
                },
            )
            .unwrap();
            let mut target: Vec<_> = (0..130).map(|i| ((i * 13) % 31) as f64 - 15.0).collect();
            Components::from_laplacian(&graph)
                .center_in_place(&mut target)
                .unwrap();
            let rhs = graph.matvec(&target).unwrap();
            let guess: Vec<_> = target.iter().map(|x| x / 3.0).collect();
            let solve = Pcg::new(&graph, &preconditioner, SolveContext::default());
            let mut reference_workspace = PcgWorkspace::new(&preconditioner);
            let mut fused_workspace = PcgWorkspace::new(&preconditioner);
            for initial_guess in [None, Some(guess.as_slice()), None] {
                let options = PcgOptions {
                    residual_recompute_interval: 3,
                    ..PcgOptions::default()
                };
                let reference = solve
                    .kernel::<false, _, _>(
                        &rhs,
                        initial_guess,
                        options,
                        &mut reference_workspace,
                        &mut Unmetered,
                        &mut (),
                    )
                    .unwrap();
                let fused = solve
                    .kernel::<true, _, _>(
                        &rhs,
                        initial_guess,
                        options,
                        &mut fused_workspace,
                        &mut Unmetered,
                        &mut (),
                    )
                    .unwrap();
                assert_eq!(reference, fused);
                assert!(
                    reference_workspace
                        .solution
                        .iter()
                        .zip(&fused_workspace.solution)
                        .all(|(a, b)| a.to_bits() == b.to_bits())
                );
                assert!(reference.restarts() > 0);
            }
        }
    }

    #[cfg(feature = "experimental-components")]
    #[test]
    fn paired_norms_preserve_independent_bits_across_scale_and_zero() {
        let paired =
            |left: &[f64], right: &[f64]| paired_serial_norms(left, right, &mut Unmetered).unwrap();
        for len in [0, 1, 17, 257] {
            for left_scale in [0.0, 1e-310, 1e-150, 1.0, 1e150, 1e300] {
                for right_scale in [0.0, 1e-300, 1.0, 1e300] {
                    let left: Vec<_> = (0..len)
                        .map(|i| left_scale * (((i * 17) % 23) as f64 - 11.0))
                        .collect();
                    let right: Vec<_> = (0..len)
                        .map(|i| right_scale * (((i * 29) % 31) as f64 - 15.0))
                        .collect();
                    let pair = paired(&left, &right);
                    assert_eq!(
                        pair.0.to_bits(),
                        serial_norm(&left, &mut crate::cancel::Unmetered)
                            .unwrap()
                            .to_bits()
                    );
                    assert_eq!(
                        pair.1.to_bits(),
                        serial_norm(&right, &mut crate::cancel::Unmetered)
                            .unwrap()
                            .to_bits()
                    );
                }
            }
        }
        for left in [
            vec![f64::MAX, f64::MAX],
            vec![f64::INFINITY, 0.0],
            vec![f64::NAN, 1.0],
            vec![0.0, -0.0],
        ] {
            let right = [f64::from_bits(1), -f64::from_bits(2)];
            let pair = paired(&left, &right);
            assert_eq!(
                pair.0.to_bits(),
                serial_norm(&left, &mut crate::cancel::Unmetered)
                    .unwrap()
                    .to_bits()
            );
            assert_eq!(
                pair.1.to_bits(),
                serial_norm(&right, &mut crate::cancel::Unmetered)
                    .unwrap()
                    .to_bits()
            );
        }
    }

    #[cfg(feature = "parallel")]
    fn executor(threads: usize) -> crate::ParallelExecutor {
        crate::ParallelExecutor::new(crate::ParallelOptions {
            threads,
            min_parallel_len: 1,
            reduction_chunk_size: 16,
            ..crate::ParallelOptions::default()
        })
        .unwrap()
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn fixed_chunk_reductions_are_thread_count_invariant() {
        let (graph, preconditioner, _) = fixture();
        let left: Vec<f64> = (0..513)
            .map(|index| ((index * 17) % 101) as f64 / 13.0 - 3.0)
            .collect();
        let right: Vec<f64> = (0..513)
            .map(|index| ((index * 31 + 7) % 113) as f64 / 19.0 - 2.0)
            .collect();
        let serial = Pcg::new(&graph, &preconditioner, SolveContext::default());
        let reductions = |team: Team<'_>| {
            let solve = Pcg::new(
                &graph,
                &preconditioner,
                SolveContext {
                    team: Some(team),
                    ..SolveContext::default()
                },
            );
            (
                solve.dot(&left, &right, &mut Unmetered).unwrap().to_bits(),
                solve.norm(&right, &mut Unmetered).unwrap().to_bits(),
            )
        };
        // One worker included: the fixed-chunk tree depends only on lengths
        // and thresholds, and a one-lane view of a wider team is the same.
        let mut reference = None;
        for threads in [1, 2, 3, 4, 8] {
            let executor = executor(threads);
            for bits in [
                reductions(executor.team()),
                reductions(executor.team().single_lane()),
            ] {
                match reference {
                    Some(expected) => assert_eq!(bits, expected, "{threads} threads"),
                    None => reference = Some(bits),
                }
            }
        }
        // The team-free serial recurrence keeps its single compensated sum,
        // within rounding of the tree.
        let (dot, norm) = reference.unwrap();
        let serial_dot = serial.dot(&left, &right, &mut Unmetered).unwrap();
        let serial_norm = serial.norm(&right, &mut Unmetered).unwrap();
        assert!((f64::from_bits(dot) - serial_dot).abs() <= 2.0e-13 * (1.0 + serial_dot.abs()));
        assert!((f64::from_bits(norm) - serial_norm).abs() <= 3.0e-13 * (1.0 + serial_norm));
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn paired_norms_keep_each_team_reduction_tree() {
        let (graph, preconditioner, _) = fixture();
        let left: Vec<_> = (0..513).map(|i| ((i * 7) % 23) as f64 - 11.0).collect();
        let right: Vec<_> = (0..513).map(|i| ((i * 13) % 31) as f64 - 15.0).collect();
        for threads in [1, 2, 4] {
            let executor = executor(threads);
            let solve = Pcg::new(
                &graph,
                &preconditioner,
                SolveContext {
                    team: Some(executor.team()),
                    ..SolveContext::default()
                },
            );
            let pair = solve.paired_norms(&left, &right, &mut Unmetered).unwrap();
            assert_eq!(
                pair.0.to_bits(),
                solve.norm(&left, &mut Unmetered).unwrap().to_bits()
            );
            assert_eq!(
                pair.1.to_bits(),
                solve.norm(&right, &mut Unmetered).unwrap().to_bits()
            );
        }
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use crate::CmgOptions;
    use crate::cancel::StopAt;

    fn fixture(n: usize) -> (Laplacian, CmgPreconditioner, Vec<f64>) {
        let edges = (1..n)
            .map(|v| (v - 1, v, 1.0 + (v % 7) as f64 / 4.0))
            .chain((5..n).step_by(5).map(|v| (v - 5, v, 0.25)));
        let graph = Laplacian::from_edges(n, edges).unwrap();
        let preconditioner = CmgPreconditioner::build(
            &graph,
            CmgOptions {
                direct_threshold: 64,
                ..CmgOptions::default()
            },
        )
        .unwrap();
        let known: Vec<_> = (0..n).map(|v| ((v * 13) % 31) as f64 - 15.0).collect();
        let rhs = graph.matvec(&known).unwrap();
        (graph, preconditioner, rhs)
    }

    fn options() -> PcgOptions {
        PcgOptions {
            relative_tolerance: 1e-10,
            residual_recompute_interval: 4,
            ..PcgOptions::default()
        }
    }

    fn solve(
        graph: &Laplacian,
        preconditioner: &CmgPreconditioner,
        rhs: &[f64],
        workspace: &mut PcgWorkspace,
        context: SolveContext<'_>,
    ) -> Result<(PcgDiagnostics, Vec<u64>), CmgError> {
        let mut solution = vec![f64::NAN; graph.vertex_count()];
        let diagnostics = solve_pcg_into(
            graph,
            preconditioner,
            rhs,
            None,
            &mut solution,
            options(),
            workspace,
            context,
        )?;
        Ok((diagnostics, solution.iter().map(|x| x.to_bits()).collect()))
    }

    #[test]
    fn serial_pcg_cancels_mid_recurrence_and_the_workspace_restarts_exactly() {
        let (graph, preconditioner, rhs) = fixture(6_000);
        let mut workspace = PcgWorkspace::try_new(&preconditioner).unwrap();
        let expected = solve(
            &graph,
            &preconditioner,
            &rhs,
            &mut workspace,
            SolveContext::default(),
        )
        .unwrap();
        assert!(expected.0.iterations() > 3);
        let count = StopAt::never();
        let query = || count.query();
        let cancellable = SolveContext {
            cancel: Some(&query),
            ..SolveContext::default()
        };
        assert_eq!(
            solve(&graph, &preconditioner, &rhs, &mut workspace, cancellable).unwrap(),
            expected
        );
        let polls = count.polls();
        assert!(polls > 2 * expected.0.iterations(), "{polls}");
        let mut phases = std::collections::BTreeSet::new();
        for stop in (0..polls).step_by((polls / 97).max(1)).chain([polls - 1]) {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            let context = SolveContext {
                cancel: Some(&query),
                ..SolveContext::default()
            };
            match solve(&graph, &preconditioner, &rhs, &mut workspace, context) {
                Err(CmgError::Cancelled { phase }) => {
                    phases.insert(phase);
                }
                other => panic!("poll {stop} did not cancel: {other:?}"),
            }
            assert_eq!(cancel.polls(), stop + 1);
            assert_eq!(
                solve(
                    &graph,
                    &preconditioner,
                    &rhs,
                    &mut workspace,
                    SolveContext::default()
                )
                .unwrap(),
                expected
            );
        }
        assert!(phases.len() > 4, "{phases:?}");
    }

    /// The owned result of `solve_pcg_with` is a counted, fallible copy
    /// followed by the final checkpoint: cancelling at any poll inside the
    /// copy, or at the checkpoint after it, returns no result, and the same
    /// workspace then reproduces the reference exactly.
    #[test]
    fn owned_pcg_result_cancels_inside_and_after_its_copy() {
        let (graph, preconditioner, rhs) = fixture(40_000);
        let mut workspace = PcgWorkspace::try_new(&preconditioner).unwrap();
        let run = |workspace: &mut PcgWorkspace, cancel: Option<crate::Cancellation<'_>>| {
            solve_pcg_with(
                &graph,
                &preconditioner,
                &rhs,
                options(),
                workspace,
                SolveContext {
                    cancel,
                    ..SolveContext::default()
                },
            )
        };
        let reference = run(&mut workspace, None).unwrap();
        let count = StopAt::never();
        let query = || count.query();
        assert_eq!(run(&mut workspace, Some(&query)).unwrap(), reference);
        let polls = count.polls();
        // 40,000 copied values at eight units each span four poll intervals.
        let mut copy = 0;
        let mut last = None;
        for stop in polls - 8..polls {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            match run(&mut workspace, Some(&query)) {
                Err(CmgError::Cancelled { phase }) => {
                    copy += usize::from(phase == "PCG owned solution");
                    last = Some(phase);
                }
                other => panic!("poll {stop} did not cancel: {other:?}"),
            }
            assert_eq!(cancel.polls(), stop + 1);
            assert_eq!(run(&mut workspace, None).unwrap(), reference);
        }
        assert!(copy >= 5, "{copy} polls inside the owned copy");
        assert_eq!(last, Some("CMG application"));
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn planned_and_concurrent_pcg_join_every_lane_before_cancelling() {
        use crate::{ParallelExecutor, ParallelOptions};
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (graph, preconditioner, rhs) = fixture(60_000);
        let executor = ParallelExecutor::new(ParallelOptions {
            threads: 4,
            min_parallel_len: 64,
            reduction_chunk_size: 256,
            ..ParallelOptions::default()
        })
        .unwrap();
        let team = executor.team();
        let plan = ParallelCmgPlan::build(&preconditioner, team).unwrap();
        let planned = SolveContext {
            plan: Some(&plan),
            team: Some(team),
            ..SolveContext::default()
        };
        let mut workspace = PcgWorkspace::try_new(&preconditioner).unwrap();
        let expected = solve(&graph, &preconditioner, &rhs, &mut workspace, planned).unwrap();
        let total = {
            let calls = AtomicUsize::new(0);
            let query = || {
                calls.fetch_add(1, Ordering::SeqCst);
                false
            };
            let context = SolveContext {
                cancel: Some(&query),
                ..planned
            };
            assert_eq!(
                solve(&graph, &preconditioner, &rhs, &mut workspace, context).unwrap(),
                expected
            );
            calls.load(Ordering::SeqCst)
        };
        assert!(total > 50, "{total} queries");
        for threshold in [1, total / 8, total / 2, total - 1] {
            let calls = AtomicUsize::new(0);
            let query = || calls.fetch_add(1, Ordering::SeqCst) >= threshold;
            let context = SolveContext {
                cancel: Some(&query),
                ..planned
            };
            assert!(matches!(
                solve(&graph, &preconditioner, &rhs, &mut workspace, context),
                Err(CmgError::Cancelled { .. })
            ));
            let settled = calls.load(Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(5));
            assert_eq!(calls.load(Ordering::SeqCst), settled);
            assert_eq!(
                solve(&graph, &preconditioner, &rhs, &mut workspace, planned).unwrap(),
                expected
            );
        }

        let n = graph.vertex_count();
        let count = 4;
        let batch_rhs: Vec<f64> = (0..count).flat_map(|_| rhs.iter().copied()).collect();
        let mut workspaces =
            PcgBatchWorkspace::new_bank_with_budget(&preconditioner, count, usize::MAX).unwrap();
        let concurrent = SolveContext {
            team: Some(team),
            concurrent_right_hand_sides: true,
            ..SolveContext::default()
        };
        let mut run = |context: SolveContext<'_>| {
            let mut solutions = vec![0.0; n * count];
            let mut diagnostics = vec![PcgDiagnostics::default(); count];
            solve_pcg_batch_into(
                &graph,
                &preconditioner,
                PcgBatchRef::contiguous(&batch_rhs, count, n).unwrap(),
                None,
                PcgBatchMut::contiguous(&mut solutions, count, n).unwrap(),
                &mut diagnostics,
                options(),
                &mut workspaces,
                context,
            )
            .map(|()| {
                (
                    diagnostics,
                    solutions.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                )
            })
        };
        let reference = run(concurrent).unwrap();
        let total = {
            let calls = AtomicUsize::new(0);
            let query = || {
                calls.fetch_add(1, Ordering::SeqCst);
                false
            };
            assert_eq!(
                run(SolveContext {
                    cancel: Some(&query),
                    ..concurrent
                })
                .unwrap(),
                reference
            );
            calls.load(Ordering::SeqCst)
        };
        assert!(total > 50, "{total} queries");
        for threshold in [1, total / 8, total / 2, total - 1] {
            let calls = AtomicUsize::new(0);
            let query = || calls.fetch_add(1, Ordering::SeqCst) >= threshold;
            assert!(matches!(
                run(SolveContext {
                    cancel: Some(&query),
                    ..concurrent
                }),
                Err(CmgError::Cancelled { .. })
            ));
            let settled = calls.load(Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(5));
            assert_eq!(calls.load(Ordering::SeqCst), settled);
            assert_eq!(run(concurrent).unwrap(), reference);
        }
    }
}

/// One huge reduction leaf is counted while it runs: its lane polls the query
/// at the fixed cadence inside the leaf, and the fixed tree is independent of
/// the worker ceiling. On many leaves, a lane that observes cancellation stops
/// every lane at its next leaf: visited entries after the observation are
/// bounded by the ceiling's lanes, each finishing at most one poll interval.
#[cfg(all(test, feature = "parallel"))]
mod reduction_leaf_tests {
    use super::*;
    use crate::execution::{gauge, visits};
    use crate::{ParallelExecutor, ParallelOptions};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn vectors(len: usize) -> (Vec<f64>, Vec<f64>) {
        let left = (0..len)
            .map(|i| ((i * 7 % 101) as f64 - 50.0) * 1e-3)
            .collect();
        let right = (0..len).map(|i| ((i * 13 % 89) as f64).sin()).collect();
        (left, right)
    }

    #[test]
    fn single_huge_leaf_polls_inside_and_tree_bits_are_ceiling_independent() {
        let len = 1 << 20;
        let (left, right) = vectors(len);
        let executor = ParallelExecutor::new(ParallelOptions {
            threads: 4,
            ..ParallelOptions::default()
        })
        .unwrap();
        let team = executor.team();
        let polls = AtomicUsize::new(0);
        let count = || {
            polls.fetch_add(1, Ordering::SeqCst);
            false
        };
        // One leaf spanning the whole vector.
        let whole = fixed_chunk_dot(&left, &right, len, team, Some(&count)).unwrap();
        assert_eq!(
            whole.to_bits(),
            compensated_sum(left.iter().zip(&right).map(|(a, b)| a * b)).to_bits()
        );
        assert_eq!(
            polls.load(Ordering::SeqCst),
            len / crate::cancel::POLL_UNITS as usize
        );
        let squares = fixed_chunk_scaled_square_sum(&right, 3.0, len, team, Some(&count)).unwrap();
        assert_eq!(
            squares.to_bits(),
            compensated_sum(right.iter().map(|v| (v / 3.0) * (v / 3.0))).to_bits()
        );
        assert_eq!(
            polls.load(Ordering::SeqCst),
            2 * (len / crate::cancel::POLL_UNITS as usize)
        );
        // The cancelled lane stops inside the leaf at its third poll.
        let stopped = AtomicUsize::new(0);
        let stop = || stopped.fetch_add(1, Ordering::SeqCst) >= 2;
        assert!(matches!(
            fixed_chunk_dot(&left, &right, len, team, Some(&stop)),
            Err(CmgError::Cancelled { .. })
        ));
        assert_eq!(stopped.load(Ordering::SeqCst), 3);
        // Small leaves: the same balanced tree at every worker ceiling.
        let reference = fixed_chunk_dot(&left, &right, 4096, team, None)
            .unwrap()
            .to_bits();
        for threads in [1, 2, 3] {
            let executor = ParallelExecutor::new(ParallelOptions {
                threads,
                ..ParallelOptions::default()
            })
            .unwrap();
            assert_eq!(
                fixed_chunk_dot(&left, &right, 4096, executor.team(), None)
                    .unwrap()
                    .to_bits(),
                reference
            );
        }
    }

    type Reduction<'r> = dyn Fn(Cancellation<'_>) -> Result<f64, CmgError> + 'r;

    /// Run `reduce` with a query that turns true at its `stop`-th call and
    /// return the result, the entries visited when the query first returned
    /// true, and the entries visited in total.
    fn observed(
        watched: &[f64],
        stop: usize,
        reduce: impl FnOnce(Cancellation<'_>) -> Result<f64, CmgError>,
    ) -> (Result<f64, CmgError>, usize, usize) {
        let calls = AtomicUsize::new(0);
        let at_observation = AtomicUsize::new(usize::MAX);
        let query = || {
            let call = calls.fetch_add(1, Ordering::SeqCst);
            if call >= stop {
                let _ = at_observation.compare_exchange(
                    usize::MAX,
                    visits::count(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
                true
            } else {
                false
            }
        };
        visits::watch(watched);
        let result = reduce(&query);
        let total = visits::count();
        visits::unwatch();
        (result, at_observation.load(Ordering::SeqCst), total)
    }

    #[test]
    fn many_leaf_reductions_stop_after_cancellation_and_rerun_exactly() {
        let _witness = gauge::WITNESS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let len = 1 << 21;
        let chunk = 256;
        let (left, right) = vectors(len);
        // A ceiling of two lanes on a four-worker pool.
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let wide = Team::new(&pool, 2, ParallelOptions::default()).unwrap();
        let reference = (
            fixed_chunk_dot(&left, &right, chunk, wide, None).unwrap(),
            fixed_chunk_scaled_square_sum(&right, 3.0, chunk, wide, None).unwrap(),
            fixed_chunk_scale(&right, chunk, wide, None).unwrap(),
        );
        // The one-lane view runs the same tree on its calling lane: equal bits
        // and the same per-lane cancellation bounds.
        for team in [wide, wide.single_lane()] {
            let interval = crate::cancel::POLL_UNITS as usize;
            let after_bound = team.worker_ceiling() * (interval + 1024);
            let dot = fixed_chunk_dot(&left, &right, chunk, team, None).unwrap();
            let squares = fixed_chunk_scaled_square_sum(&right, 3.0, chunk, team, None).unwrap();
            let scale = fixed_chunk_scale(&right, chunk, team, None).unwrap();
            for stop in [0, 2] {
                let runs: [(&[f64], &Reduction<'_>); 3] = [
                    (&left, &|query| {
                        fixed_chunk_dot(&left, &right, chunk, team, Some(query))
                    }),
                    (&right, &|query| {
                        fixed_chunk_scaled_square_sum(&right, 3.0, chunk, team, Some(query))
                    }),
                    (&right, &|query| {
                        fixed_chunk_scale(&right, chunk, team, Some(query))
                    }),
                ];
                for (watched, run) in runs {
                    let (result, at, total) = observed(watched, stop, run);
                    assert!(matches!(result, Err(CmgError::Cancelled { .. })));
                    assert!(at <= total, "{at} > {total}");
                    assert!(
                        total - at <= after_bound,
                        "{} entries visited after cancellation was observed",
                        total - at
                    );
                    assert!(total <= (stop + 2 * team.worker_ceiling()) * (interval + 1024));
                    assert!(total < len / 4, "{total} of {len} entries visited");
                }
            }
            // The uncancelled reductions see every entry once and reproduce bits.
            visits::watch(&left);
            let again = fixed_chunk_dot(&left, &right, chunk, team, None).unwrap();
            assert_eq!(visits::count(), len);
            visits::unwatch();
            assert_eq!(again.to_bits(), dot.to_bits());
            assert_eq!(
                fixed_chunk_scaled_square_sum(&right, 3.0, chunk, team, None)
                    .unwrap()
                    .to_bits(),
                squares.to_bits()
            );
            assert_eq!(
                fixed_chunk_scale(&right, chunk, team, None)
                    .unwrap()
                    .to_bits(),
                scale.to_bits()
            );
            assert_eq!(dot.to_bits(), reference.0.to_bits());
            assert_eq!(squares.to_bits(), reference.1.to_bits());
            assert_eq!(scale.to_bits(), reference.2.to_bits());
        }
    }
}
