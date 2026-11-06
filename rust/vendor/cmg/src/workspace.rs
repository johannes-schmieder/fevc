//! Reusable caller-owned storage for CMG applications.

use crate::cancel::{Meter, try_filled, try_reserved};
use crate::components::{CenteringPlan, CenteringWorkspace, ComponentWorkspace};
use crate::{CmgError, CmgHierarchy, Components, GroundedLdl};

crate::storage::storage_fields!(
    LevelWorkspace,
    residual,
    coarse_rhs,
    coarse_correction,
    factor_forward,
    factor_solution
);
crate::storage::storage_fields!(
    CmgWorkspace,
    levels,
    component_workspace,
    centering_workspaces,
    dimensions,
    projected_rhs
);

#[derive(Debug, Clone, Default)]
pub(crate) struct LevelWorkspace {
    pub(crate) residual: Vec<f64>,
    pub(crate) coarse_rhs: Vec<f64>,
    pub(crate) coarse_correction: Vec<f64>,
    pub(crate) factor_forward: Vec<f64>,
    pub(crate) factor_solution: Vec<f64>,
}

/// Reusable work arrays for applying one fixed CMG preconditioner.
#[derive(Debug, Clone)]
pub struct CmgWorkspace {
    levels: Vec<LevelWorkspace>,
    component_workspace: ComponentWorkspace,
    centering_workspaces: Vec<CenteringWorkspace>,
    dimensions: Vec<usize>,
    projected_rhs: Vec<f64>,
}

impl CmgWorkspace {
    /// Dimension-only bound for the same requested layouts as `allocation_bytes`.
    /// The public caller has validated `max_levels >= 1`.
    pub(crate) fn allocation_bound(vertices: usize, max_levels: usize) -> Result<usize, CmgError> {
        use crate::storage::{add, bytes};
        let coarse_levels = max_levels - 1;
        let repeated = |size: usize, count: usize| {
            size.checked_mul(count).ok_or(CmgError::InvalidHierarchy {
                context: "CMG scalar workspace bound overflows",
            })
        };
        let mut total = add(
            bytes::<LevelWorkspace>(max_levels)?,
            bytes::<usize>(max_levels)?,
        )?;
        total = add(total, bytes::<CenteringWorkspace>(coarse_levels)?)?;
        // Finest projected RHS and its component projection banks.
        total = add(total, bytes::<f64>(vertices)?)?;
        total = add(total, ComponentWorkspace::allocation_bytes(vertices)?)?;
        // Every level retains a residual. Every nonterminal level additionally
        // retains two coarse vectors and one centering workspace.
        total = add(total, repeated(bytes::<f64>(vertices)?, max_levels)?)?;
        let coarse = add(
            bytes::<[f64; 2]>(vertices)?,
            CenteringWorkspace::allocation_bytes(vertices)?,
        )?;
        total = add(total, repeated(coarse, coarse_levels)?)?;
        // Only the terminal level can own factor forward/solution banks.
        add(total, bytes::<[f64; 2]>(vertices)?)
    }

