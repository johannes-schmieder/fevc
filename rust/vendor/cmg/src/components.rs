//! Deterministic connected components and Laplacian null-space operations.

use crate::cancel::{Meter, Unmetered, chunked, row_lanes, try_filled as filled};
use crate::execution::Team;
use crate::preconditioner::Width;
use crate::{CmgError, Laplacian, ValidationOptions};

crate::storage::storage_fields!(
    ComponentWorkspace,
    sums,
    corrections,
    scales,
    scale_corrections,
    means,
    projection_corrections,
    representatives
);
crate::storage::storage_fields!(CenteringWorkspace, sums, corrections, means);
crate::storage::storage_fields!(CenteringPlan, labels, sizes);
crate::storage::storage_fields!(Components, labels, sizes);

impl crate::storage::Storage for CenteringLabels {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(crate::storage::Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        match self {
            Self::Single => Ok(()),
            Self::Compact(v) => crate::storage::Storage::visit(v, visitor),
            Self::Native(v) => crate::storage::Storage::visit(v, visitor),
        }
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) struct ComponentWorkspace {
    sums: Vec<f64>,
    corrections: Vec<f64>,
    scales: Vec<f64>,
    scale_corrections: Vec<f64>,
    means: Vec<f64>,
    projection_corrections: Vec<f64>,
    representatives: Vec<usize>,
}

impl ComponentWorkspace {
    pub(crate) fn allocation_bytes(count: usize) -> Result<usize, CmgError> {
        crate::storage::add(
            crate::storage::bytes::<[f64; 6]>(count)?,
            crate::storage::bytes::<usize>(count)?,
        )
    }
    fn new(component_count: usize) -> Self {
        Self {
            sums: vec![0.0; component_count],
            corrections: vec![0.0; component_count],
            scales: vec![0.0; component_count],
            scale_corrections: vec![0.0; component_count],
            means: vec![0.0; component_count],
            projection_corrections: vec![0.0; component_count],
            representatives: vec![usize::MAX; component_count],
        }
    }

    pub(crate) fn try_new(count: usize, meter: &mut impl Meter) -> Result<Self, CmgError> {
        Ok(Self {
            sums: filled(meter, count, 0.0, "component workspace sums")?,
            corrections: filled(meter, count, 0.0, "component workspace corrections")?,
            scales: filled(meter, count, 0.0, "component workspace scales")?,
            scale_corrections: filled(meter, count, 0.0, "component workspace scale corrections")?,
            means: filled(meter, count, 0.0, "component workspace means")?,
            projection_corrections: filled(
                meter,
                count,
                0.0,
                "component workspace projection corrections",
            )?,
            representatives: filled(
                meter,
                count,
                usize::MAX,
                "component workspace representatives",
            )?,
        })
    }

    pub(crate) fn validate(&self, component_count: usize) -> Result<(), CmgError> {
        for (context, actual) in [
            ("ComponentWorkspace sums", self.sums.len()),
            ("ComponentWorkspace corrections", self.corrections.len()),
            ("ComponentWorkspace scales", self.scales.len()),
            (
                "ComponentWorkspace scale corrections",
                self.scale_corrections.len(),
            ),
            ("ComponentWorkspace means", self.means.len()),
            (
                "ComponentWorkspace projection corrections",
                self.projection_corrections.len(),
            ),
            (
                "ComponentWorkspace representatives",
                self.representatives.len(),
            ),
        ] {
            if actual != component_count {
                return Err(CmgError::dimension(context, component_count, actual));
            }
        }
        Ok(())
    }

    pub(crate) fn byte_len(&self) -> usize {
        Self::byte_len_for(self.sums.len())
    }

