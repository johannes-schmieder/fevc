//! Capacity-based ownership accounting, separate from principal-array reports.

use crate::CmgError;
use std::alloc::Layout;
use std::sync::{Arc, OnceLock};

/// A CMG storage owner supported by [`CmgStorageReport`].
///
/// Sealed: implementations follow private owning fields, including nested banks.
/// `Vec<T>` is supported when `T` is supported, so the outer bank's spare
/// capacity is counted too. Borrowed numerical slices are intentionally absent.
pub trait CmgStorage: Storage {}
impl<T: Storage> CmgStorage for T {}

/// Simultaneously reachable requested heap storage, de-duplicated by allocation.
///
/// Includes vector capacities, nested descriptor banks and shared Arc payloads.
/// Arc control blocks use the reviewed Rust two-atomic layout and are reported
/// separately: that layout is not a stable standard-library ABI guarantee.
/// Excludes inline root values, caller buffers not supplied as roots, allocator
/// overhead/caches, runtime internals, TLS and OS stacks. This is an observation,
/// not a pre-allocation admission or an RSS bound. See `docs/memory-ownership.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CmgStorageReport {
    capacity_bytes: usize,
    arc_allocation_bytes: usize,
    allocations: usize,
    runtime_unbounded: bool,
}

impl CmgStorageReport {
    /// Measure the union of the supplied live owners without allocating.
    /// Include old and new banks/frames together while they coexist. Shared
    /// graphs, component maps and lineage objects are counted only once.
    /// Complexity is quadratic in allocation count, not element count; scalar
    /// payloads are never scanned. Do not call from numerical inner loops.
    pub fn measure(roots: &[&dyn CmgStorage]) -> Result<Self, CmgError> {
        let mut result = Self::default();
        let mut ordinal = 0;
        for root in roots {
            root.visit(&mut |allocation| {
                if allocation.runtime {
                    result.runtime_unbounded = true;
                    return Ok(());
                }
                let mut previous = 0;
                let mut duplicate = false;
                for other in roots {
                    other.visit(&mut |candidate| {
                        if !candidate.runtime {
                            // Arc identity is its payload pointer, whereas a vector's
                            // identity is its backing-allocation base. An Arc<()>
                            // payload can be one-past the header and coincide with
                            // the base of an adjacent vector allocation. Keep these
                            // two identity namespaces distinct.
                            if previous < ordinal
                                && candidate.address == allocation.address
                                && candidate.arc == allocation.arc
                            {
                                duplicate = true;
                            }
                            previous = add(previous, 1)?;
                        }
                        Ok(())
                    })?;
                }
                ordinal = add(ordinal, 1)?;
                if !duplicate {
                    let target = if allocation.arc {
                        &mut result.arc_allocation_bytes
                    } else {
                        &mut result.capacity_bytes
                    };
                    *target = add(*target, allocation.bytes)?;
                    result.allocations = add(result.allocations, 1)?;
                }
                Ok(())
            })?;
        }
        result.total_bytes()?;
        Ok(result)
    }

    /// Requested backing storage of vectors, boxes and strings, including slack.
    pub const fn capacity_bytes(self) -> usize {
        self.capacity_bytes
    }
    /// Shared Arc allocations, including inline payload and modeled control block.
    pub const fn arc_allocation_bytes(self) -> usize {
        self.arc_allocation_bytes
    }
    /// Checked sum of the two disjoint storage categories.
    pub fn total_bytes(self) -> Result<usize, CmgError> {
        add(self.capacity_bytes, self.arc_allocation_bytes)
    }
    /// Number of unique nonempty allocations in the covered categories.
    pub const fn allocation_count(self) -> usize {
        self.allocations
    }
    /// True when an included executor has additional unbounded runtime storage.
    pub const fn runtime_heap_unbounded(self) -> bool {
        self.runtime_unbounded
    }
}

// Public only within this private module, to seal CmgStorage without exposing
// a user-implementable accounting contract.
#[doc(hidden)]
pub struct Allocation {
    address: usize,
    bytes: usize,
    arc: bool,
    runtime: bool,
}

#[doc(hidden)]
pub trait Storage {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError>;
    fn leaf() -> bool
    where
        Self: Sized,
    {
        false
    }
}

/// A stored `u32` or native vertex index.
pub(crate) trait StoredIndex: Copy + Send + Sync {
    fn index(self) -> usize;
}

impl StoredIndex for u32 {
    #[inline(always)]
    fn index(self) -> usize {
        self as usize
    }
}

impl StoredIndex for usize {
    #[inline(always)]
    fn index(self) -> usize {
        self
    }
}

pub(crate) fn add(a: usize, b: usize) -> Result<usize, CmgError> {
    a.checked_add(b).ok_or(CmgError::InvalidHierarchy {
        context: "CMG storage size overflows",
    })
}
pub(crate) fn bytes<T>(count: usize) -> Result<usize, CmgError> {
    Layout::array::<T>(count)
        .map(|layout| layout.size())
        .map_err(|_| CmgError::AllocationFailed {
            context: "CMG storage layout overflows",
        })
}

pub(crate) fn try_capacity<T>(count: usize, context: &'static str) -> Result<Vec<T>, CmgError> {
    bytes::<T>(count)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| CmgError::AllocationFailed { context })?;
    Ok(values)
}

