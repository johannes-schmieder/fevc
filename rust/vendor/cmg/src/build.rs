//! Execution and resource context shared by construction entry points.

use crate::Cancellation;
#[cfg(feature = "parallel")]
use crate::Team;

/// Execution and resource context for one construction.
///
/// The default builds serially, without polling and without a memory budget.
/// Results never depend on the context: a team only selects the existing
/// deterministic parallel kernels above their fixed thresholds, a cancellation
/// query only adds polls between bounded operations and before allocations,
/// and a budget only admits requested layouts before the first allocation.
#[derive(Clone, Copy, Default)]
pub struct BuildContext<'a> {
    /// Borrowed team for the parallel kernels; `None` runs serially. CMG never
    /// creates a pool during construction.
    #[cfg(feature = "parallel")]
    pub team: Option<Team<'a>>,
    /// Sticky, allocation-free cancellation query. A true query returns
    /// [`crate::CmgError::Cancelled`] and publishes nothing.
    pub cancel: Option<Cancellation<'a>>,
    /// CMG-owned requested-layout budget admitted before allocation.
    pub budget_bytes: Option<usize>,
}

impl core::fmt::Debug for BuildContext<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug = formatter.debug_struct("BuildContext");
        #[cfg(feature = "parallel")]
        debug.field("team", &self.team);
        debug
            .field("cancel", &self.cancel.is_some())
            .field("budget_bytes", &self.budget_bytes)
            .finish()
    }
}

impl<'a> BuildContext<'a> {
    /// The team, or `None` without the parallel feature.
    pub(crate) fn team(&self) -> Option<crate::execution::Team<'a>> {
        #[cfg(feature = "parallel")]
        {
            self.team
        }
        #[cfg(not(feature = "parallel"))]
        {
            None
        }
    }
}

/// Hierarchy and component options for [`crate::CmgPreconditioner::build_with`].
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildOptions {
    /// Hierarchy stopping rules, direct threshold and smoothing options.
    pub hierarchy: crate::CmgOptions,
    /// Opt-in component setup changes; the default is the ordinary builder.
    #[cfg(feature = "experimental-components")]
    pub components: crate::ComponentBuildOptions,
}

impl From<crate::CmgOptions> for BuildOptions {
    fn from(hierarchy: crate::CmgOptions) -> Self {
        Self {
            hierarchy,
            #[cfg(feature = "experimental-components")]
            components: crate::ComponentBuildOptions::default(),
        }
    }
}

impl BuildOptions {
    pub(crate) fn factor_terminal_components(&self) -> bool {
        #[cfg(feature = "experimental-components")]
        {
            self.components.factor_terminal_components
        }
        #[cfg(not(feature = "experimental-components"))]
        {
            false
        }
    }

    pub(crate) fn level_policy(&self, reserve_levels: bool) -> crate::hierarchy::LevelPolicy {
        crate::hierarchy::LevelPolicy {
            #[cfg(feature = "experimental-components")]
            prune_coarse_isolates: self.components.prune_coarse_isolates,
            #[cfg(feature = "experimental-components")]
            preserve_unpruned_stopping: self.components.prune_coarse_isolates
                && self.components.preserve_unpruned_stopping,
            reserve_levels,
        }
    }
}