    pub(crate) const fn byte_len_for(component_count: usize) -> usize {
        component_count
            .saturating_mul(6)
            .saturating_mul(8)
            .saturating_add(component_count.saturating_mul(core::mem::size_of::<usize>()))
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CenteringWorkspace {
    sums: Vec<f64>,
    corrections: Vec<f64>,
    means: Vec<f64>,
}

impl CenteringWorkspace {
    pub(crate) fn allocation_bytes(component_count: usize) -> Result<usize, CmgError> {
        crate::storage::bytes::<[f64; 3]>(component_count)
    }

    fn new(component_count: usize) -> Self {
        Self {
            sums: vec![0.0; component_count],
            corrections: vec![0.0; component_count],
            means: vec![0.0; component_count],
        }
    }

    fn try_new(component_count: usize, meter: &mut impl Meter) -> Result<Self, CmgError> {
        Ok(Self {
            sums: filled(meter, component_count, 0.0, "centering workspace sums")?,
            corrections: filled(
                meter,
                component_count,
                0.0,
                "centering workspace corrections",
            )?,
            means: filled(meter, component_count, 0.0, "centering workspace means")?,
        })
    }

    fn validate(&self, component_count: usize) -> Result<(), CmgError> {
        for (context, actual) in [
            ("CenteringWorkspace sums", self.sums.len()),
            ("CenteringWorkspace corrections", self.corrections.len()),
            ("CenteringWorkspace means", self.means.len()),
        ] {
            if actual != component_count {
                return Err(CmgError::dimension(context, component_count, actual));
            }
        }
        Ok(())
    }

    pub(crate) fn byte_len(&self) -> usize {
        Self::byte_len_for(self.sums.len())
    }

    pub(crate) const fn byte_len_for(component_count: usize) -> usize {
        component_count.saturating_mul(3).saturating_mul(8)
    }

    /// Sums, corrections and means for one scalar centering.
    pub(crate) fn parts(&mut self) -> (&mut [f64], &mut [f64], &mut [f64]) {
        (&mut self.sums, &mut self.corrections, &mut self.means)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CenteringLabels {
    Single,
    Compact(Vec<u32>),
    Native(Vec<usize>),
}

/// Minimal component metadata needed for internal recursive centering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CenteringPlan {
    vertex_count: usize,
    labels: CenteringLabels,
    sizes: Vec<usize>,
}

impl CenteringPlan {
    pub(crate) fn component_count(&self) -> usize {
        self.sizes.len()
    }

    pub(crate) fn from_laplacian<M: Meter>(
        graph: &Laplacian,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        let Components { labels, sizes, .. } = Components::from_endpoints(
            graph.vertex_count(),
            graph.edges().iter().map(|edge| (edge.u(), edge.v())),
            meter,
        )?;
        #[cfg(feature = "experimental-components")]
        let labels = labels.into_vec();
        let vertex_count = labels.len();
        let component_count = sizes.len();
        let labels = if component_count <= 1 {
            CenteringLabels::Single
        } else if component_count <= u32::MAX as usize {
            let mut compact = meter.try_capacity(labels.len(), "centering compact labels")?;
            if M::POLLS {
                for &label in &labels {
                    meter.work("centering compact labels", 8)?;
                    compact.push(label as u32);
                }
            } else {
                compact.extend(labels.iter().map(|&label| label as u32));
            }
            CenteringLabels::Compact(compact)
        } else {
            CenteringLabels::Native(labels)
        };
        Ok(Self {
            vertex_count,
            labels,
            sizes,
        })
    }
    pub(crate) fn workspace_allocation_bytes(&self) -> Result<usize, CmgError> {
        CenteringWorkspace::allocation_bytes(self.sizes.len())
    }
    pub(crate) fn workspace(&self) -> CenteringWorkspace {
        CenteringWorkspace::new(self.sizes.len())
    }

    pub(crate) fn try_workspace(
        &self,
        meter: &mut impl Meter,
    ) -> Result<CenteringWorkspace, CmgError> {
        CenteringWorkspace::try_new(self.sizes.len(), meter)
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn workspace_bytes(&self) -> usize {
        CenteringWorkspace::byte_len_for(self.sizes.len())
    }

    pub(crate) fn validate_workspace(
        &self,
        workspace: &CenteringWorkspace,
    ) -> Result<(), CmgError> {
        workspace.validate(self.sizes.len())
    }

    pub(crate) fn byte_len(&self) -> usize {
        let label_bytes = match &self.labels {
            CenteringLabels::Single => 0,
            CenteringLabels::Compact(labels) => labels.len().saturating_mul(4),
            CenteringLabels::Native(labels) => {
                labels.len().saturating_mul(core::mem::size_of::<usize>())
            }
        };
        label_bytes.saturating_add(
            self.sizes
                .len()
                .saturating_mul(core::mem::size_of::<usize>()),
        )
    }

    /// Remove each component's compensated mean from every lane.
    ///
    /// Each lane sums its component's values in ascending vertex order with
    /// Neumaier compensation, divides `sum + correction` by the component size
    /// and subtracts the mean. A non-finite value is reported before any value
    /// is changed.
    pub(crate) fn center_lanes<W: Width, M: Meter>(
        &self,
        values: &mut [f64],
        width: W,
        sums: &mut [f64],
        corrections: &mut [f64],
        means: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = width.get();
        if values.len() != self.vertex_count * columns {
            return Err(CmgError::dimension(
                "CenteringPlan::center_in_place",
                self.vertex_count * columns,
                values.len(),
            ));
        }
        let slots = self.sizes.len() * columns;
        for (context, actual) in [
            ("CenteringWorkspace sums", sums.len()),
            ("CenteringWorkspace corrections", corrections.len()),
            ("CenteringWorkspace means", means.len()),
        ] {
            if actual != slots {
                return Err(CmgError::dimension(context, slots, actual));
            }
        }
        match &self.labels {
            CenteringLabels::Single => center_labeled(
                |_| 0,
                values,
                columns,
                &self.sizes,
                sums,
                corrections,
                means,
                meter,
            ),
            CenteringLabels::Compact(labels) => center_labeled(
                |vertex| labels[vertex] as usize,
                values,
                columns,
                &self.sizes,
                sums,
                corrections,
                means,
                meter,
            ),
            CenteringLabels::Native(labels) => center_labeled(
                |vertex| labels[vertex],
                values,
                columns,
                &self.sizes,
                sums,
                corrections,
                means,
                meter,
            ),
        }
    }

    /// Whether a scalar planned cycle centers this level with the team's
    /// fixed-chunk reduction: one component above the reduction threshold.
    #[cfg(feature = "parallel")]
    pub(crate) fn uses_team_centering(&self, length: usize, team: Team<'_>) -> bool {
        matches!(self.labels, CenteringLabels::Single) && should_parallel_centering(length, team)
    }

    /// Center one component with fixed-chunk team sums (see
    /// [`Self::uses_team_centering`]).
    #[cfg(feature = "parallel")]
    pub(crate) fn center_single_with_team<M: Meter>(
        &self,
        values: &mut [f64],
        means: &mut [f64],
        team: Team<'_>,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        if values.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "CenteringPlan::center_in_place",
                self.vertex_count,
                values.len(),
            ));
        }
        if means.len() != self.sizes.len() {
            return Err(CmgError::dimension(
                "CenteringWorkspace means",
                self.sizes.len(),
                means.len(),
            ));
        }
        center_single_component(values, &self.sizes, means, team, meter)
    }
}

/// The compensated per-lane centering of [`CenteringPlan::center_lanes`].
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn center_labeled<M: Meter>(
    label: impl Fn(usize) -> usize,
    values: &mut [f64],
    columns: usize,
    sizes: &[usize],
    sums: &mut [f64],
    corrections: &mut [f64],
    means: &mut [f64],
    meter: &mut M,
) -> Result<(), CmgError> {
    crate::cancel::zero(meter, "CMG centering", sums)?;
    crate::cancel::zero(meter, "CMG centering", corrections)?;
    let vertices = values.len() / columns;
    row_lanes(
        meter,
        "CMG centering sums",
        vertices,
        columns,
        1,
        #[inline(always)]
        |vertices, lanes| {
            for vertex in vertices {
                let component = label(vertex);
                for lane in lanes.clone() {
                    let value = values[vertex * columns + lane];
                    if !value.is_finite() {
                        return Err(CmgError::NonFiniteMatrixValue {
                            row: vertex,
                            column: lane,
                            value,
                        });
                    }
                    let slot = component * columns + lane;
                    neumaier_add(&mut sums[slot], &mut corrections[slot], value);
                }
            }
            Ok(())
        },
    )?;
    row_lanes(
        meter,
        "CMG centering means",
        sizes.len(),
        columns,
        1,
        #[inline(always)]
        |components, lanes| {
            for component in components {
                for lane in lanes.clone() {
                    let slot = component * columns + lane;
                    means[slot] = (sums[slot] + corrections[slot]) / sizes[component] as f64;
                }
            }
            Ok(())
        },
    )?;
    row_lanes(
        meter,
        "CMG centering",
        vertices,
        columns,
        1,
        #[inline(always)]
        |vertices, lanes| {
            for vertex in vertices {
                let component = label(vertex);
                for lane in lanes.clone() {
                    values[vertex * columns + lane] -= means[component * columns + lane];
                }
            }
            Ok(())
        },
    )
}

/// Connected-component metadata for a weighted graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Components {
    #[cfg(not(feature = "experimental-components"))]
    labels: Vec<usize>,
    // Labels never change length after construction. A boxed slice leaves room
    // for the experimental layout flag without growing the metadata embedded
    // in every terminal factor (and its containing preconditioner).
    #[cfg(feature = "experimental-components")]
    labels: Box<[usize]>,
    sizes: Vec<usize>,
    #[cfg(feature = "experimental-components")]
    contiguous: bool,
}

impl Components {
    /// Compute deterministic component labels.
    ///
    /// Labels are assigned in first-vertex order, so relabeling does not depend
    /// on edge input order.
    #[must_use]
    pub fn from_laplacian(graph: &Laplacian) -> Self {
        Self::from_endpoints(
            graph.vertex_count(),
            graph.edges().iter().map(|edge| (edge.u(), edge.v())),
            &mut Unmetered,
        )
        .expect("component label allocation")
    }

