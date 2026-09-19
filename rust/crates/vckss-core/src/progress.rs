// SPDX-License-Identifier: GPL-3.0-only

//! Optional observational state, scoped to a host call. No host callback,
//! clock, allocation, or scheduling decision belongs in a numerical publisher.

use std::cell::Cell;
use std::sync::Mutex;

pub const SLOTS: usize = 32;
pub const PREPARATION: u32 = 1;
pub const SETUP: u32 = 2;
pub const FIT: u32 = 3;
pub const LEVERAGE: u32 = 4;
pub const TARGETS: u32 = 5;
pub const PROJECTION: u32 = 6;
pub const INFERENCE: u32 = 7;
pub const SPECTRUM: u32 = 8;
pub const GRAM: u32 = 9;
pub const EXACT: u32 = 10;
pub const VALIDATION: u32 = 11;
pub const SPECTRUM_ITERATIONS: u32 = 12;
pub const SAMPLE: u32 = 20;
pub const PRUNING: u32 = 21;
pub const PLAN: u32 = 22;
pub const MEMORY: u32 = 23;
pub const CHOICES: u32 = 24;
pub const STAYERS: u32 = 25;
pub const FALLBACK: u32 = 26;
pub const MEMORY_FLOOR: u32 = 27;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Update {
    pub sequence: u64,
    pub kind: u32,
    pub reserved: u32,
    pub completed: u64,
    pub total: u64,
    pub values: [u64; 8],
}

#[derive(Debug)]
struct State {
    sequence: u64,
    updates: [Update; SLOTS],
}

/// Fixed stack storage owned by the synchronous reporting entrypoint. Scoped
/// numerical coordinators may borrow it; it never survives the host call.
#[derive(Debug)]
pub struct Progress {
    state: Mutex<State>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                sequence: 0,
                updates: [Update::default(); SLOTS],
            }),
        }
    }
}

impl Progress {
    pub fn snapshot(&self) -> [Update; SLOTS] {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).updates
    }

    fn publish(&self, mut update: Update) {
        let Some(index) = usize::try_from(update.kind).ok().filter(|&v| v < SLOTS) else {
            return;
        };
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if update.kind == MEMORY {
            // Preparation may peak before the solve. The host bridge supplies
            // its already-accounted floor; this affects display only.
            let floor = state.updates[MEMORY_FLOOR as usize].values[0];
            update.values[0] = update.values[0].max(floor);
            update.values[1] = update.values[1].max(floor);
            update.values[2] = update.values[1].saturating_sub(update.values[0]);
        }
        state.sequence = state.sequence.saturating_add(1);
        update.sequence = state.sequence;
        state.updates[index] = update;
    }
}

thread_local! {
    static CURRENT: Cell<*const Progress> = const { Cell::new(std::ptr::null()) };
}

/// The borrow cannot escape this closure. Nested scopes restore their parent
/// even on unwind. A coordinator must join its workers before this scope ends.
pub fn with_progress<R>(progress: Option<&Progress>, operation: impl FnOnce() -> R) -> R {
    struct Restore(*const Progress);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|slot| slot.set(self.0));
        }
    }
    let previous = CURRENT.with(|slot| slot.replace(progress.map_or(std::ptr::null(), |p| p)));
    let _restore = Restore(previous);
    operation()
}

pub fn with_current<R>(operation: impl FnOnce(Option<&Progress>) -> R) -> R {
    CURRENT.with(|slot| {
        // SAFETY: only with_progress installs this pointer, for the duration
        // of a synchronous closure borrowing its stack-owned Progress.
        operation(unsafe { slot.get().as_ref() })
    })
}

pub fn enabled() -> bool {
    CURRENT.with(|slot| !slot.get().is_null())
}

pub fn advance(kind: u32, completed: usize, total: usize) {
    with_current(|progress| {
        if let Some(progress) = progress {
            progress.publish(Update {
                kind,
                completed: completed as u64,
                total: total as u64,
                ..Update::default()
            });
        }
    });
}

pub fn stage(kind: u32) {
    advance(kind, 0, 0);
}

pub fn report(kind: u32, values: [u64; 8]) {
    with_current(|progress| {
        if let Some(progress) = progress {
            progress.publish(Update {
                kind,
                values,
                ..Update::default()
            });
        }
    });
}

pub fn report_if_absent(kind: u32, values: [u64; 8]) {
    with_current(|progress| {
        if let Some(progress) = progress {
            if (kind as usize) < SLOTS && progress.snapshot()[kind as usize].sequence == 0 {
                progress.publish(Update {
                    kind,
                    values,
                    ..Update::default()
                });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_nested_scopes_unwind_and_coordinator_borrow() {
        stage(FIT);
        let outer = Progress::default();
        let inner = Progress::default();
        with_progress(Some(&outer), || {
            stage(PREPARATION);
            let _ = std::panic::catch_unwind(|| {
                with_progress(Some(&inner), || {
                    stage(EXACT);
                    panic!("injected");
                })
            });
            with_current(|state| {
                std::thread::scope(|scope| {
                    scope.spawn(move || with_progress(state, || advance(LEVERAGE, 3, 7)));
                })
            });
            stage(VALIDATION);
        });
        assert_eq!(outer.snapshot()[LEVERAGE as usize].completed, 3);
        assert_eq!(outer.snapshot()[EXACT as usize].sequence, 0);
        assert_ne!(inner.snapshot()[EXACT as usize].sequence, 0);
        with_current(|state| assert!(state.is_none()));
    }

    #[test]
    fn coalesces_counts_but_retains_summary_without_a_queue() {
        let state = Progress::default();
        with_progress(Some(&state), || {
            report(PLAN, [7; 8]);
            for i in 0..10_000 {
                advance(LEVERAGE, i, 10_000);
            }
        });
        assert_eq!(state.snapshot()[PLAN as usize].values, [7; 8]);
        assert_eq!(state.snapshot()[LEVERAGE as usize].completed, 9_999);
    }
}
