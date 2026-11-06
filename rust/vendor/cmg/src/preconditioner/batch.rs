//! Interleaved, fixed-cycle CMG applications sharing one hierarchy traversal.

use super::cycle::{Cycle, LevelParts, Parts, Store};
use super::{ApplyContext, CmgPreconditioner};
use crate::CmgError;
use crate::cancel::Meter;
use crate::storage::{add, bytes};

#[derive(Debug, Clone, Default)]
struct BatchLevel {
    rows: usize,
    coarse_rows: usize,
    factor_rows: usize,
    components: usize,
    residual: Vec<f64>,
    coarse_rhs: Vec<f64>,
    coarse_correction: Vec<f64>,
    forward: Vec<f64>,
    factor_solution: Vec<f64>,
    sums: Vec<f64>,
    corrections: Vec<f64>,
    means: Vec<f64>,
}
crate::storage::storage_fields!(
    BatchLevel,
    residual,
    coarse_rhs,
    coarse_correction,
    forward,
    factor_solution,
    sums,
    corrections,
    means
);

/// Reusable, fallibly admitted storage for interleaved stationary CMG cycles.
///
/// Capacity bounds the number of right-hand sides per application. Each call
/// packs only its actual columns, in vertex-major order; a smaller call reuses
/// the corresponding prefixes without touching or reading spare capacity.
#[derive(Debug, Clone)]
pub struct CmgBatchWorkspace {
    capacity: usize,
    storage: BatchStorage,
}

// Keep the scalar workspace inline: boxing would add an infallible allocation
// to the admitted constructor, solely to save a small fixed root-value size.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum BatchStorage {
    Scalar(crate::CmgWorkspace),
    Interleaved(Vec<BatchLevel>),
}
impl crate::storage::Storage for BatchStorage {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(crate::storage::Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        match self {
            Self::Scalar(workspace) => crate::storage::Storage::visit(workspace, visitor),
            Self::Interleaved(levels) => crate::storage::Storage::visit(levels, visitor),
        }
    }
}
crate::storage::storage_fields!(CmgBatchWorkspace, storage);

fn area(rows: usize, columns: usize) -> Result<usize, CmgError> {
    rows.checked_mul(columns).ok_or(CmgError::AllocationFailed {
        context: "CMG batch dimensions overflow",
    })
}

fn shape(cmg: &CmgPreconditioner, index: usize) -> (usize, usize, usize, usize) {
    let levels = cmg.hierarchy.levels();
    let rows = levels[index].graph().vertex_count();
    let coarse_rows = levels
        .get(index + 1)
        .map_or(0, |l| l.graph().vertex_count());
    let factor_rows = if index + 1 == levels.len() {
        cmg.direct_terminal
            .as_ref()
            .map_or(0, crate::GroundedLdl::active_dimension)
    } else {
        0
    };
    let components = cmg
        .coarse_centering
        .get(index)
        .map_or(0, |p| p.component_count());
    (rows, coarse_rows, factor_rows, components)
}

impl CmgBatchWorkspace {
    /// Checked requested heap layouts for this workspace, excluding the borrowed
    /// hierarchy, caller buffers, allocator overhead and any parallel runtime.
    pub fn allocation_bytes(cmg: &CmgPreconditioner, max_rhs: usize) -> Result<usize, CmgError> {
        Self::allocation_bytes_metered(cmg, max_rhs, &mut crate::cancel::Unmetered)
    }

    /// Cancellable `allocation_bytes`: the same value and errors, polling
    /// `cancel` between bounded level steps and once before returning.
    pub fn allocation_bytes_controlled(
        cmg: &CmgPreconditioner,
        max_rhs: usize,
        cancel: crate::Cancellation<'_>,
    ) -> Result<usize, CmgError> {
        Self::allocation_bytes_metered(cmg, max_rhs, &mut crate::cancel::CancelMeter::new(cancel))
    }

