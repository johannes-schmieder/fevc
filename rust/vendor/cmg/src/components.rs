//! Deterministic connected components and Laplacian null-space operations.

#[cfg(feature = "parallel")]
use crate::ParallelExecutor;
use crate::{CmgError, Laplacian, ValidationOptions};
#[cfg(feature = "parallel")]
use rayon::prelude::*;

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
    fn new(component_count: usize) -> Self {
        Self {
            sums: vec![0.0; component_count],
            corrections: vec![0.0; component_count],
            means: vec![0.0; component_count],
        }
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
    pub(crate) fn from_laplacian(graph: &Laplacian) -> Self {
        let Components { labels, sizes } = Components::from_laplacian(graph);
        let vertex_count = labels.len();
        let component_count = sizes.len();
        let labels = if component_count <= 1 {
            CenteringLabels::Single
        } else if component_count <= u32::MAX as usize {
            CenteringLabels::Compact(labels.into_iter().map(|label| label as u32).collect())
        } else {
            CenteringLabels::Native(labels)
        };
        Self {
            vertex_count,
            labels,
            sizes,
        }
    }

    pub(crate) fn workspace(&self) -> CenteringWorkspace {
        CenteringWorkspace::new(self.sizes.len())
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

    pub(crate) fn center_in_place_with_workspace(
        &self,
        values: &mut [f64],
        workspace: &mut CenteringWorkspace,
    ) -> Result<(), CmgError> {
        if values.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "CenteringPlan::center_in_place",
                self.vertex_count,
                values.len(),
            ));
        }
        workspace.validate(self.sizes.len())?;
        workspace.sums.fill(0.0);
        workspace.corrections.fill(0.0);

        match &self.labels {
            CenteringLabels::Single => {
                if !self.sizes.is_empty() {
                    let mut sum = 0.0;
                    let mut correction = 0.0;
                    for (vertex, value) in values.iter().enumerate() {
                        if !value.is_finite() {
                            return Err(CmgError::NonFiniteMatrixValue {
                                row: vertex,
                                column: 0,
                                value: *value,
                            });
                        }
                        neumaier_add(&mut sum, &mut correction, *value);
                    }
                    workspace.sums[0] = sum;
                    workspace.corrections[0] = correction;
                }
            }
            CenteringLabels::Compact(labels) => {
                for (vertex, (value, label)) in values.iter().zip(labels).enumerate() {
                    if !value.is_finite() {
                        return Err(CmgError::NonFiniteMatrixValue {
                            row: vertex,
                            column: 0,
                            value: *value,
                        });
                    }
                    let label = *label as usize;
                    neumaier_add(
                        &mut workspace.sums[label],
                        &mut workspace.corrections[label],
                        *value,
                    );
                }
            }
            CenteringLabels::Native(labels) => {
                for (vertex, (value, label)) in values.iter().zip(labels).enumerate() {
                    if !value.is_finite() {
                        return Err(CmgError::NonFiniteMatrixValue {
                            row: vertex,
                            column: 0,
                            value: *value,
                        });
                    }
                    neumaier_add(
                        &mut workspace.sums[*label],
                        &mut workspace.corrections[*label],
                        *value,
                    );
                }
            }
        }

        for component in 0..self.sizes.len() {
            workspace.sums[component] += workspace.corrections[component];
            workspace.means[component] = workspace.sums[component] / self.sizes[component] as f64;
        }
        match &self.labels {
            CenteringLabels::Single => {
                if let Some(mean) = workspace.means.first() {
                    for value in values {
                        *value -= *mean;
                    }
                }
            }
            CenteringLabels::Compact(labels) => {
                for (value, label) in values.iter_mut().zip(labels) {
                    *value -= workspace.means[*label as usize];
                }
            }
            CenteringLabels::Native(labels) => {
                for (value, label) in values.iter_mut().zip(labels) {
                    *value -= workspace.means[*label];
                }
            }
        }
        Ok(())
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn center_in_place_with_workspace_and_executor(
        &self,
        values: &mut [f64],
        workspace: &mut CenteringWorkspace,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        if !matches!(self.labels, CenteringLabels::Single)
            || !should_parallel_centering(values.len(), executor)
        {
            return self.center_in_place_with_workspace(values, workspace);
        }
        if values.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "CenteringPlan::center_in_place",
                self.vertex_count,
                values.len(),
            ));
        }
        workspace.validate(self.sizes.len())?;
        center_single_component(values, &self.sizes, &mut workspace.means, executor)
    }
}