    pub(crate) fn allocation_bytes(
        hierarchy: &CmgHierarchy,
        direct_terminal: Option<&GroundedLdl>,
        finest_components: &Components,
        coarse_centering: &[CenteringPlan],
        meter: &mut impl Meter,
    ) -> Result<usize, CmgError> {
        use crate::storage::{add, bytes};
        let levels = hierarchy.levels();
        let mut total = add(
            bytes::<LevelWorkspace>(levels.len())?,
            bytes::<usize>(levels.len())?,
        )?;
        total = add(total, bytes::<CenteringWorkspace>(coarse_centering.len())?)?;
        total = add(total, bytes::<f64>(levels[0].graph().vertex_count())?)?;
        total = add(
            total,
            ComponentWorkspace::allocation_bytes(finest_components.count())?,
        )?;
        for plan in coarse_centering {
            meter.work("CMG workspace centering layouts", 32)?;
            total = add(total, plan.workspace_allocation_bytes()?)?;
        }
        for (index, level) in levels.iter().enumerate() {
            meter.work("CMG workspace level layouts", 64)?;
            total = add(total, bytes::<f64>(level.graph().vertex_count())?)?;
            let coarse = levels
                .get(index + 1)
                .map_or(0, |l| l.graph().vertex_count());
            total = add(total, bytes::<[f64; 2]>(coarse)?)?;
        }
        let total = add(
            total,
            bytes::<[f64; 2]>(direct_terminal.map_or(0, GroundedLdl::active_dimension))?,
        )?;
        meter.checkpoint("CMG workspace layouts")?;
        Ok(total)
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn required_bytes(
        hierarchy: &CmgHierarchy,
        direct_terminal: Option<&GroundedLdl>,
        finest_components: &Components,
        coarse_centering: &[CenteringPlan],
    ) -> usize {
        let levels = hierarchy.levels();
        let last = levels.len().saturating_sub(1);
        let level_values = levels
            .iter()
            .enumerate()
            .fold(0_usize, |values, (index, level)| {
                let dimension = level.graph().vertex_count();
                let coarse_dimension = levels
                    .get(index + 1)
                    .map(|coarse| coarse.graph().vertex_count())
                    .unwrap_or(0);
                let factor_dimension = if index == last {
                    direct_terminal
                        .map(GroundedLdl::active_dimension)
                        .unwrap_or(0)
                } else {
                    0
                };
                values
                    .saturating_add(dimension)
                    .saturating_add(coarse_dimension.saturating_mul(2))
                    .saturating_add(factor_dimension.saturating_mul(2))
            });
        let centering_bytes = coarse_centering
            .iter()
            .map(CenteringPlan::workspace_bytes)
            .fold(0_usize, usize::saturating_add);
        level_values
            .saturating_mul(core::mem::size_of::<f64>())
            .saturating_add(finest_components.workspace_bytes())
            .saturating_add(centering_bytes)
            .saturating_add(
                levels
                    .first()
                    .map(|level| level.graph().vertex_count())
                    .unwrap_or(0)
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
    }

    pub(crate) fn new(
        hierarchy: &CmgHierarchy,
        direct_terminal: Option<&GroundedLdl>,
        finest_components: &Components,
        coarse_centering: &[CenteringPlan],
    ) -> Self {
        let dimensions: Vec<usize> = hierarchy
            .levels()
            .iter()
            .map(|level| level.graph().vertex_count())
            .collect();
        let projected_rhs = vec![0.0; dimensions.first().copied().unwrap_or(0)];
        let last = dimensions.len().saturating_sub(1);
        let levels = dimensions
            .iter()
            .enumerate()
            .map(|(index, &dimension)| {
                let coarse_dimension = dimensions.get(index + 1).copied().unwrap_or(0);
                let factor_dimension = if index == last {
                    direct_terminal
                        .map(GroundedLdl::active_dimension)
                        .unwrap_or(0)
                } else {
                    0
                };
                LevelWorkspace {
                    residual: vec![0.0; dimension],
                    coarse_rhs: vec![0.0; coarse_dimension],
                    coarse_correction: vec![0.0; coarse_dimension],
                    factor_forward: vec![0.0; factor_dimension],
                    factor_solution: vec![0.0; factor_dimension],
                }
            })
            .collect();
        debug_assert_eq!(coarse_centering.len(), dimensions.len().saturating_sub(1));
        let component_workspace = finest_components.workspace();
        let centering_workspaces = coarse_centering
            .iter()
            .map(CenteringPlan::workspace)
            .collect();
        Self {
            levels,
            component_workspace,
            centering_workspaces,
            dimensions,
            projected_rhs,
        }
    }

    pub(crate) fn try_new(
        hierarchy: &CmgHierarchy,
        direct_terminal: Option<&GroundedLdl>,
        finest_components: &Components,
        coarse_centering: &[CenteringPlan],
        meter: &mut impl Meter,
    ) -> Result<Self, CmgError> {
        let mut dimensions =
            try_reserved(meter, hierarchy.levels().len(), "CMG workspace dimensions")?;
        for level in hierarchy.levels() {
            meter.work("CMG workspace dimensions", 16)?;
            dimensions.push(level.graph().vertex_count());
        }
        let projected_rhs = try_filled(
            meter,
            dimensions.first().copied().unwrap_or(0),
            0.0,
            "CMG workspace projected RHS",
        )?;
        let last = dimensions.len().saturating_sub(1);
        let mut levels = try_reserved(meter, dimensions.len(), "CMG workspace levels")?;
        for (index, &dimension) in dimensions.iter().enumerate() {
            meter.work("CMG workspace level dimensions", 32)?;
            let coarse_dimension = dimensions.get(index + 1).copied().unwrap_or(0);
            let factor_dimension = if index == last {
                direct_terminal
                    .map(GroundedLdl::active_dimension)
                    .unwrap_or(0)
            } else {
                0
            };
            levels.push(LevelWorkspace {
                residual: try_filled(meter, dimension, 0.0, "CMG workspace residual")?,
                coarse_rhs: try_filled(meter, coarse_dimension, 0.0, "CMG workspace coarse RHS")?,
                coarse_correction: try_filled(
                    meter,
                    coarse_dimension,
                    0.0,
                    "CMG workspace coarse correction",
                )?,
                factor_forward: try_filled(
                    meter,
                    factor_dimension,
                    0.0,
                    "CMG workspace factor forward",
                )?,
                factor_solution: try_filled(
                    meter,
                    factor_dimension,
                    0.0,
                    "CMG workspace factor solution",
                )?,
            });
        }
        debug_assert_eq!(coarse_centering.len(), dimensions.len().saturating_sub(1));
        let component_workspace = finest_components.try_workspace(meter)?;
        let mut centering_workspaces =
            try_reserved(meter, coarse_centering.len(), "CMG centering workspaces")?;
        for plan in coarse_centering {
            meter.work("CMG centering workspace plan", 16)?;
            centering_workspaces.push(plan.try_workspace(meter)?);
        }
        meter.checkpoint("CMG workspace finish")?;
        Ok(Self {
            levels,
            component_workspace,
            centering_workspaces,
            dimensions,
            projected_rhs,
        })
    }

    /// Return the number of hierarchy levels represented by this workspace.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Return fine-to-coarse vector dimensions.
    #[must_use]
    pub fn dimensions(&self) -> &[usize] {
        &self.dimensions
    }

    /// Return the number of heap bytes reserved by the principal work arrays.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        let level_bytes: usize = self
            .levels
            .iter()
            .map(|level| {
                [
                    level.residual.len(),
                    level.coarse_rhs.len(),
                    level.coarse_correction.len(),
                    level.factor_forward.len(),
                    level.factor_solution.len(),
                ]
                .into_iter()
                .sum::<usize>()
                .saturating_mul(8)
            })
            .sum();
        let centering_bytes: usize = self
            .centering_workspaces
            .iter()
            .map(CenteringWorkspace::byte_len)
            .sum();
        level_bytes
            .saturating_add(self.component_workspace.byte_len())
            .saturating_add(centering_bytes)
            .saturating_add(self.projected_rhs.len().saturating_mul(8))
    }

    pub(crate) fn validate(
        &self,
        hierarchy: &CmgHierarchy,
        direct_terminal: Option<&GroundedLdl>,
        finest_components: &Components,
        coarse_centering: &[CenteringPlan],
    ) -> Result<(), CmgError> {
        if self.levels.len() != hierarchy.levels().len() {
            return Err(CmgError::dimension(
                "CmgWorkspace level count",
                hierarchy.levels().len(),
                self.levels.len(),
            ));
        }
        if self.centering_workspaces.len() != coarse_centering.len() {
            return Err(CmgError::dimension(
                "CmgWorkspace centering level count",
                coarse_centering.len(),
                self.centering_workspaces.len(),
            ));
        }
        finest_components.validate_workspace(&self.component_workspace)?;
        for (plan, centering) in coarse_centering.iter().zip(&self.centering_workspaces) {
            plan.validate_workspace(centering)?;
        }
        let fine_dimension = hierarchy
            .levels()
            .first()
            .map(|level| level.graph().vertex_count())
            .unwrap_or(0);
        validate_length(
            "CmgWorkspace projected rhs",
            fine_dimension,
            self.projected_rhs.len(),
        )?;
        let last = self.levels.len().saturating_sub(1);
        for (index, (workspace, level)) in self.levels.iter().zip(hierarchy.levels()).enumerate() {
            let dimension = level.graph().vertex_count();
            validate_length("CmgWorkspace residual", dimension, workspace.residual.len())?;
            let coarse_dimension = hierarchy
                .levels()
                .get(index + 1)
                .map(|coarse| coarse.graph().vertex_count())
                .unwrap_or(0);
            validate_length(
                "CmgWorkspace coarse rhs",
                coarse_dimension,
                workspace.coarse_rhs.len(),
            )?;
            validate_length(
                "CmgWorkspace coarse correction",
                coarse_dimension,
                workspace.coarse_correction.len(),
            )?;
            let factor_dimension = if index == last {
                direct_terminal
                    .map(GroundedLdl::active_dimension)
                    .unwrap_or(0)
            } else {
                0
            };
            validate_length(
                "CmgWorkspace factor forward",
                factor_dimension,
                workspace.factor_forward.len(),
            )?;
            validate_length(
                "CmgWorkspace factor solution",
                factor_dimension,
                workspace.factor_solution.len(),
            )?;
        }
        Ok(())
    }

    pub(crate) fn take_projected_rhs(&mut self) -> Vec<f64> {
        core::mem::take(&mut self.projected_rhs)
    }

    pub(crate) fn put_projected_rhs(&mut self, projected_rhs: Vec<f64>) {
        self.projected_rhs = projected_rhs;
    }

    pub(crate) fn take_component(&mut self) -> ComponentWorkspace {
        core::mem::take(&mut self.component_workspace)
    }

    pub(crate) fn put_component(&mut self, workspace: ComponentWorkspace) {
        self.component_workspace = workspace;
    }
}

/// One scalar level moved out of the workspace during its recursive visit.
pub(crate) struct ScalarLevel {
    level: LevelWorkspace,
    centering: CenteringWorkspace,
}

impl crate::preconditioner::LevelParts for ScalarLevel {
    #[inline]
    fn parts(&mut self, _: usize) -> crate::preconditioner::Parts<'_> {
        let (sums, corrections, means) = self.centering.parts();
        crate::preconditioner::Parts {
            residual: &mut self.level.residual,
            coarse_rhs: &mut self.level.coarse_rhs,
            coarse_correction: &mut self.level.coarse_correction,
            forward: &mut self.level.factor_forward,
            solution: &mut self.level.factor_solution,
            sums,
            corrections,
            means,
        }
    }
}