    /// Union-find components in first-vertex label order, with bounded
    /// counted operations and allocation checkpoints when metered.
    pub(crate) fn from_endpoints<M: Meter, I>(
        vertex_count: usize,
        endpoints: I,
        meter: &mut M,
    ) -> Result<Self, CmgError>
    where
        I: IntoIterator<Item = (usize, usize)>,
    {
        let mut parent = meter.try_capacity(vertex_count, "component parents")?;
        for vertex in 0..vertex_count {
            meter.work("component parents", 8)?;
            parent.push(vertex);
        }
        for (left, right) in endpoints {
            meter.work("component union", 32)?;
            let left_root = find_root(&mut parent, left, meter)?;
            let right_root = find_root(&mut parent, right, meter)?;
            if left_root != right_root {
                parent[left_root.max(right_root)] = left_root.min(right_root);
            }
        }
        for vertex in 0..vertex_count {
            meter.work("component compress", 8)?;
            parent[vertex] = find_root(&mut parent, vertex, meter)?;
        }
        let mut root_to_label =
            meter.try_filled(vertex_count, usize::MAX, "component root labels")?;
        let mut labels = meter.try_filled(vertex_count, 0, "component labels")?;
        let mut component_count = 0;
        for vertex in 0..vertex_count {
            meter.work("component labels", 32)?;
            let root = parent[vertex];
            let label = if root_to_label[root] == usize::MAX {
                let next = component_count;
                component_count += 1;
                root_to_label[root] = next;
                next
            } else {
                root_to_label[root]
            };
            labels[vertex] = label;
        }
        let mut sizes = meter.try_filled(component_count, 0, "component sizes")?;
        for &label in &labels {
            meter.work("component sizes", 8)?;
            sizes[label] += 1;
        }
        #[cfg(feature = "experimental-components")]
        let contiguous = {
            let mut sorted = true;
            for pair in labels.windows(2) {
                meter.work("component contiguous", 8)?;
                sorted &= pair[0] <= pair[1];
            }
            sorted
        };
        // An exact-capacity conversion does not allocate or traverse the bank.
        // A polling meter refuses excess allocator capacity rather than hide a
        // shrink/reallocation.
        #[cfg(feature = "experimental-components")]
        if M::POLLS && labels.capacity() != labels.len() {
            return Err(CmgError::AllocationFailed {
                context: "component boxed labels capacity",
            });
        }
        Ok(Self {
            #[cfg(feature = "experimental-components")]
            contiguous,
            #[cfg(not(feature = "experimental-components"))]
            labels,
            #[cfg(feature = "experimental-components")]
            labels: labels.into_boxed_slice(),
            sizes,
        })
    }

