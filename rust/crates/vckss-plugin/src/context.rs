// SPDX-License-Identifier: GPL-3.0-only

//! Generation-safe ownership for one staged native backend context.
//!
//! The Stata plugin runs synchronously, but interrupted commands and stale
//! handles still require an explicit lifecycle.  This module is deliberately
//! independent of the Stata SPI: callers prepare one owned payload, consume it
//! in one solve, export a validated result, and release resources idempotently.

use std::panic::{catch_unwind, AssertUnwindSafe};

use vckss_core::error::{BackendError, ErrorCode, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ContextHandle(u64);

impl ContextHandle {
    pub fn from_generation(generation: u64) -> Result<Self> {
        if generation == 0 {
            return Err(BackendError::new(
                ErrorCode::StaleContext,
                "context_ffi",
                "native context generation zero is invalid",
            ));
        }
        Ok(Self(generation))
    }

    #[must_use]
    pub const fn generation(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextStateTag {
    Empty,
    Prepared,
    Solving,
    Solved,
    Failed,
    Poisoned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextSnapshot {
    pub state: ContextStateTag,
    pub generation: Option<u64>,
    pub last_released_generation: u64,
}

#[derive(Debug)]
enum ContextState<Prepared, Solved> {
    Prepared(Prepared),
    Solving,
    Solved(Solved),
    Failed {
        error: BackendError,
        prepared: Option<Prepared>,
    },
    Poisoned {
        message: String,
        prepared: Option<Prepared>,
    },
}

impl<Prepared, Solved> ContextState<Prepared, Solved> {
    const fn tag(&self) -> ContextStateTag {
        match self {
            Self::Prepared(_) => ContextStateTag::Prepared,
            Self::Solving => ContextStateTag::Solving,
            Self::Solved(_) => ContextStateTag::Solved,
            Self::Failed { .. } => ContextStateTag::Failed,
            Self::Poisoned { .. } => ContextStateTag::Poisoned,
        }
    }
}

/// Borrowed active payload for metadata that must remain under the registry's
/// ownership before, during, and after a numerical solve.
#[derive(Clone, Copy, Debug)]
pub enum ContextPayloadRef<'a, Prepared, Solved> {
    Prepared(&'a Prepared),
    Solved(&'a Solved),
}

#[derive(Debug)]
struct ActiveContext<Prepared, Solved> {
    generation: u64,
    state: ContextState<Prepared, Solved>,
}

/// Registry for at most one production context in one Stata process.
#[derive(Debug)]
pub struct ContextRegistry<Prepared, Solved> {
    next_generation: u64,
    last_released_generation: u64,
    active: Option<ActiveContext<Prepared, Solved>>,
}

impl<Prepared, Solved> Default for ContextRegistry<Prepared, Solved> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Prepared, Solved> ContextRegistry<Prepared, Solved> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_generation: 1,
            last_released_generation: 0,
            active: None,
        }
    }

    pub fn prepare(&mut self, payload: Prepared) -> Result<ContextHandle> {
        if let Some(active) = &self.active {
            return Err(BackendError::new(
                ErrorCode::ContextPoisoned,
                "context_prepare",
                format!(
                    "generation {} is still active in state {:?}; release it before preparing another context",
                    active.generation,
                    active.state.tag()
                ),
            ));
        }
        let generation = self.next_generation;
        self.next_generation = self.next_generation.checked_add(1).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "context_prepare",
                "native context generation counter overflow",
            )
        })?;
        self.active = Some(ActiveContext {
            generation,
            state: ContextState::Prepared(payload),
        });
        Ok(ContextHandle(generation))
    }

    /// Consume a prepared payload and store either one solved result or a
    /// terminal failure. A Rust panic is contained and poisons the context.
    pub fn solve<F>(&mut self, handle: ContextHandle, solve: F) -> Result<()>
    where
        F: FnOnce(Prepared) -> Result<Solved>,
    {
        self.require_generation(handle)?;
        let prepared = {
            let active = self.active.as_mut().expect("generation was validated");
            let state = std::mem::replace(&mut active.state, ContextState::Solving);
            match state {
                ContextState::Prepared(payload) => payload,
                other => {
                    active.state = other;
                    return Err(invalid_state(
                        "context_solve",
                        handle,
                        active.state.tag(),
                        ContextStateTag::Prepared,
                    ));
                }
            }
        };

        match catch_unwind(AssertUnwindSafe(|| solve(prepared))) {
            Ok(Ok(result)) => {
                self.active
                    .as_mut()
                    .expect("active during synchronous solve")
                    .state = ContextState::Solved(result);
                Ok(())
            }
            Ok(Err(error)) => {
                self.active
                    .as_mut()
                    .expect("active during synchronous solve")
                    .state = ContextState::Failed {
                    error: error.clone(),
                    prepared: None,
                };
                Err(error)
            }
            Err(_) => {
                let message = "Rust panic was contained while solving the staged context";
                self.active
                    .as_mut()
                    .expect("active during synchronous solve")
                    .state = ContextState::Poisoned {
                    message: message.to_owned(),
                    prepared: None,
                };
                Err(BackendError::new(
                    ErrorCode::Panic,
                    "context_solve",
                    message,
                ))
            }
        }
    }

    /// Solve while retaining the prepared payload inside a terminal failure.
    /// This is used by the production ABI so the authoritative preparation
    /// receipt and retained-row mask survive a failed numerical solve.
    pub fn solve_preserving<F>(&mut self, handle: ContextHandle, solve: F) -> Result<()>
    where
        F: FnOnce(&Prepared) -> Result<Solved>,
    {
        self.require_generation(handle)?;
        let prepared = {
            let active = self.active.as_mut().expect("generation was validated");
            let state = std::mem::replace(&mut active.state, ContextState::Solving);
            match state {
                ContextState::Prepared(payload) => payload,
                other => {
                    active.state = other;
                    return Err(invalid_state(
                        "context_solve",
                        handle,
                        active.state.tag(),
                        ContextStateTag::Prepared,
                    ));
                }
            }
        };

        match catch_unwind(AssertUnwindSafe(|| solve(&prepared))) {
            Ok(Ok(result)) => {
                self.active
                    .as_mut()
                    .expect("active during synchronous solve")
                    .state = ContextState::Solved(result);
                Ok(())
            }
            Ok(Err(error)) => {
                self.active
                    .as_mut()
                    .expect("active during synchronous solve")
                    .state = ContextState::Failed {
                    error: error.clone(),
                    prepared: Some(prepared),
                };
                Err(error)
            }
            Err(_) => {
                let message = "Rust panic was contained while solving the staged context";
                self.active
                    .as_mut()
                    .expect("active during synchronous solve")
                    .state = ContextState::Poisoned {
                    message: message.to_owned(),
                    prepared: Some(prepared),
                };
                Err(BackendError::new(
                    ErrorCode::Panic,
                    "context_solve",
                    message,
                ))
            }
        }
    }

    pub fn result(&self, handle: ContextHandle) -> Result<&Solved> {
        self.require_generation(handle)?;
        let active = self.active.as_ref().expect("generation was validated");
        match &active.state {
            ContextState::Solved(result) => Ok(result),
            ContextState::Failed { error, .. } => Err(error.clone()),
            ContextState::Poisoned { message, .. } => Err(BackendError::new(
                ErrorCode::ContextPoisoned,
                "context_export",
                message.clone(),
            )),
            state => Err(invalid_state(
                "context_export",
                handle,
                state.tag(),
                ContextStateTag::Solved,
            )),
        }
    }

    /// Inspect the registry-owned payload without creating a parallel metadata
    /// registry. Failed or poisoned preserving solves expose their original
    /// prepared payload; ordinary terminal failures retain their typed error.
    pub fn payload(
        &self,
        handle: ContextHandle,
    ) -> Result<ContextPayloadRef<'_, Prepared, Solved>> {
        self.require_generation(handle)?;
        let active = self.active.as_ref().expect("generation was validated");
        match &active.state {
            ContextState::Prepared(prepared) => Ok(ContextPayloadRef::Prepared(prepared)),
            ContextState::Solved(solved) => Ok(ContextPayloadRef::Solved(solved)),
            ContextState::Failed {
                prepared: Some(prepared),
                ..
            }
            | ContextState::Poisoned {
                prepared: Some(prepared),
                ..
            } => Ok(ContextPayloadRef::Prepared(prepared)),
            ContextState::Failed { error, .. } => Err(error.clone()),
            ContextState::Poisoned { message, .. } => Err(BackendError::new(
                ErrorCode::ContextPoisoned,
                "context_export",
                message.clone(),
            )),
            ContextState::Solving => Err(invalid_state(
                "context_export",
                handle,
                ContextStateTag::Solving,
                ContextStateTag::Prepared,
            )),
        }
    }

    /// Release the matching active context. Repeating release for an already
    /// released generation succeeds and returns `false`.
    pub fn release(&mut self, handle: ContextHandle) -> Result<bool> {
        match &self.active {
            Some(active) if active.generation == handle.generation() => {
                self.active = None;
                self.last_released_generation =
                    self.last_released_generation.max(handle.generation());
                Ok(true)
            }
            Some(active) => Err(stale_handle(
                "context_release",
                handle,
                Some(active.generation),
            )),
            None if handle.generation() <= self.last_released_generation => Ok(false),
            None => Err(stale_handle("context_release", handle, None)),
        }
    }

    /// Drop any abandoned context at command entry. This is intentionally
    /// generation-agnostic and returns the generation that was discarded.
    pub fn clear_abandoned(&mut self) -> Option<ContextHandle> {
        let active = self.active.take()?;
        self.last_released_generation = self.last_released_generation.max(active.generation);
        Some(ContextHandle(active.generation))
    }

    #[must_use]
    pub fn snapshot(&self) -> ContextSnapshot {
        match &self.active {
            Some(active) => ContextSnapshot {
                state: active.state.tag(),
                generation: Some(active.generation),
                last_released_generation: self.last_released_generation,
            },
            None => ContextSnapshot {
                state: ContextStateTag::Empty,
                generation: None,
                last_released_generation: self.last_released_generation,
            },
        }
    }

    fn require_generation(&self, handle: ContextHandle) -> Result<()> {
        match &self.active {
            Some(active) if active.generation == handle.generation() => Ok(()),
            Some(active) => Err(stale_handle("context", handle, Some(active.generation))),
            None => Err(stale_handle("context", handle, None)),
        }
    }
}

fn invalid_state(
    phase: &'static str,
    handle: ContextHandle,
    actual: ContextStateTag,
    required: ContextStateTag,
) -> BackendError {
    BackendError::new(
        ErrorCode::ContextPoisoned,
        phase,
        format!(
            "generation {} is in state {actual:?}; state {required:?} is required",
            handle.generation()
        ),
    )
}

fn stale_handle(
    phase: &'static str,
    handle: ContextHandle,
    active_generation: Option<u64>,
) -> BackendError {
    let active = active_generation.map_or_else(
        || "no generation is active".to_owned(),
        |generation| format!("active generation is {generation}"),
    );
    BackendError::new(
        ErrorCode::StaleContext,
        phase,
        format!(
            "stale native context generation {}; {active}",
            handle.generation()
        ),
    )
}
