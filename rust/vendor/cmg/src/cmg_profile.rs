//! Exclusive attribution inside the stationary recursive CMG cycle.

/// One non-overlapping operation inside a recursive CMG application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum CmgApplyPhase {
    /// Initial diagonal application on the first sweep.
    Initialization,
    /// A smoothing matvec followed by its Jacobi update.
    Smoothing,
    /// The matvec used to form a restricted residual.
    ResidualMatvec,
    /// Residual subtraction and restriction to the child.
    Restriction,
    /// Removal of coarse component null-space drift.
    Centering,
    /// Addition of the child's prolonged correction.
    Prolongation,
    /// A direct or diagonal terminal application.
    Terminal,
}

/// Observer of the one cycle algorithm; `()` records nothing and compiles
/// away, a cycle profile records exclusive phase times.
pub(crate) trait CycleRecorder {
    type Stamp;
    fn start() -> Self::Stamp;
    fn finish(&mut self, level: usize, phase: CmgApplyPhase, start: Self::Stamp);
    fn enter(&mut self, level: usize);
    fn iteration(&mut self, level: usize);
}

impl CycleRecorder for () {
    type Stamp = ();
    #[inline(always)]
    fn start() {}
    #[inline(always)]
    fn finish(&mut self, _: usize, _: CmgApplyPhase, (): ()) {}
    #[inline(always)]
    fn enter(&mut self, _: usize) {}
    #[inline(always)]
    fn iteration(&mut self, _: usize) {}
}

#[cfg(feature = "cycle-profiling")]
mod timed {
    use super::{CmgApplyPhase, CycleRecorder};
    use std::time::Instant;

    impl CmgApplyPhase {
        /// All phases in their report order.
        pub const ALL: [Self; 7] = [
            Self::Initialization,
            Self::Smoothing,
            Self::ResidualMatvec,
            Self::Restriction,
            Self::Centering,
            Self::Prolongation,
            Self::Terminal,
        ];

        /// Stable machine-readable phase name.
        #[must_use]
        pub const fn name(self) -> &'static str {
            match self {
                Self::Initialization => "initialization",
                Self::Smoothing => "smoothing",
                Self::ResidualMatvec => "residual_matvec",
                Self::Restriction => "restriction",
                Self::Centering => "centering",
                Self::Prolongation => "prolongation",
                Self::Terminal => "terminal",
            }
        }
    }

    /// Exclusive phase times and actual recursive visits for one hierarchy level.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct CmgApplyLevelProfile {
        nanoseconds: [u128; 7],
        calls: [usize; 7],
        visits: usize,
        iterations: usize,
    }

    impl CmgApplyLevelProfile {
        /// Return exclusive nanoseconds for one phase at this level.
        #[must_use]
        pub fn nanoseconds(&self, phase: CmgApplyPhase) -> u128 {
            self.nanoseconds[phase as usize]
        }
        /// Return measured operation invocations for one phase.
        #[must_use]
        pub fn calls(&self, phase: CmgApplyPhase) -> usize {
            self.calls[phase as usize]
        }
        /// Return actual entries into the recursive application at this level.
        #[must_use]
        pub const fn visits(&self) -> usize {
            self.visits
        }
        /// Return completed nonterminal stationary iterations.
        #[must_use]
        pub const fn iterations(&self) -> usize {
            self.iterations
        }
    }

    /// Accumulated exclusive timings inside one or more CMG applications.
    ///
    /// Parent timings exclude recursive child applications. Their sum can be
    /// compared with the enclosing PCG preconditioner timer without double counting.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct CmgApplyProfile {
        levels: Vec<CmgApplyLevelProfile>,
    }

    impl CmgApplyProfile {
        pub(crate) fn new(level_count: usize) -> Self {
            Self {
                levels: vec![CmgApplyLevelProfile::default(); level_count],
            }
        }
        /// Return profiles in hierarchy-level order.
        #[must_use]
        pub fn levels(&self) -> &[CmgApplyLevelProfile] {
            &self.levels
        }
        /// Return the sum of non-overlapping operation timings at all levels.
        #[must_use]
        pub fn attributed_nanoseconds(&self) -> u128 {
            self.levels.iter().flat_map(|l| l.nanoseconds).sum()
        }
    }

    impl CycleRecorder for CmgApplyProfile {
        type Stamp = Instant;
        fn start() -> Instant {
            Instant::now()
        }
        fn finish(&mut self, level: usize, phase: CmgApplyPhase, start: Instant) {
            let elapsed = start.elapsed().as_nanos();
            self.levels[level].nanoseconds[phase as usize] += elapsed;
            self.levels[level].calls[phase as usize] += 1;
        }
        fn enter(&mut self, level: usize) {
            self.levels[level].visits += 1;
        }
        fn iteration(&mut self, level: usize) {
            self.levels[level].iterations += 1;
        }
    }
}

#[cfg(feature = "cycle-profiling")]
pub use timed::{CmgApplyLevelProfile, CmgApplyProfile};
