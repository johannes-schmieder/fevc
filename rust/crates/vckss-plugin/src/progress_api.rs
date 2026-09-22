// SPDX-License-Identifier: GPL-3.0-only

//! Synchronous, additive host reporting scope. Existing estimator ABIs and
//! error transports run unchanged inside the supplied operation.

use std::cell::Cell;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use vckss_core::progress::{self, Progress, Update, SLOTS};

type Display = unsafe extern "C" fn(*mut c_void, *const Update, u64, u32) -> i32;
type Operation = unsafe extern "C" fn(*mut c_void) -> i32;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VckssProgressOptionsV1 {
    pub struct_size: u32,
    pub schema: u32,
    pub level: u32,
    pub reserved: u32,
    pub display: Option<Display>,
    pub context: *mut c_void,
}

#[derive(Clone, Copy, Debug, Default)]
struct Tracker {
    sequence: u64,
    completed: u64,
    total: u64,
    values: [u64; 8],
    last_ms: u64,
    start_ms: u64,
}

impl Tracker {
    fn due(&self, update: Update, now: u64, active: bool, finish: bool) -> bool {
        if update.sequence == 0 {
            return false;
        }
        if self.sequence == 0 {
            return true;
        }
        if update.kind >= 20 {
            // Internal passes may republish unchanged command settings.
            return self.sequence != update.sequence && self.values != update.values;
        }
        if update.completed < self.completed || update.total != self.total {
            return true;
        }
        let changed = update.completed != self.completed || update.total != self.total;
        if changed && update.total > 0 && update.completed == update.total {
            return true;
        }
        if finish && changed {
            return true;
        }
        let bucket = |done: u64, total: u64| {
            if total == 0 {
                0
            } else {
                (u128::from(done) * 10 / u128::from(total)) as u64
            }
        };
        (changed
            && now.saturating_sub(self.last_ms) >= 1_000
            && bucket(update.completed, update.total) > bucket(self.completed, self.total))
            || (active && now.saturating_sub(self.last_ms) >= 30_000)
    }

    fn record(&mut self, update: Update, now: u64) {
        if self.sequence == 0 || update.completed < self.completed || update.total != self.total {
            self.start_ms = now;
        }
        self.sequence = update.sequence;
        self.completed = update.completed;
        self.total = update.total;
        self.values = update.values;
        self.last_ms = now;
    }
}

struct Reporter<'a> {
    options: VckssProgressOptionsV1,
    progress: &'a Progress,
    start: Instant,
    trackers: [Tracker; SLOTS],
    next_check_ms: u64,
    status: i32,
}

impl Reporter<'_> {
    fn drain(&mut self, finish: bool) -> i32 {
        let now = self.start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        self.drain_at(now, finish)
    }

    fn drain_at(&mut self, now: u64, finish: bool) -> i32 {
        if self.status != 0 {
            return self.status;
        }
        if !finish && now < self.next_check_ms {
            return 0;
        }
        self.next_check_ms = now.saturating_add(50);
        let mut updates = self.progress.snapshot();
        let leverage = updates[progress::LEVERAGE as usize];
        let targets = updates[progress::TARGETS as usize];
        let active = updates
            .iter()
            .filter(|u| u.kind < 20)
            .max_by_key(|u| u.sequence)
            .map_or(0, |u| u.kind);
        updates.sort_unstable_by_key(|u| u.sequence);
        for mut update in updates {
            if self.options.schema == 2
                && matches!(update.kind, progress::LEVERAGE | progress::TARGETS)
            {
                // Emit one coherent pair from the snapshot, without replaying
                // coalesced counts or guessing an overall command percentage.
                if update.sequence != leverage.sequence.max(targets.sequence)
                    || ![leverage, targets].iter().any(|u| {
                        self.trackers[u.kind as usize].due(*u, now, u.kind == active, finish)
                    })
                {
                    continue;
                }
                for u in [leverage, targets] {
                    if u.sequence != 0 {
                        self.trackers[u.kind as usize].record(u, now);
                    }
                }
                update.values = [
                    leverage.completed,
                    leverage.total,
                    targets.completed,
                    if targets.sequence == 0 {
                        leverage.total
                    } else {
                        targets.total
                    },
                    targets.sequence,
                    0,
                    0,
                    0,
                ];
                self.display(&update, now);
                if self.status != 0 {
                    break;
                }
                continue;
            }
            let tracker = &mut self.trackers[update.kind as usize];
            if tracker.due(update, now, update.kind == active, finish) {
                tracker.record(update, now);
                let elapsed = if self.options.schema == 2 {
                    now
                } else {
                    now.saturating_sub(tracker.start_ms)
                };
                self.display(&update, elapsed);
                if self.status != 0 {
                    break;
                }
            }
        }
        self.status
    }

    fn display(&mut self, update: &Update, elapsed: u64) {
        // SAFETY: callback/context are borrowed for this synchronous call;
        // Reporter is accessed only on the Stata caller thread.
        self.status = unsafe {
            (self.options.display.expect("validated display"))(
                self.options.context,
                update,
                elapsed,
                self.options.level,
            )
        };
    }
}