    fn allocation_bytes_metered<M: Meter>(
        cmg: &CmgPreconditioner,
        max_rhs: usize,
        meter: &mut M,
    ) -> Result<usize, CmgError> {
        if max_rhs == 0 {
            return Err(CmgError::InvalidOption {
                name: "CMG batch capacity",
                value: 0.0,
            });
        }
        if max_rhs == 1 {
            return cmg.workspace_allocation_bytes_metered(meter);
        }
        let mut total = bytes::<BatchLevel>(cmg.hierarchy.levels().len())?;
        for index in 0..cmg.hierarchy.levels().len() {
            meter.work("CMG batch layout", 64)?;
            let (rows, coarse, factor, components) = shape(cmg, index);
            let values = add(
                add(rows, area(coarse, 2)?)?,
                add(area(factor, 2)?, area(components, 3)?)?,
            )?;
            total = add(total, bytes::<f64>(area(values, max_rhs)?)?)?;
        }
        meter.checkpoint("CMG batch layout")?;
        Ok(total)
    }

    /// Admit all workspace layouts before the first allocation, then allocate
    /// each known-size buffer fallibly. Admission does not cover live parents.
    pub fn try_new(
        cmg: &CmgPreconditioner,
        max_rhs: usize,
        budget_bytes: usize,
    ) -> Result<Self, CmgError> {
        Self::try_new_metered(cmg, max_rhs, budget_bytes, &mut crate::cancel::Unmetered)
    }

    /// Cancellable `try_new` with the same layout, contents and errors. It
    /// polls `cancel` while computing the layout, before every allocation, at
    /// the fixed construction cadence while zero-filling and before returning.
    /// A true query drops the partial workspace and returns
    /// [`CmgError::Cancelled`].
    pub fn try_new_controlled(
        cmg: &CmgPreconditioner,
        max_rhs: usize,
        budget_bytes: usize,
        cancel: crate::Cancellation<'_>,
    ) -> Result<Self, CmgError> {
        Self::try_new_metered(
            cmg,
            max_rhs,
            budget_bytes,
            &mut crate::cancel::CancelMeter::new(cancel),
        )
    }

    fn try_new_metered<M: Meter>(
        cmg: &CmgPreconditioner,
        max_rhs: usize,
        budget_bytes: usize,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        if max_rhs == 1 {
            crate::storage::admit(cmg.workspace_allocation_bytes_metered(meter)?, budget_bytes)?;
            return Ok(Self {
                capacity: 1,
                storage: BatchStorage::Scalar(cmg.try_workspace_metered(meter)?),
            });
        }
        crate::storage::admit(
            Self::allocation_bytes_metered(cmg, max_rhs, meter)?,
            budget_bytes,
        )?;
        let mut levels = meter.try_capacity(cmg.hierarchy.levels().len(), "CMG batch levels")?;
        for index in 0..cmg.hierarchy.levels().len() {
            meter.work("CMG batch level", 64)?;
            let (rows, coarse_rows, factor_rows, components) = shape(cmg, index);
            let mut zero =
                |rows| meter.try_filled(area(rows, max_rhs)?, 0.0, "CMG batch workspace");
            levels.push(BatchLevel {
                rows,
                coarse_rows,
                factor_rows,
                components,
                residual: zero(rows)?,
                coarse_rhs: zero(coarse_rows)?,
                coarse_correction: zero(coarse_rows)?,
                forward: zero(factor_rows)?,
                factor_solution: zero(factor_rows)?,
                sums: zero(components)?,
                corrections: zero(components)?,
                means: zero(components)?,
            });
        }
        meter.checkpoint("CMG batch workspace finish")?;
        Ok(Self {
            capacity: max_rhs,
            storage: BatchStorage::Interleaved(levels),
        })
    }

    /// Maximum number of interleaved right-hand sides admitted at construction.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    fn validate(&self, cmg: &CmgPreconditioner, columns: usize) -> Result<(), CmgError> {
        if columns > self.capacity {
            return Err(CmgError::dimension(
                "CMG batch capacity",
                self.capacity,
                columns,
            ));
        }
        let levels = match &self.storage {
            BatchStorage::Scalar(workspace) => return cmg.validate_workspace(workspace),
            BatchStorage::Interleaved(levels) => levels,
        };
        if levels.len() != cmg.hierarchy.levels().len() {
            return Err(CmgError::dimension(
                "CMG batch levels",
                cmg.hierarchy.levels().len(),
                levels.len(),
            ));
        }
        for (index, level) in levels.iter().enumerate() {
            if (
                level.rows,
                level.coarse_rows,
                level.factor_rows,
                level.components,
            ) != shape(cmg, index)
            {
                return Err(CmgError::InvalidHierarchy {
                    context: "CMG batch workspace has different level dimensions",
                });
            }
        }
        Ok(())
    }
}

