// SPDX-License-Identifier: GPL-3.0-only

//! Build-time-only diagnostics. Disabled guards are zero-sized no-ops; timing
//! never enters an execution plan, a numerical decision or a public receipt.

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Command,
    Leverage,
    LeverageRhs,
    LeverageStatistics,
    Target,
    TargetRng,
    TargetRhs,
    TargetStatistics,
    ModelSolve,
    GenericCertification,
    DirectSolve,
    Projection,
    Component,
    Exact,
    ComponentGram,
    ComponentSpectrum,
    ComponentPrepare,
    ComponentStatistics,
    ExactInformation,
    ExactInverse,
    ExactCorrection,
    ProjectionStatistics,
}

#[cfg(not(feature = "pipeline-profile"))]
pub(crate) struct Scope;

#[cfg(not(feature = "pipeline-profile"))]
impl Scope {
    #[inline(always)]
    pub(crate) fn new(_phase: Phase) -> Self {
        Self
    }
}

#[cfg(not(feature = "pipeline-profile"))]
impl Drop for Scope {
    #[inline(always)]
    fn drop(&mut self) {}
}

#[cfg(feature = "pipeline-profile")]
pub(crate) use enabled::Scope;

#[cfg(feature = "pipeline-profile")]
mod enabled {
    use super::Phase;
    use std::cell::RefCell;
    use std::time::Instant;

    const NAMES: [&str; 22] = [
        "command",
        "leverage",
        "leverage_rhs",
        "leverage_statistics",
        "target",
        "target_rng",
        "target_rhs",
        "target_statistics",
        "model_solve",
        "generic_certification",
        "direct_solve",
        "projection",
        "component",
        "exact",
        "component_gram",
        "component_spectrum",
        "component_prepare",
        "component_statistics",
        "exact_information",
        "exact_inverse",
        "exact_correction",
        "projection_statistics",
    ];

    #[derive(Clone, Copy, Default)]
    struct Stats {
        calls: u64,
        inclusive: u128,
        exclusive: u128,
    }

    #[derive(Default)]
    struct State {
        stats: [Stats; 22],
        children: Vec<u128>,
    }

    thread_local! {
        static STATE: RefCell<State> = RefCell::new(State::default());
    }

    pub(crate) struct Scope {
        phase: Phase,
        start: Instant,
        depth: usize,
        // A diagnostic scope belongs to its originating thread.
        _thread: core::marker::PhantomData<std::rc::Rc<()>>,
    }

    impl Scope {
        pub(crate) fn new(phase: Phase) -> Self {
            let depth = STATE.with_borrow_mut(|state| {
                if state.children.is_empty() {
                    state.stats.fill(Stats::default());
                }
                let depth = state.children.len();
                state.children.push(0);
                depth
            });
            Self {
                phase,
                start: Instant::now(),
                depth,
                _thread: core::marker::PhantomData,
            }
        }
    }

    impl Drop for Scope {
        fn drop(&mut self) {
            let elapsed = self.start.elapsed().as_nanos();
            STATE.with_borrow_mut(|state| {
                assert_eq!(state.children.len(), self.depth + 1);
                let children = state.children.pop().expect("profile scope");
                let stats = &mut state.stats[self.phase as usize];
                stats.calls += 1;
                stats.inclusive += elapsed;
                stats.exclusive += elapsed.saturating_sub(children);
                if let Some(parent) = state.children.last_mut() {
                    *parent += elapsed;
                } else {
                    for (name, stats) in NAMES.into_iter().zip(state.stats) {
                        if stats.calls != 0 {
                            eprintln!(
                                "FEVC_PIPELINE_PROFILE_V1\t{name}\t{}\t{}\t{}",
                                stats.calls, stats.inclusive, stats.exclusive
                            );
                        }
                    }
                }
            });
        }
    }

    #[test]
    fn nested_exclusive_times_reconcile() {
        let outer = Scope::new(Phase::Command);
        let inner = Scope::new(Phase::Target);
        drop(inner);
        drop(outer);
        STATE.with_borrow(|state| {
            let total = state.stats[Phase::Command as usize];
            assert_eq!(total.calls, 1);
            assert_eq!(
                total.inclusive,
                state.stats.iter().map(|v| v.exclusive).sum()
            );
            assert!(state.children.is_empty());
        });
    }
}