    #[cfg(all(test, feature = "experimental-components"))]
    pub(crate) fn try_from_endpoints(
        vertex_count: usize,
        endpoints: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self, CmgError> {
        Self::from_endpoints(vertex_count, endpoints, &mut Unmetered)
    }

    /// Return the number of connected components.
    #[must_use]
    pub fn count(&self) -> usize {
        self.sizes.len()
    }

    /// Return the component label of every vertex.
    #[must_use]
    pub fn labels(&self) -> &[usize] {
        &self.labels
    }

    /// Return component sizes in label order.
    #[must_use]
    pub fn sizes(&self) -> &[usize] {
        &self.sizes
    }

    pub(crate) fn workspace(&self) -> ComponentWorkspace {
        ComponentWorkspace::new(self.count())
    }

    pub(crate) fn try_workspace(
        &self,
        meter: &mut impl Meter,
    ) -> Result<ComponentWorkspace, CmgError> {
        ComponentWorkspace::try_new(self.count(), meter)
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn workspace_bytes(&self) -> usize {
        ComponentWorkspace::byte_len_for(self.count())
    }

    pub(crate) fn validate_workspace(
        &self,
        workspace: &ComponentWorkspace,
    ) -> Result<(), CmgError> {
        workspace.validate(self.count())
    }

    pub(crate) fn byte_len(&self) -> usize {
        self.labels
            .len()
            .saturating_add(self.sizes.len())
            .saturating_mul(core::mem::size_of::<usize>())
    }

    /// Return component-wise compensated sums of a vector.
    pub fn sums(&self, values: &[f64]) -> Result<Vec<f64>, CmgError> {
        let mut workspace = self.workspace();
        self.compensated_sums_into(
            values,
            "Components::sums",
            &mut workspace.sums,
            &mut workspace.corrections,
            &mut Unmetered,
        )?;
        Ok(workspace.sums)
    }

    /// Verify that a right-hand side is numerically compatible with every
    /// component null space.
    pub fn validate_rhs(&self, rhs: &[f64], options: ValidationOptions) -> Result<(), CmgError> {
        let options = options.validate()?;
        let mut workspace = self.workspace();
        self.compatibility_data_into(
            rhs,
            "Components::validate_rhs",
            &mut workspace,
            &mut Unmetered,
        )?;
        self.validate_component_sums(&workspace.sums, &workspace.scales, options, &mut Unmetered)
    }

    /// Project accepted floating-point compatibility defects onto the exact
    /// Laplacian range and return the Euclidean norm of the removed component.
    ///
    /// A component whose sum exceeds `compatibility_tolerance` relative to its
    /// one-norm is rejected rather than modified. The mean is first removed
    /// from every vertex. Any residual summation error is then absorbed at the
    /// smallest-magnitude vertex in the component, with vertex index breaking
    /// ties, so that the correction is both deterministic and numerically
    /// effective.
    pub fn project_rhs_in_place(
        &self,
        rhs: &mut [f64],
        options: ValidationOptions,
    ) -> Result<f64, CmgError> {
        let mut workspace = self.workspace();
        self.project_rhs_in_place_with_workspace(rhs, options, &mut workspace, &mut Unmetered)
    }

    pub(crate) fn project_rhs_in_place_with_workspace<M: Meter>(
        &self,
        rhs: &mut [f64],
        options: ValidationOptions,
        workspace: &mut ComponentWorkspace,
        meter: &mut M,
    ) -> Result<f64, CmgError> {
        const PHASE: &str = "Components::project_rhs_in_place";
        let options = options.validate()?;
        workspace.validate(self.count())?;
        self.compatibility_data_into(rhs, PHASE, workspace, meter)?;
        self.validate_component_sums(&workspace.sums, &workspace.scales, options, meter)?;

        let ComponentWorkspace {
            sums,
            corrections,
            means,
            projection_corrections,
            representatives,
            ..
        } = workspace;
        chunked(meter, PHASE, self.count(), 1, |range| {
            for component in range {
                means[component] = sums[component] / self.sizes[component] as f64;
            }
            Ok(())
        })?;
        chunked(meter, PHASE, rhs.len(), 1, |range| {
            for vertex in range {
                rhs[vertex] -= means[self.labels[vertex]];
            }
            Ok(())
        })?;

        self.stable_representatives_into(rhs, representatives, meter)?;
        crate::cancel::zero(meter, PHASE, projection_corrections)?;
        for _ in 0..2 {
            self.compensated_sums_into(rhs, PHASE, sums, corrections, meter)?;
            chunked(meter, PHASE, self.count(), 1, |range| {
                for component in range {
                    let residual_sum = sums[component];
                    rhs[representatives[component]] -= residual_sum;
                    projection_corrections[component] += residual_sum;
                }
                Ok(())
            })?;
        }

        let mut projection_scale = 0.0_f64;
        chunked(meter, PHASE, self.count(), 2, |range| {
            for component in range {
                let mean = means[component];
                projection_scale = projection_scale.max(mean.abs());
                projection_scale =
                    projection_scale.max((mean + projection_corrections[component]).abs());
            }
            Ok(())
        })?;
        if projection_scale == 0.0 {
            return Ok(0.0);
        }
        // Continue the standard floating-point sum in component order.
        let mut projection_squared = core::iter::empty::<f64>().sum::<f64>();
        chunked(meter, PHASE, self.count(), 2, |range| {
            for component in range {
                let mean = means[component];
                let regular = mean / projection_scale;
                let representative = (mean + projection_corrections[component]) / projection_scale;
                projection_squared += (self.sizes[component] - 1) as f64 * regular * regular
                    + representative * representative;
            }
            Ok(())
        })?;
        Ok(projection_scale * projection_squared.sqrt())
    }

    /// Subtract the mean within every component in place.
    pub fn center_in_place(&self, values: &mut [f64]) -> Result<(), CmgError> {
        let mut workspace = self.workspace();
        self.center_in_place_with_workspace(values, &mut workspace, &mut Unmetered)
    }

    pub(crate) fn center_in_place_with_workspace<M: Meter>(
        &self,
        values: &mut [f64],
        workspace: &mut ComponentWorkspace,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        const PHASE: &str = "Components::center_in_place";
        if values.len() != self.labels.len() {
            return Err(CmgError::dimension(PHASE, self.labels.len(), values.len()));
        }
        workspace.validate(self.count())?;
        #[cfg(feature = "experimental-components")]
        if self.contiguous && self.count() > 1 {
            // Every component occupies one slice in label order. Accumulate in
            // registers and subtract one constant per slice, preserving the
            // existing vertex order within each compensated sum.
            self.contiguous_means(values, workspace, meter)?;
            // Complete all finite-value checks before mutating the input, as
            // the general labeled path does on an error.
            let mut start = 0;
            for (&size, &mean) in self.sizes.iter().zip(&workspace.means) {
                let component = &mut values[start..start + size];
                chunked(meter, PHASE, size, 1, |range| {
                    for value in &mut component[range] {
                        *value -= mean;
                    }
                    Ok(())
                })?;
                start += size;
            }
            return Ok(());
        }
        let ComponentWorkspace {
            sums,
            corrections,
            means,
            ..
        } = workspace;
        self.compensated_sums_into(values, PHASE, sums, corrections, meter)?;
        chunked(meter, PHASE, self.count(), 1, |range| {
            for component in range {
                means[component] = sums[component] / self.sizes[component] as f64;
            }
            Ok(())
        })?;
        if self.count() == 1 {
            let mean = means[0];
            chunked(meter, PHASE, values.len(), 1, |range| {
                for value in &mut values[range] {
                    *value -= mean;
                }
                Ok(())
            })
        } else {
            chunked(meter, PHASE, values.len(), 1, |range| {
                for vertex in range {
                    values[vertex] -= means[self.labels[vertex]];
                }
                Ok(())
            })
        }
    }

    /// Per-slice compensated sums and means of contiguous components; every
    /// value is checked before the caller mutates any.
    #[cfg(feature = "experimental-components")]
    fn contiguous_means<M: Meter>(
        &self,
        values: &[f64],
        workspace: &mut ComponentWorkspace,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let mut start = 0;
        for (component, &size) in self.sizes.iter().enumerate() {
            let mut sum = 0.0;
            let mut correction = 0.0;
            chunked(meter, "Components::center_in_place", size, 1, |range| {
                let first = range.start + start;
                for (offset, &value) in values[first..range.end + start].iter().enumerate() {
                    if !value.is_finite() {
                        return Err(CmgError::NonFiniteMatrixValue {
                            row: first + offset,
                            column: 0,
                            value,
                        });
                    }
                    neumaier_add(&mut sum, &mut correction, value);
                }
                Ok(())
            })?;
            workspace.sums[component] = sum + correction;
            workspace.corrections[component] = correction;
            workspace.means[component] = workspace.sums[component] / size as f64;
            start += size;
        }
        Ok(())
    }

    /// Center a vector and accumulate its dot product with `left` in vertex order.
    /// All input checks finish before the first centered value is written.
    #[cfg(feature = "experimental-components")]
    pub(crate) fn center_and_dot_with_workspace<M: Meter>(
        &self,
        values: &mut [f64],
        left: &[f64],
        workspace: &mut ComponentWorkspace,
        meter: &mut M,
    ) -> Result<f64, CmgError> {
        const PHASE: &str = "Components::center_in_place";
        if values.len() != self.labels.len() {
            return Err(CmgError::dimension(PHASE, self.labels.len(), values.len()));
        }
        if left.len() != values.len() {
            return Err(CmgError::dimension(
                "Components::center_and_dot",
                values.len(),
                left.len(),
            ));
        }
        workspace.validate(self.count())?;
        if self.contiguous {
            self.contiguous_means(values, workspace, meter)?;
        } else {
            let ComponentWorkspace {
                sums,
                corrections,
                means,
                ..
            } = workspace;
            self.compensated_sums_into(values, PHASE, sums, corrections, meter)?;
            chunked(meter, PHASE, self.count(), 1, |range| {
                for component in range {
                    means[component] = sums[component] / self.sizes[component] as f64;
                }
                Ok(())
            })?;
        }

        let mut sum = 0.0;
        let mut correction = 0.0;
        if self.contiguous {
            let mut start = 0;
            for (&size, &mean) in self.sizes.iter().zip(&workspace.means) {
                chunked(meter, PHASE, size, 2, |range| {
                    for vertex in range.start + start..range.end + start {
                        values[vertex] -= mean;
                        crate::graph::compensated_add(
                            &mut sum,
                            &mut correction,
                            left[vertex] * values[vertex],
                        );
                    }
                    Ok(())
                })?;
                start += size;
            }
        } else {
            let means = &workspace.means;
            chunked(meter, PHASE, values.len(), 2, |range| {
                for vertex in range {
                    values[vertex] -= means[self.labels[vertex]];
                    crate::graph::compensated_add(
                        &mut sum,
                        &mut correction,
                        left[vertex] * values[vertex],
                    );
                }
                Ok(())
            })?;
        }
        Ok(sum + correction)
    }

    /// Center in place, using fixed-chunk team sums for one component above
    /// the team's reduction threshold and compensated serial sums otherwise.
    pub(crate) fn center_with_team<M: Meter>(
        &self,
        values: &mut [f64],
        workspace: &mut ComponentWorkspace,
        #[cfg_attr(not(feature = "parallel"), allow(unused_variables))] team: Option<Team<'_>>,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        #[cfg(feature = "parallel")]
        if let Some(team) = team {
            if self.count() == 1 && should_parallel_centering(values.len(), team) {
                if values.len() != self.labels.len() {
                    return Err(CmgError::dimension(
                        "Components::center_in_place",
                        self.labels.len(),
                        values.len(),
                    ));
                }
                workspace.validate(self.count())?;
                return center_single_component(
                    values,
                    &self.sizes,
                    &mut workspace.means,
                    team,
                    meter,
                );
            }
        }
        self.center_in_place_with_workspace(values, workspace, meter)
    }

    fn stable_representatives_into<M: Meter>(
        &self,
        values: &[f64],
        representatives: &mut [usize],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        const PHASE: &str = "Components representatives";
        chunked(meter, PHASE, representatives.len(), 1, |range| {
            representatives[range].fill(usize::MAX);
            Ok(())
        })?;
        chunked(meter, PHASE, values.len(), 2, |range| {
            for vertex in range {
                let value = values[vertex];
                let label = self.labels[vertex];
                let current = representatives[label];
                if current == usize::MAX
                    || value.abs() < values[current].abs()
                    || (value.abs() == values[current].abs() && vertex < current)
                {
                    representatives[label] = vertex;
                }
            }
            Ok(())
        })
    }

    fn compatibility_data_into<M: Meter>(
        &self,
        values: &[f64],
        context: &'static str,
        workspace: &mut ComponentWorkspace,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        if values.len() != self.labels.len() {
            return Err(CmgError::dimension(
                context,
                self.labels.len(),
                values.len(),
            ));
        }
        let ComponentWorkspace {
            sums,
            corrections,
            scales,
            scale_corrections,
            ..
        } = workspace;
        for bank in [
            &mut *sums,
            &mut *corrections,
            &mut *scales,
            &mut *scale_corrections,
        ] {
            crate::cancel::zero(meter, context, bank)?;
        }
        chunked(meter, context, values.len(), 2, |range| {
            for vertex in range {
                let value = values[vertex];
                let label = self.labels[vertex];
                if !value.is_finite() {
                    return Err(CmgError::NonFiniteMatrixValue {
                        row: vertex,
                        column: 0,
                        value,
                    });
                }
                neumaier_add(&mut sums[label], &mut corrections[label], value);
                neumaier_add(
                    &mut scales[label],
                    &mut scale_corrections[label],
                    value.abs(),
                );
            }
            Ok(())
        })?;
        chunked(meter, context, self.count(), 2, |range| {
            for component in range {
                sums[component] += corrections[component];
                scales[component] += scale_corrections[component];
            }
            Ok(())
        })
    }

    fn validate_component_sums<M: Meter>(
        &self,
        sums: &[f64],
        scales: &[f64],
        options: ValidationOptions,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        chunked(
            meter,
            "Components compatibility",
            self.count(),
            1,
            |range| {
                for component in range {
                    let tolerance = options.compatibility_tolerance * scales[component].max(1.0);
                    if sums[component].abs() > tolerance {
                        return Err(CmgError::IncompatibleLaplacianRhs {
                            component,
                            sum: sums[component],
                            tolerance,
                        });
                    }
                }
                Ok(())
            },
        )
    }

    fn compensated_sums_into<M: Meter>(
        &self,
        values: &[f64],
        context: &'static str,
        sums: &mut [f64],
        corrections: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        if values.len() != self.labels.len() {
            return Err(CmgError::dimension(
                context,
                self.labels.len(),
                values.len(),
            ));
        }
        if sums.len() != self.count() {
            return Err(CmgError::dimension(context, self.count(), sums.len()));
        }
        if corrections.len() != self.count() {
            return Err(CmgError::dimension(
                context,
                self.count(),
                corrections.len(),
            ));
        }
        crate::cancel::zero(meter, context, sums)?;
        crate::cancel::zero(meter, context, corrections)?;
        if self.count() == 1 {
            let mut sum = 0.0;
            let mut correction = 0.0;
            chunked(meter, context, values.len(), 1, |range| {
                for vertex in range {
                    let value = values[vertex];
                    if !value.is_finite() {
                        return Err(CmgError::NonFiniteMatrixValue {
                            row: vertex,
                            column: 0,
                            value,
                        });
                    }
                    neumaier_add(&mut sum, &mut correction, value);
                }
                Ok(())
            })?;
            sums[0] = sum + correction;
            corrections[0] = correction;
            return Ok(());
        }
        chunked(meter, context, values.len(), 1, |range| {
            for vertex in range {
                let value = values[vertex];
                if !value.is_finite() {
                    return Err(CmgError::NonFiniteMatrixValue {
                        row: vertex,
                        column: 0,
                        value,
                    });
                }
                let label = self.labels[vertex];
                neumaier_add(&mut sums[label], &mut corrections[label], value);
            }
            Ok(())
        })?;
        chunked(meter, context, self.count(), 1, |range| {
            for component in range {
                sums[component] += corrections[component];
            }
            Ok(())
        })
    }
}

#[cfg(feature = "parallel")]
fn should_parallel_centering(length: usize, team: Team<'_>) -> bool {
    let options = team.options();
    let parallel_floor = options
        .min_parallel_len
        .max(options.reduction_chunk_size.saturating_mul(8));
    length >= parallel_floor
}

/// One-component centering with fixed-chunk team sums. Lanes poll the meter's
/// query; the coordinator polls after each join, before values change.
#[cfg(feature = "parallel")]
fn center_single_component<M: Meter>(
    values: &mut [f64],
    sizes: &[usize],
    means: &mut [f64],
    team: Team<'_>,
    meter: &mut M,
) -> Result<(), CmgError> {
    let Some(&size) = sizes.first() else {
        return Ok(());
    };
    let chunk_size = team.options().reduction_chunk_size;
    let sum = fixed_chunk_sum(values, chunk_size, team, meter.query());
    meter.joined("CMG fixed-chunk centering sums")?;
    let mean = sum? / size as f64;
    means[0] = mean;
    let rows =
        crate::preconditioner::rows_parallel(team, meter.query(), values, 1, |_, _, value| {
            value[0] -= mean;
        });
    meter.joined("CMG fixed-chunk centering")?;
    rows
}

/// Fixed-chunk compensated sum: chunk partials combined by the team's fixed
/// balanced tree. Each leaf counts its chunk in segments through its lane's
/// poll; a failing leaf stops its lane and no later leaf of that lane runs.
/// The joined caller reports cancellation; otherwise the leftmost non-finite
/// value is reported.
#[cfg(feature = "parallel")]
fn fixed_chunk_sum(
    values: &[f64],
    chunk_size: usize,
    team: Team<'_>,
    query: Option<crate::Cancellation<'_>>,
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
            let mut sum = 0.0;
            let mut correction = 0.0;
            chunked(
                poll,
                "CMG fixed-chunk centering sums",
                end - start,
                1,
                |range| {
                    for (offset, &value) in values[start + range.start..start + range.end]
                        .iter()
                        .enumerate()
                    {
                        #[cfg(test)]
                        crate::execution::visits::visit(values.as_ptr());
                        if !value.is_finite() {
                            return Err(CmgError::NonFiniteMatrixValue {
                                row: start + range.start + offset,
                                column: 0,
                                value,
                            });
                        }
                        neumaier_add(&mut sum, &mut correction, value);
                    }
                    Ok(())
                },
            )?;
            Ok(sum + correction)
        },
        |mut sum, right| {
            let mut correction = 0.0;
            neumaier_add(&mut sum, &mut correction, right);
            sum + correction
        },
    )
}

fn neumaier_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let next = *sum + value;
    *correction += if sum.abs() >= value.abs() {
        (*sum - next) + value
    } else {
        (value - next) + *sum
    };
    *sum = next;
}