impl LevelParts for BatchLevel {
    #[inline]
    fn parts(&mut self, columns: usize) -> Parts<'_> {
        Parts {
            residual: &mut self.residual[..self.rows * columns],
            coarse_rhs: &mut self.coarse_rhs[..self.coarse_rows * columns],
            coarse_correction: &mut self.coarse_correction[..self.coarse_rows * columns],
            forward: &mut self.forward[..self.factor_rows * columns],
            solution: &mut self.factor_solution[..self.factor_rows * columns],
            sums: &mut self.sums[..self.components * columns],
            corrections: &mut self.corrections[..self.components * columns],
            means: &mut self.means[..self.components * columns],
        }
    }
}

/// Interleaved levels of one batch application with its lane count.
struct Interleaved<'w> {
    levels: &'w mut [BatchLevel],
    columns: usize,
}

impl Store for Interleaved<'_> {
    type Width = usize;
    type Level = BatchLevel;
    const TEAM_CENTERING: bool = false;

    #[inline]
    fn width(&self) -> usize {
        self.columns
    }

    #[inline]
    fn take(&mut self, index: usize) -> BatchLevel {
        core::mem::take(&mut self.levels[index])
    }

    #[inline]
    fn put(&mut self, index: usize, level: BatchLevel) {
        self.levels[index] = level;
    }
}

impl CmgPreconditioner {
    /// Apply the fixed stationary cycle to component-compatible right-hand sides.
    ///
    /// Inputs and outputs have exactly `vertex_count * columns` entries, with
    /// lane `j` of vertex `i` at `i * columns + j`. `columns` may be zero and
    /// must not exceed workspace capacity. No CMG-owned allocation, fine-level
    /// projection, adaptive solve or stopping test occurs. Each column uses the
    /// scalar cycle's edge, transfer, terminal and compensated centering order;
    /// coarse sums keep that serial order independently of the context's team.
    /// A capacity-one workspace runs the scalar cycle under the same context.
    pub fn apply_compatible_batch_into(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        columns: usize,
        workspace: &mut CmgBatchWorkspace,
        context: ApplyContext<'_>,
    ) -> Result<(), CmgError> {
        context.validate(self)?;
        self.validate_batch(rhs, output, columns, workspace)?;
        if columns == 0 {
            return Ok(());
        }
        match &mut workspace.storage {
            BatchStorage::Scalar(workspace) => {
                self.apply_compatible_into_with(rhs, output, workspace, context)
            }
            BatchStorage::Interleaved(levels) => with_meter!(context.cancel, meter => {
                Cycle {
                    cmg: self,
                    store: Interleaved { levels, columns },
                    #[cfg(feature = "parallel")]
                    plan: context.plan,
                    #[cfg(feature = "parallel")]
                    team: context.team,
                }
                .level(0, rhs, output, 1, meter, &mut ())
            }),
        }
    }

    fn validate_batch(
        &self,
        rhs: &[f64],
        output: &[f64],
        columns: usize,
        workspace: &CmgBatchWorkspace,
    ) -> Result<(), CmgError> {
        let expected = area(self.hierarchy.levels()[0].graph().vertex_count(), columns)?;
        for (context, actual) in [
            ("CMG batch RHS", rhs.len()),
            ("CMG batch output", output.len()),
        ] {
            if actual != expected {
                return Err(CmgError::dimension(context, expected, actual));
            }
        }
        workspace.validate(self, columns)
    }
}

/// Batches wider than one counted segment: every kernel of the cycle (edge
/// list, rows, restriction, prolongation, centering and the terminal factor)
/// counts its lane tiles, so no single count covers more than 1024 units; the
/// counts are deterministic, every lane keeps the width-four lane's bits, and
/// a cancellation at any count publishes nothing.
#[cfg(test)]
mod wide_batch_tests {
    use super::*;
    use crate::cancel::Recorder;
    use crate::{CmgOptions, Laplacian};