pub(crate) fn try_filled<T: Clone>(
    count: usize,
    value: T,
    context: &'static str,
) -> Result<Vec<T>, CmgError> {
    let mut values = try_capacity(count, context)?;
    values.resize(count, value);
    Ok(values)
}

pub(crate) fn admit(required_bytes: usize, budget_bytes: usize) -> Result<(), CmgError> {
    if required_bytes > budget_bytes {
        Err(CmgError::MemoryBudgetExceeded {
            required_bytes,
            budget_bytes,
        })
    } else {
        Ok(())
    }
}

pub(crate) fn arc_bytes<T>() -> Result<usize, CmgError> {
    Layout::new::<[std::sync::atomic::AtomicUsize; 2]>()
        .extend(Layout::new::<T>())
        .map(|(layout, _)| layout.pad_to_align().size())
        .map_err(|_| CmgError::InvalidHierarchy {
            context: "Arc storage layout overflows",
        })
}
#[cfg(feature = "parallel")]
pub(crate) fn runtime(
    visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
) -> Result<(), CmgError> {
    visitor(Allocation {
        address: 0,
        bytes: 0,
        arc: false,
        runtime: true,
    })
}

macro_rules! storage_fields {
    ($ty:ty, $($field:ident),+ $(,)?) => {
        impl $crate::storage::Storage for $ty {
            fn visit(&self, visitor: &mut dyn FnMut($crate::storage::Allocation) -> Result<(), $crate::CmgError>) -> Result<(), $crate::CmgError> {
                $($crate::storage::Storage::visit(&self.$field, visitor)?;)+
                Ok(())
            }
        }
    };
}
pub(crate) use storage_fields;

macro_rules! leaf {
    ($($ty:ty),+ $(,)?) => {$ (
        impl Storage for $ty {
            fn visit(&self, _: &mut dyn FnMut(Allocation) -> Result<(), CmgError>) -> Result<(), CmgError> { Ok(()) }
            fn leaf() -> bool { true }
        }
    )+};
}
leaf!(
    (),
    usize,
    u32,
    u64,
    f64,
    crate::Edge,
    crate::PcgDiagnostics,
    (u32, u32)
);

impl<T: Storage> Storage for Vec<T> {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        let bytes = bytes::<T>(self.capacity())?;
        if bytes != 0 {
            visitor(Allocation {
                address: self.as_ptr() as usize,
                bytes,
                arc: false,
                runtime: false,
            })?;
        }
        if !T::leaf() {
            for value in self {
                value.visit(visitor)?;
            }
        }
        Ok(())
    }
}
impl<T: Storage> Storage for Box<[T]> {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        let bytes = bytes::<T>(self.len())?;
        if bytes != 0 {
            visitor(Allocation {
                address: self.as_ptr() as usize,
                bytes,
                arc: false,
                runtime: false,
            })?;
        }
        if !T::leaf() {
            for value in self.iter() {
                value.visit(visitor)?;
            }
        }
        Ok(())
    }
}
impl<T: Storage> Storage for Arc<T> {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        // Rust ArcInner<T>: strong, weak, data. Layout::extend includes padding.
        let layout = Layout::new::<[std::sync::atomic::AtomicUsize; 2]>()
            .extend(Layout::new::<T>())
            .map_err(|_| CmgError::InvalidHierarchy {
                context: "Arc storage layout overflows",
            })?
            .0
            .pad_to_align();
        visitor(Allocation {
            address: Arc::as_ptr(self) as usize,
            bytes: layout.size(),
            arc: true,
            runtime: false,
        })?;
        (**self).visit(visitor)
    }
}
impl<T: Storage> Storage for Option<T> {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        if let Some(value) = self {
            value.visit(visitor)?;
        }
        Ok(())
    }
}
impl<T: Storage> Storage for OnceLock<T> {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        if let Some(value) = self.get() {
            value.visit(visitor)?;
        }
        Ok(())
    }
}
impl<T: Storage, E: Storage> Storage for Result<T, E> {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        match self {
            Ok(v) => v.visit(visitor),
            Err(e) => e.visit(visitor),
        }
    }
}
impl Storage for CmgError {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        if let Self::ParallelRuntime { message } = self {
            if message.capacity() != 0 {
                visitor(Allocation {
                    address: message.as_ptr() as usize,
                    bytes: message.capacity(),
                    arc: false,
                    runtime: false,
                })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod allocation_identity_tests {
    use super::*;

    struct AdjacentOwners;
    impl Storage for AdjacentOwners {
        fn visit(
            &self,
            visitor: &mut dyn FnMut(Allocation) -> Result<(), CmgError>,
        ) -> Result<(), CmgError> {
            // A zero-sized Arc payload can equal the next allocation's base.
            visitor(Allocation {
                address: 0x1010,
                bytes: 16,
                arc: true,
                runtime: false,
            })?;
            visitor(Allocation {
                address: 0x1010,
                bytes: 24,
                arc: false,
                runtime: false,
            })
        }
    }

    #[test]
    fn zero_sized_arc_payload_does_not_alias_adjacent_vector_base() {
        let owners = AdjacentOwners;
        let report = CmgStorageReport::measure(&[&owners, &owners]).unwrap();
        assert_eq!(report.total_bytes().unwrap(), 40);
        assert_eq!(report.arc_allocation_bytes(), 16);
        assert_eq!(report.capacity_bytes(), 24);
        assert_eq!(report.allocation_count(), 2);
    }
}