fn find_root<M: Meter>(
    parent: &mut [usize],
    vertex: usize,
    meter: &mut M,
) -> Result<usize, CmgError> {
    let mut root = vertex;
    while parent[root] != root {
        meter.work("component root walk", 8)?;
        root = parent[root];
    }
    let mut current = vertex;
    while parent[current] != current {
        meter.work("component root compression", 16)?;
        let next = parent[current];
        parent[current] = root;
        current = next;
    }
    Ok(root)
}

#[cfg(all(test, feature = "experimental-components"))]
mod contiguous_tests {
    use super::{Components, Laplacian};
    use crate::cancel::Unmetered;

    fn bits(values: &[f64]) -> Vec<u64> {
        values.iter().map(|value| value.to_bits()).collect()
    }

    #[test]
    fn contiguous_centering_matches_labeled_path_with_reused_workspace() {
        for graph in [
            Laplacian::from_edges(0, []).unwrap(),
            Laplacian::from_edges(9, []).unwrap(),
            Laplacian::from_edges(9, (0..8).map(|v| (v, v + 1, 1.0))).unwrap(),
            Laplacian::from_edges(15, [(0, 1, 1.0), (1, 2, 1.0), (4, 5, 1.0), (7, 8, 1.0)])
                .unwrap(),
        ] {
            let components = Components::from_laplacian(&graph);
            assert!(components.contiguous);
            let prepared = Components::try_from_endpoints(
                graph.vertex_count(),
                graph.edges().iter().map(|e| (e.u(), e.v())),
            )
            .unwrap();
            assert_eq!(components, prepared);
            let mut labeled = components.clone();
            labeled.contiguous = false;
            let mut workspace = components.workspace();
            let mut reference_workspace = labeled.workspace();
            let values = [1e150, 1.0, -1e150, -0.0, 0.0, 1e-150, -1e-150];
            for offset in 0..values.len() {
                let mut actual: Vec<_> = (0..graph.vertex_count())
                    .map(|v| values[(v + offset) % values.len()])
                    .collect();
                let mut expected = actual.clone();
                workspace.sums.fill(f64::NAN);
                workspace.corrections.fill(f64::NAN);
                workspace.means.fill(f64::NAN);
                components
                    .center_in_place_with_workspace(&mut actual, &mut workspace, &mut Unmetered)
                    .unwrap();
                labeled
                    .center_in_place_with_workspace(
                        &mut expected,
                        &mut reference_workspace,
                        &mut Unmetered,
                    )
                    .unwrap();
                assert_eq!(bits(&actual), bits(&expected));
                assert_eq!(bits(&workspace.sums), bits(&reference_workspace.sums));
                assert_eq!(
                    bits(&workspace.corrections),
                    bits(&reference_workspace.corrections)
                );
                assert_eq!(bits(&workspace.means), bits(&reference_workspace.means));
            }
        }
        let scattered = Laplacian::from_edges(4, [(0, 2, 1.0), (1, 3, 1.0)]).unwrap();
        assert!(!Components::from_laplacian(&scattered).contiguous);
        assert!(
            !Components::try_from_endpoints(4, [(0, 2), (1, 3)])
                .unwrap()
                .contiguous
        );
    }