    const WIDE: usize = 1_100;

    fn fixture() -> (CmgPreconditioner, usize, Vec<f64>) {
        let n = 600;
        let edges = (1..n)
            .map(|v| (v - 1, v, 1.0 + (v % 7) as f64 / 4.0))
            .chain((9..n).step_by(9).map(|v| (v - 9, v, 0.5)));
        let graph = Laplacian::from_edges(n, edges).unwrap();
        let preconditioner = CmgPreconditioner::build(
            &graph,
            CmgOptions {
                direct_threshold: 64,
                ..CmgOptions::default()
            },
        )
        .unwrap();
        assert!(preconditioner.hierarchy.levels().len() > 1);
        // Four compatible right-hand sides, interleaved.
        let lanes: Vec<Vec<f64>> = (0..4)
            .map(|lane| {
                let known: Vec<f64> = (0..n)
                    .map(|v| (((v + 3) * (lane + 5)) % 37) as f64 - 18.0)
                    .collect();
                graph.matvec(&known).unwrap()
            })
            .collect();
        let base = (0..n)
            .flat_map(|v| lanes.iter().map(move |rhs| rhs[v]))
            .collect();
        (preconditioner, n, base)
    }

    fn bits(values: &[f64]) -> Vec<u64> {
        values.iter().map(|value| value.to_bits()).collect()
    }

    #[test]
    fn wide_batches_count_lane_tiles_and_keep_lane_bits() {
        let (preconditioner, n, base) = fixture();
        let mut narrow = CmgBatchWorkspace::try_new(&preconditioner, 4, usize::MAX).unwrap();
        let mut expected = vec![0.0; n * 4];
        preconditioner
            .apply_compatible_batch_into(
                &base,
                &mut expected,
                4,
                &mut narrow,
                ApplyContext::default(),
            )
            .unwrap();
        let rhs: Vec<f64> = (0..n)
            .flat_map(|v| (0..WIDE).map(move |lane| (v, lane)))
            .map(|(v, lane)| base[v * 4 + lane % 4])
            .collect();
        let mut workspace = CmgBatchWorkspace::try_new(&preconditioner, WIDE, usize::MAX).unwrap();
        let mut output = vec![0.0; n * WIDE];
        preconditioner
            .apply_compatible_batch_into(
                &rhs,
                &mut output,
                WIDE,
                &mut workspace,
                ApplyContext::default(),
            )
            .unwrap();
        let lane_bits: Vec<u64> = (0..n * WIDE)
            .map(|index| expected[(index / WIDE) * 4 + (index % WIDE) % 4].to_bits())
            .collect();
        assert_eq!(bits(&output), lane_bits);

        let mut run = |recorder: &mut Recorder, output: &mut [f64]| {
            let BatchStorage::Interleaved(levels) = &mut workspace.storage else {
                panic!("interleaved wide workspace");
            };
            Cycle {
                cmg: &preconditioner,
                store: Interleaved {
                    levels,
                    columns: WIDE,
                },
                #[cfg(feature = "parallel")]
                plan: None,
                #[cfg(feature = "parallel")]
                team: None,
            }
            .level(0, &rhs, output, 1, recorder, &mut ())
        };
        let mut full = Recorder::default();
        let mut counted = vec![f64::NAN; n * WIDE];
        run(&mut full, &mut counted).unwrap();
        assert_eq!(bits(&counted), lane_bits);
        assert!(
            full.max_units <= 1024,
            "{} units in one count",
            full.max_units
        );
        let mut again = Recorder::default();
        run(&mut again, &mut counted).unwrap();
        assert_eq!((again.advances, again.units), (full.advances, full.units));
        for stop in [
            1,
            full.advances / 3,
            full.advances / 2,
            full.advances - 1,
            full.advances,
        ] {
            let mut cancelled = Recorder {
                stop_at: Some(stop),
                ..Recorder::default()
            };
            assert!(matches!(
                run(&mut cancelled, &mut counted),
                Err(CmgError::Cancelled { .. })
            ));
            assert_eq!(cancelled.advances, stop);
            assert!(cancelled.max_units <= 1024);
        }
        let mut output = vec![0.0; n * WIDE];
        preconditioner
            .apply_compatible_batch_into(
                &rhs,
                &mut output,
                WIDE,
                &mut workspace,
                ApplyContext::default(),
            )
            .unwrap();
        assert_eq!(bits(&output), lane_bits);
    }

