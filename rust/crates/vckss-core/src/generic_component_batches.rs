// SPDX-License-Identifier: GPL-3.0-only

//! Execution-only inference intent. The borrowed view never copies retained
//! variances, changes RNG domains, or mutates a prepared generation.
use super::*;
use crate::batch_plan::BatchSelectionReason;
use crate::component_inference::ComponentInferenceOptions;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ComponentBatchPolicy {
    /// Preserve both the declared component width and legacy Gram width.
    #[default]
    Literal,
    Automatic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentBatchReceipt {
    pub policy: ComponentBatchPolicy,
    pub selection_reason: BatchSelectionReason,
    pub declared_component_width: usize,
    pub declared_gram_width: usize,
    pub component_width: usize,
    pub gram_width: usize,
    pub automatic_cap: usize,
    pub permitted_threads: usize,
}

#[derive(Clone, Copy)]
pub(super) struct ComponentExecution<'a> {
    prepared: &'a PreparedComponentInference,
    pub options: ComponentInferenceOptions,
    pub residual_moments: Option<crate::residual_moment_inference::Options>,
}

impl<'a> ComponentExecution<'a> {
    pub fn new(prepared: &'a PreparedComponentInference) -> Self {
        Self {
            prepared,
            options: prepared.options,
            residual_moments: prepared.residual_moments,
        }
    }

    pub fn automatic(self, width: usize) -> Self {
        Self {
            options: ComponentInferenceOptions {
                batch_width: width
                    .min((self.options.probes as usize).max(self.options.spectrum_probes as usize)),
                ..self.options
            },
            residual_moments: self.residual_moments.map(|mut options| {
                options.batch_width = width.min(options.probes);
                options
            }),
            ..self
        }
    }

    pub fn candidates(self, policy: ComponentBatchPolicy, threads: usize) -> Result<Vec<usize>> {
        match policy {
            ComponentBatchPolicy::Literal => diagonal::widths(BatchRequest::Explicit(1), 1, 1),
            ComponentBatchPolicy::Automatic => {
                diagonal::widths(BatchRequest::Auto, self.cap(threads)?, self.probes())
            }
        }
    }

    fn probes(self) -> usize {
        (self.options.probes as usize)
            .max(self.options.spectrum_probes as usize)
            .max(self.residual_moments.map_or(0, |value| value.probes))
    }

    pub fn cap(self, threads: usize) -> Result<usize> {
        Ok(crate::full_cmg_batch_policy::caps(
            self.probes(),
            threads,
            crate::full_cmg_batch_policy::SELECTED_K,
        )?
        .0)
    }

    pub fn select(self, policy: ComponentBatchPolicy, width: usize) -> Self {
        match policy {
            ComponentBatchPolicy::Literal => self,
            ComponentBatchPolicy::Automatic => self.automatic(width),
        }
    }

    pub fn receipt(
        self,
        policy: ComponentBatchPolicy,
        width: usize,
        threads: usize,
        automatic_reason: BatchSelectionReason,
    ) -> Result<ComponentBatchReceipt> {
        let selected = self.select(policy, width);
        Ok(ComponentBatchReceipt {
            policy,
            selection_reason: if policy == ComponentBatchPolicy::Literal {
                BatchSelectionReason::ExplicitWidth
            } else {
                automatic_reason
            },
            declared_component_width: self.prepared.options.batch_width,
            declared_gram_width: self
                .prepared
                .residual_moments
                .map_or(0, |value| value.batch_width),
            component_width: selected.options.batch_width,
            gram_width: selected
                .residual_moments
                .map_or(0, |value| value.batch_width),
            automatic_cap: self.cap(threads)?,
            permitted_threads: threads,
        })
    }
}

impl std::ops::Deref for ComponentExecution<'_> {
    type Target = PreparedComponentInference;
    fn deref(&self) -> &Self::Target {
        self.prepared
    }
}

pub(super) fn capacity(point: usize, policy: ComponentBatchPolicy, width: usize) -> usize {
    match policy {
        ComponentBatchPolicy::Literal => point,
        ComponentBatchPolicy::Automatic => point.max(width),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_policy_caps_are_independent_of_literal_eight_and_legacy_gram_cap() {
        let prepared = PreparedComponentInference {
            schema_version: crate::component_inference::COMPONENT_INFERENCE_SCHEMA_VERSION,
            inference_unit: ComponentInferenceUnit::Observation,
            variance_source: ComponentVarianceSource::Oracle,
            variance: vec![0.04; 5],
            options: ComponentInferenceOptions {
                probes: 129,
                spectrum_probes: 17,
                batch_width: 8,
                ..Default::default()
            },
            structured_options: crate::structured_variance::StructuredVarianceOptions::default(),
            persistent_bytes: 40,
            individual_intervals: false,
            residual_moments: Some(crate::residual_moment_inference::Options {
                probes: 513,
                batch_width: 16,
                ..Default::default()
            }),
            design_only_order: false,
            unified_variance_fit: false,
        };
        let view = ComponentExecution::new(&prepared);
        for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
            let cap = 513.min(32.max(8 * threads));
            let widths = view
                .candidates(ComponentBatchPolicy::Automatic, threads)
                .unwrap();
            assert_eq!(widths[0], 1);
            assert_eq!(*widths.last().unwrap(), cap);
            assert!(widths.windows(2).all(|pair| pair[0] < pair[1]));
            for width in widths {
                let selected = view.automatic(width);
                assert_eq!(selected.options.batch_width, width.min(129));
                assert_eq!(
                    selected.residual_moments.unwrap().batch_width,
                    width.min(513)
                );
                assert_eq!(selected.variance.as_ptr(), prepared.variance.as_ptr());
                assert_eq!(selected.options.seed, prepared.options.seed);
                assert_eq!(
                    selected.options.spectrum_iterations,
                    prepared.options.spectrum_iterations
                );
            }
            let literal = view
                .receipt(
                    ComponentBatchPolicy::Literal,
                    1,
                    threads,
                    BatchSelectionReason::NoBudgetPerformanceChoice,
                )
                .unwrap();
            assert_eq!((literal.component_width, literal.gram_width), (8, 16));
        }
        assert_eq!(view.cap(0).unwrap_err().code, ErrorCode::InvalidInput);
        assert_eq!(
            view.cap(usize::MAX).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
        assert_eq!(prepared.options.batch_width, 8);
        assert_eq!(prepared.residual_moments.unwrap().batch_width, 16);
    }
}