thread_local! {
    static REPORTER: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
}

/// Called only by the existing host interruption checker, never by workers.
pub(crate) fn poll() -> i32 {
    REPORTER.with(|slot| {
        if slot.get().is_null() {
            return 0;
        }
        // SAFETY: installed only by report_call for its synchronous lifetime.
        unsafe { (*slot.get().cast::<Reporter<'_>>()).drain(false) }
    })
}

/// Execute an existing host operation with optional progress. Return values
/// are host (Stata) statuses, not engine error codes. No callback is retained.
///
/// # Safety
/// All pointers and callbacks must be valid for this synchronous call. The
/// operation must join every thread borrowing reporting state before return.
#[no_mangle]
pub unsafe extern "C" fn vckss_rust_report_call_v1(
    options: *const VckssProgressOptionsV1,
    operation: Option<Operation>,
    context: *mut c_void,
) -> i32 {
    if options.is_null() || (options as usize) % align_of::<VckssProgressOptionsV1>() != 0 {
        return 198;
    }
    // Read the size before the body so a short version header is rejected
    // without accessing fields outside the supplied buffer.
    if unsafe { options.cast::<u32>().read() } as usize != size_of::<VckssProgressOptionsV1>() {
        return 198;
    }
    // SAFETY: caller provides an aligned readable structure of the declared size.
    let options = unsafe { *options };
    let Some(operation) = operation else {
        return 198;
    };
    if options.struct_size as usize != size_of::<VckssProgressOptionsV1>()
        || !matches!(options.schema, 1 | 2)
        || options.reserved != 0
        || options.level > 2
        || (options.level != 0 && options.display.is_none())
        || (options.level == 0 && (options.display.is_some() || !options.context.is_null()))
        || REPORTER.with(|slot| !slot.get().is_null())
    {
        return 198;
    }
    catch_unwind(AssertUnwindSafe(|| {
        if options.level == 0 {
            // SAFETY: the caller supplies the operation and its context.
            return unsafe { operation(context) };
        }
        let progress = Progress::default();
        let mut reporter = Reporter {
            options,
            progress: &progress,
            start: Instant::now(),
            trackers: [Tracker::default(); SLOTS],
            next_check_ms: 0,
            status: 0,
        };
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                REPORTER.with(|slot| slot.set(std::ptr::null_mut()));
            }
        }
        REPORTER.with(|slot| slot.set((&mut reporter as *mut Reporter<'_>).cast()));
        let _reset = Reset;
        let status = progress::with_progress(Some(&progress), || {
            // SAFETY: the operation cannot outlive this synchronous scope.
            unsafe { operation(context) }
        });
        if status != 0 {
            status
        } else {
            reporter.drain(true)
        }
    }))
    .unwrap_or(498)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestones_throttle_heartbeat_and_completion_without_sleep() {
        let mut tracker = Tracker::default();
        let mut u = Update {
            sequence: 1,
            kind: 4,
            completed: 0,
            total: 33,
            ..Update::default()
        };
        assert!(tracker.due(u, 0, true, false));
        tracker.record(u, 0);
        u.sequence += 1;
        u.completed = 4;
        assert!(!tracker.due(u, 999, true, false));
        assert!(tracker.due(u, 1000, true, false));
        tracker.record(u, 1000);
        assert!(!tracker.due(u, 30999, true, false));
        assert!(tracker.due(u, 31000, true, false));
        assert!(!tracker.due(u, 31000, false, false));
        u.completed = 33;
        assert!(tracker.due(u, 1001, true, false));
        u.total = 0;
        u.completed = 0;
        tracker.record(u, 1001);
        assert!(tracker.due(u, 31001, true, false));
    }

    struct Capture {
        owner: std::thread::ThreadId,
        events: Vec<Update>,
        elapsed: Vec<u64>,
        operation_status: i32,
        display_status: i32,
    }

    #[test]
    fn repeated_summary_values_do_not_interrupt_progress() {
        for kind in [progress::PLAN, progress::MEMORY, progress::CHOICES] {
            let mut tracker = Tracker::default();
            let mut update = Update {
                sequence: 1,
                kind,
                values: [1, 0, 0, 4, 0, 0, 0, 0],
                ..Update::default()
            };
            assert!(tracker.due(update, 0, false, false));
            tracker.record(update, 0);
            update.sequence += 1;
            assert!(!tracker.due(update, 30_000, false, false));
            assert!(!tracker.due(update, 30_000, false, true));
            // A revised plan or forecast must still be reported.
            update.values[3] += 1;
            assert!(tracker.due(update, 30_000, false, false));
            tracker.record(update, 30_000);
            assert!(!tracker.due(update, 60_000, false, true));
            // A new command owns a fresh reporting scope.
            assert!(Tracker::default().due(update, 0, false, false));
        }
    }

    unsafe extern "C" fn display(
        context: *mut c_void,
        event: *const Update,
        elapsed: u64,
        _: u32,
    ) -> i32 {
        // SAFETY: the fixture passes its live Capture and a synchronous event.
        let capture = unsafe { &mut *context.cast::<Capture>() };
        assert_eq!(capture.owner, std::thread::current().id());
        capture.events.push(unsafe { *event });
        capture.elapsed.push(elapsed);
        capture.display_status
    }

    unsafe extern "C" fn operation(context: *mut c_void) -> i32 {
        progress::with_current(|state| {
            std::thread::scope(|scope| {
                scope.spawn(move || {
                    progress::with_progress(state, || {
                        progress::report(progress::PLAN, [3; 8]);
                        progress::advance(progress::LEVERAGE, 7, 7);
                        assert_eq!(poll(), 0); // Worker has no foreign callback.
                    })
                });
            })
        });
        // SAFETY: caller owns Capture through this synchronous operation.
        unsafe { (*context.cast::<Capture>()).operation_status }
    }

    #[test]
    fn scope_lifetime_failure_break_reuse_and_header_validation() {
        assert_eq!(size_of::<VckssProgressOptionsV1>(), 32);
        assert_eq!(size_of::<Update>(), 96);
        let short_header = 8_u64;
        // Only the aligned size header is readable; no callback may run.
        assert_eq!(
            unsafe {
                vckss_rust_report_call_v1(
                    (&short_header as *const u64).cast(),
                    None,
                    std::ptr::null_mut(),
                )
            },
            198
        );
        let mut capture = Capture {
            owner: std::thread::current().id(),
            events: vec![],
            elapsed: vec![],
            operation_status: 0,
            display_status: 0,
        };
        let context = (&mut capture as *mut Capture).cast();
        let mut options = VckssProgressOptionsV1 {
            struct_size: 32,
            schema: 1,
            level: 1,
            reserved: 0,
            display: Some(display),
            context,
        };
        let call = |options: &VckssProgressOptionsV1| {
            // SAFETY: all pointers refer to this test's live stack state.
            unsafe { vckss_rust_report_call_v1(options, Some(operation), context) }
        };
        assert_eq!(call(&options), 0);
        assert_eq!(capture.events.len(), 2);
        capture.events.clear();
        capture.operation_status = 498;
        assert_eq!(call(&options), 498);
        assert!(capture.events.is_empty()); // No final completion after failure.
        capture.operation_status = 0;
        capture.display_status = 1;
        assert_eq!(call(&options), 1);
        capture.display_status = 0;
        capture.events.clear();
        assert_eq!(call(&options), 0);
        assert_eq!(capture.events.len(), 2);
        options.schema = 3;
        assert_eq!(call(&options), 198);
        options.schema = 1;
        options.reserved = 1;
        assert_eq!(call(&options), 198);
        options.reserved = 0;
        options.level = 0;
        options.display = None;
        options.context = std::ptr::null_mut();
        capture.events.clear();
        assert_eq!(call(&options), 0);
        assert!(capture.events.is_empty());
        assert_eq!(poll(), 0);
        progress::with_current(|state| assert!(state.is_none()));
    }

    #[test]
    fn schema_two_pairs_probes_and_keeps_elapsed_across_phases() {
        let progress = Progress::default();
        let mut capture = Capture {
            owner: std::thread::current().id(),
            events: vec![],
            elapsed: vec![],
            operation_status: 0,
            display_status: 0,
        };
        let mut reporter = Reporter {
            options: VckssProgressOptionsV1 {
                struct_size: 32,
                schema: 2,
                level: 1,
                reserved: 0,
                display: Some(display),
                context: (&mut capture as *mut Capture).cast(),
            },
            progress: &progress,
            start: Instant::now(),
            trackers: [Tracker::default(); SLOTS],
            next_check_ms: 0,
            status: 0,
        };
        progress::with_progress(Some(&progress), || {
            progress::stage(progress::SETUP);
            assert_eq!(reporter.drain_at(1700, false), 0);
            progress::advance(progress::LEVERAGE, 200, 4500);
            reporter.drain_at(3200, false);
            assert_eq!(
                &capture.events.last().unwrap().values[..5],
                &[200, 4500, 0, 4500, 0]
            );
            // Coalescing both counters must print one consistent pair.
            progress::advance(progress::LEVERAGE, 4500, 4500);
            progress::advance(progress::TARGETS, 640, 4500);
            reporter.drain_at(4200, false);
            assert_eq!(capture.events.len(), 3);
            assert_eq!(
                &capture.events.last().unwrap().values[..4],
                &[4500, 4500, 640, 4500]
            );
            assert_ne!(capture.events.last().unwrap().values[4], 0);
            progress::advance(progress::TARGETS, 4500, 4500);
            reporter.drain_at(4300, false); // Completion bypasses periodic throttle.
            progress::stage(progress::VALIDATION);
            reporter.drain_at(4500, true);
        });
        assert_eq!(capture.elapsed, [1700, 3200, 4200, 4300, 4500]);
        assert_eq!(capture.events.last().unwrap().kind, progress::VALIDATION);
    }
}