    /// The planned wide batch on a team: parallel rows and row operators run
    /// in lane tiles of at most 1024 lanes, lanes keep the planned width-four
    /// bits, and cancellation at sampled polls joins and rebuilds exactly.
    #[cfg(feature = "parallel")]
    #[test]
    fn planned_wide_batches_tile_parallel_rows_and_rebuild_exactly() {
        use crate::{ParallelCmgPlan, ParallelExecutor, ParallelOptions};
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (preconditioner, n, base) = fixture();
        let executor = ParallelExecutor::new(ParallelOptions {
            threads: 4,
            min_parallel_len: 64,
            ..ParallelOptions::default()
        })
        .unwrap();
        let team = executor.team();
        let widest = AtomicUsize::new(0);
        let mut rows = vec![0.0; 8 * WIDE];
        crate::preconditioner::rows_parallel(team, None, &mut rows, WIDE, |row, first, values| {
            widest.fetch_max(values.len(), Ordering::SeqCst);
            for (lane, value) in (first..).zip(values.iter_mut()) {
                *value = (row * WIDE + lane) as f64;
            }
        })
        .unwrap();
        assert_eq!(widest.load(Ordering::SeqCst), 1024);
        assert!(
            rows.iter()
                .enumerate()
                .all(|(index, &value)| value == index as f64)
        );

        let plan = ParallelCmgPlan::build(&preconditioner, team).unwrap();
        let planned = ApplyContext {
            plan: Some(&plan),
            team: Some(team),
            cancel: None,
        };
        let mut narrow = CmgBatchWorkspace::try_new(&preconditioner, 4, usize::MAX).unwrap();
        let mut expected = vec![0.0; n * 4];
        preconditioner
            .apply_compatible_batch_into(&base, &mut expected, 4, &mut narrow, planned)
            .unwrap();
        let rhs: Vec<f64> = (0..n)
            .flat_map(|v| (0..WIDE).map(move |lane| (v, lane)))
            .map(|(v, lane)| base[v * 4 + lane % 4])
            .collect();
        let lane_bits: Vec<u64> = (0..n * WIDE)
            .map(|index| expected[(index / WIDE) * 4 + (index % WIDE) % 4].to_bits())
            .collect();
        let mut workspace = CmgBatchWorkspace::try_new(&preconditioner, WIDE, usize::MAX).unwrap();
        let mut output = vec![0.0; n * WIDE];
        let calls = AtomicUsize::new(0);
        let count = || {
            calls.fetch_add(1, Ordering::SeqCst);
            false
        };
        preconditioner
            .apply_compatible_batch_into(
                &rhs,
                &mut output,
                WIDE,
                &mut workspace,
                ApplyContext {
                    cancel: Some(&count),
                    ..planned
                },
            )
            .unwrap();
        assert_eq!(bits(&output), lane_bits);
        let total = calls.load(Ordering::SeqCst);
        assert!(total > 8, "{total} queries");
        for threshold in [1, total / 4, total / 2, total - 1] {
            let calls = AtomicUsize::new(0);
            let query = || calls.fetch_add(1, Ordering::SeqCst) >= threshold;
            match preconditioner.apply_compatible_batch_into(
                &rhs,
                &mut output,
                WIDE,
                &mut workspace,
                ApplyContext {
                    cancel: Some(&query),
                    ..planned
                },
            ) {
                Err(CmgError::Cancelled { .. }) => {}
                // Lane polls depend on chunk claiming; a run may end first.
                Ok(()) => assert!(
                    calls.load(Ordering::SeqCst) <= threshold,
                    "query {threshold} ignored"
                ),
                Err(error) => panic!("threshold {threshold}: {error}"),
            }
            preconditioner
                .apply_compatible_batch_into(&rhs, &mut output, WIDE, &mut workspace, planned)
                .unwrap();
            assert_eq!(bits(&output), lane_bits);
        }
    }
}