impl crate::preconditioner::Store for &mut CmgWorkspace {
    type Width = crate::preconditioner::One;
    type Level = ScalarLevel;
    const TEAM_CENTERING: bool = true;

    #[inline]
    fn width(&self) -> crate::preconditioner::One {
        crate::preconditioner::One
    }

    #[inline]
    fn take(&mut self, index: usize) -> ScalarLevel {
        ScalarLevel {
            level: core::mem::take(&mut self.levels[index]),
            centering: self
                .centering_workspaces
                .get_mut(index)
                .map(core::mem::take)
                .unwrap_or_default(),
        }
    }

    #[inline]
    fn put(&mut self, index: usize, level: ScalarLevel) {
        self.levels[index] = level.level;
        if let Some(centering) = self.centering_workspaces.get_mut(index) {
            *centering = level.centering;
        }
    }
}

fn validate_length(context: &'static str, expected: usize, actual: usize) -> Result<(), CmgError> {
    if expected == actual {
        Ok(())
    } else {
        Err(CmgError::dimension(context, expected, actual))
    }
}

#[cfg(test)]
mod tests {
    use crate::{CmgOptions, CmgPreconditioner, Laplacian};

    #[test]
    fn recursive_apply_overwrites_stale_coarse_corrections() {
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
        let known: Vec<f64> = (0..128).map(|index| (index as f64 / 11.0).sin()).collect();
        let rhs = graph.matvec(&known).unwrap();
        let mut workspace = preconditioner.workspace();
        let mut output = vec![0.0; 128];
        preconditioner
            .apply_compatible_into(&rhs, &mut output, &mut workspace)
            .unwrap();
        let expected = output.clone();

        output.fill(f64::NAN);
        for level in &mut workspace.levels {
            level.coarse_correction.fill(f64::NAN);
        }
        preconditioner
            .apply_compatible_into(&rhs, &mut output, &mut workspace)
            .unwrap();
        assert_eq!(
            expected
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            output
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
    }
}

#[cfg(test)]
mod cancellable_preparation_tests {
    use super::CmgWorkspace;
    use crate::cancel::{CancelMeter, StopAt, Unmetered};
    use crate::components::CenteringPlan;
    use crate::{CmgError, CmgOptions, CmgPreconditioner, Components, Laplacian};