    #[test]
    fn contiguous_centering_preserves_input_on_nonfinite_failure() {
        let graph = Laplacian::from_edges(6, [(0, 1, 1.0), (2, 3, 1.0)]).unwrap();
        let components = Components::from_laplacian(&graph);
        let mut workspace = components.workspace();
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut input = vec![1.0, 3.0, 5.0, bad, bad, 0.0];
            let before = bits(&input);
            let error = components
                .center_in_place_with_workspace(&mut input, &mut workspace, &mut Unmetered)
                .unwrap_err();
            match error {
                crate::CmgError::NonFiniteMatrixValue { row, column, value } => {
                    assert_eq!((row, column), (3, 0));
                    assert_eq!(value.to_bits(), bad.to_bits());
                }
                other => panic!("unexpected error: {other:?}"),
            }
            assert_eq!(bits(&input), before);
            input[3] = 7.0;
            input[4] = 0.0;
            components
                .center_in_place_with_workspace(&mut input, &mut workspace, &mut Unmetered)
                .unwrap();
            assert_eq!(input, [-1.0, 1.0, -1.0, 1.0, 0.0, 0.0]);
        }
    }
}

#[cfg(all(test, feature = "experimental-components"))]
mod centered_dot_tests {
    use super::{ComponentWorkspace, Components, Laplacian};
    use crate::cancel::Unmetered;
    use crate::{CmgError, graph::compensated_sum};