/// Connected-component metadata for a weighted graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Components {
    labels: Vec<usize>,
    sizes: Vec<usize>,
}

impl Components {
    /// Compute deterministic component labels.
    ///
    /// Labels are assigned in first-vertex order, so relabeling does not depend
    /// on edge input order.
    #[must_use]
    pub fn from_laplacian(graph: &Laplacian) -> Self {
        let vertex_count = graph.vertex_count();
        let mut parent: Vec<usize> = (0..vertex_count).collect();
        for edge in graph.edges() {
            union_min_root(&mut parent, edge.u(), edge.v());
        }
        for vertex in 0..vertex_count {
            let root = find_root(&mut parent, vertex);
            parent[vertex] = root;
        }

        let mut root_to_label = vec![usize::MAX; vertex_count];
        let mut labels = vec![0; vertex_count];
        let mut sizes = Vec::new();
        for vertex in 0..vertex_count {
            let root = parent[vertex];
            let label = if root_to_label[root] == usize::MAX {
                let next = sizes.len();
                root_to_label[root] = next;
                sizes.push(0);
                next
            } else {
                root_to_label[root]
            };
            labels[vertex] = label;
            sizes[label] += 1;
        }
        Self { labels, sizes }
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
        )?;
        Ok(workspace.sums)
    }

    /// Verify that a right-hand side is numerically compatible with every
    /// component null space.
    pub fn validate_rhs(&self, rhs: &[f64], options: ValidationOptions) -> Result<(), CmgError> {
        let mut workspace = self.workspace();
        self.validate_rhs_with_workspace(rhs, options, &mut workspace)
    }

    pub(crate) fn validate_rhs_with_workspace(
        &self,
        rhs: &[f64],
        options: ValidationOptions,
        workspace: &mut ComponentWorkspace,
    ) -> Result<(), CmgError> {
        let options = options.validate()?;
        workspace.validate(self.count())?;
        self.compatibility_data_into(rhs, "Components::validate_rhs", workspace)?;
        self.validate_component_sums(&workspace.sums, &workspace.scales, options)
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
        self.project_rhs_in_place_with_workspace(rhs, options, &mut workspace)
    }

    pub(crate) fn project_rhs_in_place_with_workspace(
        &self,
        rhs: &mut [f64],
        options: ValidationOptions,
        workspace: &mut ComponentWorkspace,
    ) -> Result<f64, CmgError> {
        let options = options.validate()?;
        workspace.validate(self.count())?;
        self.compatibility_data_into(rhs, "Components::project_rhs_in_place", workspace)?;
        self.validate_component_sums(&workspace.sums, &workspace.scales, options)?;

        for component in 0..self.count() {
            workspace.means[component] = workspace.sums[component] / self.sizes[component] as f64;
        }
        for (value, label) in rhs.iter_mut().zip(&self.labels) {
            *value -= workspace.means[*label];
        }

        self.stable_representatives_into(rhs, &mut workspace.representatives);
        workspace.projection_corrections.fill(0.0);
        for _ in 0..2 {
            self.compensated_sums_into(
                rhs,
                "Components::project_rhs_in_place",
                &mut workspace.sums,
                &mut workspace.corrections,
            )?;
            for component in 0..self.count() {
                let residual_sum = workspace.sums[component];
                rhs[workspace.representatives[component]] -= residual_sum;
                workspace.projection_corrections[component] += residual_sum;
            }
        }

        let projection_scale = workspace
            .means
            .iter()
            .zip(&workspace.projection_corrections)
            .flat_map(|(mean, correction)| [mean.abs(), (*mean + *correction).abs()])
            .fold(0.0, f64::max);
        if projection_scale == 0.0 {
            return Ok(0.0);
        }
        let projection_squared = workspace
            .means
            .iter()
            .zip(&workspace.projection_corrections)
            .zip(&self.sizes)
            .map(|((mean, correction), size)| {
                let regular = *mean / projection_scale;
                let representative = (*mean + *correction) / projection_scale;
                (*size - 1) as f64 * regular * regular + representative * representative
            })
            .sum::<f64>();
        Ok(projection_scale * projection_squared.sqrt())
    }

    /// Subtract the mean within every component in place.
    pub fn center_in_place(&self, values: &mut [f64]) -> Result<(), CmgError> {
        let mut workspace = self.workspace();
        self.center_in_place_with_workspace(values, &mut workspace)
    }

    pub(crate) fn center_in_place_with_workspace(
        &self,
        values: &mut [f64],
        workspace: &mut ComponentWorkspace,
    ) -> Result<(), CmgError> {
        if values.len() != self.labels.len() {
            return Err(CmgError::dimension(
                "Components::center_in_place",
                self.labels.len(),
                values.len(),
            ));
        }
        workspace.validate(self.count())?;
        self.compensated_sums_into(
            values,
            "Components::center_in_place",
            &mut workspace.sums,
            &mut workspace.corrections,
        )?;
        for component in 0..self.count() {
            workspace.means[component] = workspace.sums[component] / self.sizes[component] as f64;
        }
        if self.count() == 1 {
            let mean = workspace.means[0];
            for value in values {
                *value -= mean;
            }
        } else {
            for (value, label) in values.iter_mut().zip(&self.labels) {
                *value -= workspace.means[*label];
            }
        }
        Ok(())
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn center_in_place_with_workspace_and_executor(
        &self,
        values: &mut [f64],
        workspace: &mut ComponentWorkspace,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        if self.count() != 1 || !should_parallel_centering(values.len(), executor) {
            return self.center_in_place_with_workspace(values, workspace);
        }
        if values.len() != self.labels.len() {
            return Err(CmgError::dimension(
                "Components::center_in_place",
                self.labels.len(),
                values.len(),
            ));
        }
        workspace.validate(self.count())?;
        center_single_component(values, &self.sizes, &mut workspace.means, executor)
    }

    fn stable_representatives_into(&self, values: &[f64], representatives: &mut [usize]) {
        representatives.fill(usize::MAX);
        for (vertex, (value, label)) in values.iter().zip(&self.labels).enumerate() {
            let current = representatives[*label];
            if current == usize::MAX
                || value.abs() < values[current].abs()
                || (value.abs() == values[current].abs() && vertex < current)
            {
                representatives[*label] = vertex;
            }
        }
    }

    fn compatibility_data_into(
        &self,
        values: &[f64],
        context: &'static str,
        workspace: &mut ComponentWorkspace,
    ) -> Result<(), CmgError> {
        if values.len() != self.labels.len() {
            return Err(CmgError::dimension(
                context,
                self.labels.len(),
                values.len(),
            ));
        }
        workspace.sums.fill(0.0);
        workspace.corrections.fill(0.0);
        workspace.scales.fill(0.0);
        workspace.scale_corrections.fill(0.0);
        for (vertex, (value, label)) in values.iter().zip(&self.labels).enumerate() {
            if !value.is_finite() {
                return Err(CmgError::NonFiniteMatrixValue {
                    row: vertex,
                    column: 0,
                    value: *value,
                });
            }
            neumaier_add(
                &mut workspace.sums[*label],
                &mut workspace.corrections[*label],
                *value,
            );
            neumaier_add(
                &mut workspace.scales[*label],
                &mut workspace.scale_corrections[*label],
                value.abs(),
            );
        }
        for component in 0..self.count() {
            workspace.sums[component] += workspace.corrections[component];
            workspace.scales[component] += workspace.scale_corrections[component];
        }
        Ok(())
    }

    fn validate_component_sums(
        &self,
        sums: &[f64],
        scales: &[f64],
        options: ValidationOptions,
    ) -> Result<(), CmgError> {
        for component in 0..self.count() {
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
    }

    fn compensated_sums_into(
        &self,
        values: &[f64],
        context: &'static str,
        sums: &mut [f64],
        corrections: &mut [f64],
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
        sums.fill(0.0);
        corrections.fill(0.0);
        if self.count() == 1 {
            let mut sum = 0.0;
            let mut correction = 0.0;
            for (vertex, value) in values.iter().enumerate() {
                if !value.is_finite() {
                    return Err(CmgError::NonFiniteMatrixValue {
                        row: vertex,
                        column: 0,
                        value: *value,
                    });
                }
                neumaier_add(&mut sum, &mut correction, *value);
            }
            sums[0] = sum + correction;
            corrections[0] = correction;
            return Ok(());
        }
        for (vertex, (value, label)) in values.iter().zip(&self.labels).enumerate() {
            if !value.is_finite() {
                return Err(CmgError::NonFiniteMatrixValue {
                    row: vertex,
                    column: 0,
                    value: *value,
                });
            }
            neumaier_add(&mut sums[*label], &mut corrections[*label], *value);
        }
        for component in 0..self.count() {
            sums[component] += corrections[component];
        }
        Ok(())
    }
}

#[cfg(feature = "parallel")]
fn should_parallel_centering(length: usize, executor: &ParallelExecutor) -> bool {
    let options = executor.options();
    let parallel_floor = options
        .min_parallel_len
        .max(options.reduction_chunk_size.saturating_mul(8));
    executor.thread_count() > 1 && length >= parallel_floor
}

#[cfg(feature = "parallel")]
fn center_single_component(
    values: &mut [f64],
    sizes: &[usize],
    means: &mut [f64],
    executor: &ParallelExecutor,
) -> Result<(), CmgError> {
    let Some(&size) = sizes.first() else {
        return Ok(());
    };
    let chunk_size = executor.options().reduction_chunk_size;
    let sum = executor.install(|| fixed_chunk_sum(values, chunk_size))?;
    let mean = sum / size as f64;
    means[0] = mean;
    executor.install(|| values.par_iter_mut().for_each(|value| *value -= mean));
    Ok(())
}

#[cfg(feature = "parallel")]
fn fixed_chunk_sum(values: &[f64], chunk_size: usize) -> Result<f64, CmgError> {
    let chunk_count = values.len().div_ceil(chunk_size);
    if chunk_count == 0 {
        return Ok(0.0);
    }

    fn reduce_range(
        values: &[f64],
        chunk_size: usize,
        first_chunk: usize,
        last_chunk: usize,
    ) -> Result<f64, CmgError> {
        if last_chunk - first_chunk == 1 {
            let start = first_chunk * chunk_size;
            let end = values.len().min(start + chunk_size);
            let mut sum = 0.0;
            let mut correction = 0.0;
            for (offset, &value) in values[start..end].iter().enumerate() {
                if !value.is_finite() {
                    return Err(CmgError::NonFiniteMatrixValue {
                        row: start + offset,
                        column: 0,
                        value,
                    });
                }
                neumaier_add(&mut sum, &mut correction, value);
            }
            return Ok(sum + correction);
        }
        let middle = first_chunk + (last_chunk - first_chunk) / 2;
        let (left, right) = rayon::join(
            || reduce_range(values, chunk_size, first_chunk, middle),
            || reduce_range(values, chunk_size, middle, last_chunk),
        );
        let mut sum = left?;
        let mut correction = 0.0;
        neumaier_add(&mut sum, &mut correction, right?);
        Ok(sum + correction)
    }

    reduce_range(values, chunk_size, 0, chunk_count)
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

fn find_root(parent: &mut [usize], vertex: usize) -> usize {
    let mut root = vertex;
    while parent[root] != root {
        root = parent[root];
    }
    let mut current = vertex;
    while parent[current] != current {
        let next = parent[current];
        parent[current] = root;
        current = next;
    }
    root
}

fn union_min_root(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find_root(parent, left);
    let right_root = find_root(parent, right);
    if left_root == right_root {
        return;
    }
    let (root, child) = if left_root < right_root {
        (left_root, right_root)
    } else {
        (right_root, left_root)
    };
    parent[child] = root;
}

#[cfg(all(test, feature = "parallel"))]
mod deterministic_parallel_centering_tests {
    use super::Components;
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
                .center_in_place_with_workspace_and_executor(
                    &mut values,
                    &mut workspace,
                    &executor(threads),
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
                .center_in_place_with_workspace_and_executor(
                    &mut values,
                    &mut workspace,
                    &executor(threads),
                )
                .unwrap_err();
            assert!(matches!(
                error,
                CmgError::NonFiniteMatrixValue { row: 130, .. }
            ));
        }
    }
}