    // Run `prepare` with a meter that polls after every bounded operation,
    // once uncancelled and then cancelled at each poll. Return the phases of
    // the cancelled polls; every attempt must stop at its first true poll.
    fn cancel_at_every_poll<T: core::fmt::Debug>(
        mut prepare: impl FnMut(&mut CancelMeter<'_>) -> Result<T, CmgError>,
    ) -> (T, Vec<&'static str>) {
        let count = StopAt::never();
        let query = || count.query();
        let complete = prepare(&mut CancelMeter::every_operation(&query)).unwrap();
        let mut phases = Vec::new();
        for stop in 0..count.polls() {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            match prepare(&mut CancelMeter::every_operation(&query)) {
                Err(CmgError::Cancelled { phase }) => phases.push(phase),
                other => panic!("poll {stop} did not cancel: {other:?}"),
            }
            assert_eq!(cancel.polls(), stop + 1);
        }
        (complete, phases)
    }

    fn occurrences(phases: &[&str], phase: &str) -> usize {
        phases.iter().filter(|&&observed| observed == phase).count()
    }

    #[test]
    fn component_and_centering_workspaces_cancel_inside_initialization() {
        // 300 disjoint edges: 300 components, so each bank needs three fill
        // chunks after its allocation poll.
        let graph =
            Laplacian::from_edges(600, (0..300).map(|pair| (2 * pair, 2 * pair + 1, 1.0))).unwrap();
        let components = Components::from_laplacian(&graph);
        let plan = CenteringPlan::from_laplacian(&graph, &mut Unmetered).unwrap();
        assert_eq!(components.count(), 300);

        let (workspace, phases) = cancel_at_every_poll(|meter| components.try_workspace(meter));
        assert_eq!(
            format!("{workspace:?}"),
            format!("{:?}", components.try_workspace(&mut Unmetered).unwrap())
        );
        for phase in [
            "component workspace sums",
            "component workspace corrections",
            "component workspace scales",
            "component workspace scale corrections",
            "component workspace means",
            "component workspace projection corrections",
            "component workspace representatives",
        ] {
            // One poll before the allocation and one per initialized chunk.
            assert_eq!(occurrences(&phases, phase), 4, "{phase}");
        }
        assert_eq!(phases.len(), 7 * 4);

        let (workspace, phases) = cancel_at_every_poll(|meter| plan.try_workspace(meter));
        assert_eq!(
            format!("{workspace:?}"),
            format!("{:?}", plan.try_workspace(&mut Unmetered).unwrap())
        );
        for phase in [
            "centering workspace sums",
            "centering workspace corrections",
            "centering workspace means",
        ] {
            assert_eq!(occurrences(&phases, phase), 4, "{phase}");
        }
        assert_eq!(phases.len(), 3 * 4);
    }

    #[test]
    fn scalar_workspace_and_layouts_cancel_at_every_poll_then_rebuild_and_apply() {
        for direct_threshold in [2, 128] {
            let n = 300;
            let graph =
                Laplacian::from_edges(n, (1..n).map(|v| (v - 1, v, 1.0 + (v % 3) as f64))).unwrap();
            let options = CmgOptions {
                direct_threshold,
                ..CmgOptions::default()
            };
            let preconditioner = CmgPreconditioner::build(&graph, options).unwrap();
            let hierarchy = preconditioner.hierarchy();
            let terminal = preconditioner.terminal_factor();
            let components = Components::from_laplacian(&graph);
            let plans: Vec<_> = hierarchy.levels()[1..]
                .iter()
                .map(|level| CenteringPlan::from_laplacian(level.graph(), &mut Unmetered).unwrap())
                .collect();
            assert!(hierarchy.levels().len() > 1 || direct_threshold == 128);

            let ordinary_bytes = CmgWorkspace::allocation_bytes(
                hierarchy,
                terminal,
                &components,
                &plans,
                &mut Unmetered,
            )
            .unwrap();
            let (bytes, phases) = cancel_at_every_poll(|meter| {
                CmgWorkspace::allocation_bytes(hierarchy, terminal, &components, &plans, meter)
            });
            assert_eq!(bytes, ordinary_bytes);
            assert!(occurrences(&phases, "CMG workspace level layouts") >= 1);
            assert_eq!(phases.last(), Some(&"CMG workspace layouts"));

            let (mut controlled, phases) = cancel_at_every_poll(|meter| {
                CmgWorkspace::try_new(hierarchy, terminal, &components, &plans, meter)
            });
            // Each residual polls before its allocation and once per counted
            // 128-value chunk; the finest one (300 values) has three chunks.
            let residual_polls: usize = hierarchy
                .levels()
                .iter()
                .map(|level| 1 + level.graph().vertex_count().div_ceil(128))
                .sum();
            assert!(residual_polls >= 4);
            assert_eq!(
                occurrences(&phases, "CMG workspace residual"),
                residual_polls
            );
            assert!(occurrences(&phases, "component workspace sums") >= 2);
            if direct_threshold == 2 {
                assert!(occurrences(&phases, "centering workspace sums") >= 2);
            } else {
                assert!(occurrences(&phases, "CMG workspace factor forward") >= 2);
            }
            assert_eq!(phases.last(), Some(&"CMG workspace finish"));

            // A cancelled attempt leaves nothing behind: a fresh preparation
            // equals the ordinary workspace and applies identical bits.
            let mut ordinary =
                CmgWorkspace::try_new(hierarchy, terminal, &components, &plans, &mut Unmetered)
                    .unwrap();
            assert_eq!(format!("{controlled:?}"), format!("{ordinary:?}"));
            assert_eq!(controlled.byte_len(), ordinary.byte_len());
            controlled
                .validate(hierarchy, terminal, &components, &plans)
                .unwrap();
            let known: Vec<_> = (0..n).map(|vertex| (vertex as f64 / 7.0).sin()).collect();
            let rhs = graph.matvec(&known).unwrap();
            let mut expected = vec![0.0; n];
            let mut actual = vec![0.0; n];
            preconditioner
                .apply_compatible_into(&rhs, &mut expected, &mut ordinary)
                .unwrap();
            preconditioner
                .apply_compatible_into(&rhs, &mut actual, &mut controlled)
                .unwrap();
            assert!(
                actual
                    .iter()
                    .zip(&expected)
                    .all(|(actual, expected)| actual.to_bits() == expected.to_bits())
            );
        }
    }
}