    fn bits(values: &[f64]) -> Vec<u64> {
        values.iter().map(|value| value.to_bits()).collect()
    }

    #[test]
    fn centered_dot_matches_separate_passes_across_layouts_and_scales() {
        for graph in [
            Laplacian::from_edges(0, []).unwrap(),
            Laplacian::from_edges(9, []).unwrap(),
            Laplacian::from_edges(9, (0..8).map(|v| (v, v + 1, 1.0))).unwrap(),
            Laplacian::from_edges(15, [(0, 1, 1.0), (1, 2, 1.0), (4, 5, 1.0)]).unwrap(),
            Laplacian::from_edges(15, [(0, 4, 1.0), (1, 5, 1.0), (4, 8, 1.0)]).unwrap(),
        ] {
            let components = Components::try_from_endpoints(
                graph.vertex_count(),
                graph.edges().iter().map(|e| (e.u(), e.v())),
            )
            .unwrap();
            let mut workspace = components.workspace();
            let mut reference_workspace = components.workspace();
            let values = [1e150, 1.0, -1e150, -0.0, 0.0, 1e-150, -1e-150];
            for scale in [0.0, 1e-150, 1.0, 1e150] {
                for offset in 0..values.len() {
                    let mut actual: Vec<_> = (0..graph.vertex_count())
                        .map(|v| values[(v + offset) % values.len()])
                        .collect();
                    let mut expected = actual.clone();
                    let left: Vec<_> = (0..graph.vertex_count())
                        .map(|v| scale * (((v * 17 + 3) % 19) as f64 - 9.0))
                        .collect();
                    workspace.sums.fill(f64::NAN);
                    workspace.corrections.fill(f64::NAN);
                    workspace.means.fill(f64::NAN);
                    components
                        .center_in_place_with_workspace(
                            &mut expected,
                            &mut reference_workspace,
                            &mut Unmetered,
                        )
                        .unwrap();
                    let reference = compensated_sum(left.iter().zip(&expected).map(|(a, b)| a * b));
                    let actual_dot = components
                        .center_and_dot_with_workspace(
                            &mut actual,
                            &left,
                            &mut workspace,
                            &mut Unmetered,
                        )
                        .unwrap();
                    assert_eq!(bits(&actual), bits(&expected));
                    assert_eq!(actual_dot.to_bits(), reference.to_bits());
                    assert_eq!(bits(&workspace.sums), bits(&reference_workspace.sums));
                    assert_eq!(
                        bits(&workspace.corrections),
                        bits(&reference_workspace.corrections)
                    );
                    assert_eq!(bits(&workspace.means), bits(&reference_workspace.means));
                }
            }
        }
    }

    #[test]
    fn centered_dot_preserves_input_on_validation_errors_and_reuses_workspace() {
        for edges in [[(0, 1, 1.0), (2, 3, 1.0)], [(0, 2, 1.0), (1, 3, 1.0)]] {
            let components = Components::from_laplacian(&Laplacian::from_edges(6, edges).unwrap());
            let mut workspace = components.workspace();
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let mut values = [1.0, 3.0, 5.0, bad, bad, 0.0];
                let before = bits(&values);
                let error = components
                    .center_and_dot_with_workspace(
                        &mut values,
                        &[1.0; 6],
                        &mut workspace,
                        &mut Unmetered,
                    )
                    .unwrap_err();
                match error {
                    CmgError::NonFiniteMatrixValue { row, column, value } => {
                        assert_eq!((row, column), (3, 0));
                        assert_eq!(value.to_bits(), bad.to_bits());
                    }
                    other => panic!("unexpected error: {other:?}"),
                }
                assert_eq!(bits(&values), before);
                values[3] = 7.0;
                values[4] = 0.0;
                let mut expected = values;
                components.center_in_place(&mut expected).unwrap();
                let actual = components
                    .center_and_dot_with_workspace(
                        &mut values,
                        &[1.0; 6],
                        &mut workspace,
                        &mut Unmetered,
                    )
                    .unwrap();
                assert_eq!(bits(&values), bits(&expected));
                assert_eq!(actual.to_bits(), compensated_sum(expected).to_bits());
            }
            let mut values = [1.0; 6];
            assert!(
                components
                    .center_and_dot_with_workspace(
                        &mut values,
                        &[1.0; 5],
                        &mut workspace,
                        &mut Unmetered
                    )
                    .is_err()
            );
            assert_eq!(values, [1.0; 6]);
            assert!(
                components
                    .center_and_dot_with_workspace(
                        &mut values,
                        &[1.0; 6],
                        &mut ComponentWorkspace::new(0),
                        &mut Unmetered
                    )
                    .is_err()
            );
            assert_eq!(values, [1.0; 6]);
        }
    }
}

#[cfg(all(test, feature = "parallel"))]
mod deterministic_parallel_centering_tests {
    use super::Components;
    use crate::cancel::Unmetered;
    use crate::{CmgError, Laplacian, ParallelExecutor, ParallelOptions};

    fn path(vertices: usize) -> Laplacian {
        Laplacian::from_edges(
            vertices,
            (0..vertices - 1).map(|vertex| (vertex, vertex + 1, 1.0)),
        )
        .unwrap()
    }

    fn executor(threads: usize) -> ParallelExecutor {
        ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            reduction_chunk_size: 16,
            ..ParallelOptions::default()
        })
        .unwrap()
    }

    #[test]
    fn fixed_chunk_centering_is_thread_count_invariant() {
        let components = Components::from_laplacian(&path(513));
        let input: Vec<f64> = (0..513)
            .map(|index| ((index * 29 + 11) % 137) as f64 / 17.0 - 4.0)
            .collect();
        let mut serial = input.clone();
        components.center_in_place(&mut serial).unwrap();
        let mut reference = None;
        for threads in [2, 3, 4, 8] {
            let mut values = input.clone();
            let mut workspace = components.workspace();
            components
                .center_with_team(
                    &mut values,
                    &mut workspace,
                    Some(executor(threads).team()),
                    &mut Unmetered,
                )
                .unwrap();
            let bits: Vec<u64> = values.iter().map(|value| value.to_bits()).collect();
            match &reference {
                Some(expected) => assert_eq!(expected, &bits),
                None => reference = Some(bits),
            }
            for (&expected, &actual) in serial.iter().zip(&values) {
                assert!((expected - actual).abs() <= 3.0e-14 * (1.0 + expected.abs()));
            }
        }
    }

    #[test]
    fn parallel_centering_reports_the_first_nonfinite_vertex() {
        let components = Components::from_laplacian(&path(513));
        for threads in [2, 3, 8] {
            let mut values = vec![1.0; 513];
            values[300] = f64::INFINITY;
            values[130] = f64::NAN;
            let mut workspace = components.workspace();
            let error = components
                .center_with_team(
                    &mut values,
                    &mut workspace,
                    Some(executor(threads).team()),
                    &mut Unmetered,
                )
                .unwrap_err();
            assert!(matches!(
                error,
                CmgError::NonFiniteMatrixValue { row: 130, .. }
            ));
        }
    }
}

#[cfg(all(test, feature = "experimental-components"))]
mod controlled_component_tests {
    use super::*;
    use crate::cancel::{CancelMeter, StopAt};

    #[test]
    fn controlled_components_and_centering_match_prepared_paths() {
        for graph in [
            Laplacian::from_edges(0, []).unwrap(),
            Laplacian::from_edges(8, []).unwrap(),
            Laplacian::from_edges(8, (0..7).map(|v| (v, v + 1, 1.0))).unwrap(),
            Laplacian::from_edges(8, [(0, 2, 1.0), (1, 3, 1.0), (2, 4, 2.0)]).unwrap(),
        ] {
            let never = || false;
            let mut meter = CancelMeter::every_operation(&never);
            let controlled = Components::from_endpoints(
                graph.vertex_count(),
                graph.edges().iter().rev().map(|edge| (edge.u(), edge.v())),
                &mut meter,
            )
            .unwrap();
            let expected = Components::try_from_endpoints(
                graph.vertex_count(),
                graph.edges().iter().map(|edge| (edge.u(), edge.v())),
            )
            .unwrap();
            assert_eq!(controlled, expected);
            let actual_plan = CenteringPlan::from_laplacian(&graph, &mut meter).unwrap();
            let expected_plan = CenteringPlan::from_laplacian(&graph, &mut Unmetered).unwrap();
            assert_eq!(actual_plan, expected_plan);
        }
    }

    #[test]
    fn controlled_component_root_walk_can_be_cancelled_inside_a_chain() {
        let endpoints = (0..8)
            .rev()
            .map(|v| (v, v + 1))
            .chain([(8, 0)])
            .collect::<Vec<_>>();
        let count = StopAt::never();
        let query = || count.query();
        Components::from_endpoints(
            9,
            endpoints.iter().copied(),
            &mut CancelMeter::every_operation(&query),
        )
        .unwrap();
        let mut root_walk = 0;
        for stop in 0..count.polls() {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            let Err(CmgError::Cancelled { phase }) = Components::from_endpoints(
                9,
                endpoints.iter().copied(),
                &mut CancelMeter::every_operation(&query),
            ) else {
                panic!("cancellation at poll {stop} must stop component discovery");
            };
            assert_eq!(cancel.polls(), stop + 1);
            root_walk += usize::from(phase == "component root walk");
        }
        assert!(root_walk >= 8);
    }
}

#[cfg(all(test, feature = "parallel"))]
mod fixed_chunk_sum_tests {
    use super::*;
    use crate::{ParallelExecutor, ParallelOptions};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn huge_centering_leaf_polls_inside_and_keeps_leftmost_error() {
        let len = 1 << 20;
        let values: Vec<f64> = (0..len).map(|i| ((i * 11 % 103) as f64).cos()).collect();
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
        let whole = fixed_chunk_sum(&values, len, team, Some(&count)).unwrap();
        let (mut sum, mut correction) = (0.0, 0.0);
        for &value in &values {
            neumaier_add(&mut sum, &mut correction, value);
        }
        assert_eq!(whole.to_bits(), (sum + correction).to_bits());
        assert_eq!(
            polls.load(Ordering::SeqCst),
            len / crate::cancel::POLL_UNITS as usize
        );
        let reference = fixed_chunk_sum(&values, 4096, team, None)
            .unwrap()
            .to_bits();
        for threads in [1, 2, 3] {
            let executor = ParallelExecutor::new(ParallelOptions {
                threads,
                ..ParallelOptions::default()
            })
            .unwrap();
            assert_eq!(
                fixed_chunk_sum(&values, 4096, executor.team(), None)
                    .unwrap()
                    .to_bits(),
                reference
            );
        }
        let mut bad = values;
        bad[70_000] = f64::NAN;
        bad[900_000] = f64::INFINITY;
        assert!(matches!(
            fixed_chunk_sum(&bad, 4096, team, None),
            Err(CmgError::NonFiniteMatrixValue { row: 70_000, .. })
        ));
    }

    /// Many centering leaves on a ceiling of two lanes over a four-worker pool:
    /// after a lane observes cancellation, no lane starts another leaf, so the
    /// entries visited after the observation stay within one poll interval
    /// per lane; the uncancelled sum then visits every entry and is exact.
    #[test]
    fn many_centering_leaves_stop_after_cancellation_and_rerun_exactly() {
        use crate::execution::{gauge, visits};
        let _witness = gauge::WITNESS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let len = 1 << 21;
        let chunk = 256;
        let values: Vec<f64> = (0..len).map(|i| ((i * 11 % 103) as f64).cos()).collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let wide = Team::new(&pool, 2, ParallelOptions::default()).unwrap();
        let interval = crate::cancel::POLL_UNITS as usize;
        let reference = fixed_chunk_sum(&values, chunk, wide, None).unwrap();
        // The one-lane view runs the same tree on its calling lane: equal bits
        // and the same per-lane cancellation bounds.
        for team in [wide, wide.single_lane()] {
            for stop in [0, 2] {
                let calls = AtomicUsize::new(0);
                let at = AtomicUsize::new(usize::MAX);
                let query = || {
                    if calls.fetch_add(1, Ordering::SeqCst) < stop {
                        return false;
                    }
                    let _ = at.compare_exchange(
                        usize::MAX,
                        visits::count(),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    );
                    true
                };
                visits::watch(&values);
                let result = fixed_chunk_sum(&values, chunk, team, Some(&query));
                let total = visits::count();
                visits::unwatch();
                assert!(matches!(result, Err(CmgError::Cancelled { .. })));
                let at = at.load(Ordering::SeqCst);
                assert!(at <= total);
                assert!(total - at <= team.worker_ceiling() * (interval + 1024));
                assert!(total <= (stop + 2 * team.worker_ceiling()) * (interval + 1024));
                assert!(total < len / 4, "{total} of {len} entries visited");
            }
            visits::watch(&values);
            let again = fixed_chunk_sum(&values, chunk, team, None).unwrap();
            assert_eq!(visits::count(), len);
            visits::unwatch();
            assert_eq!(again.to_bits(), reference.to_bits());
        }
    }
}
