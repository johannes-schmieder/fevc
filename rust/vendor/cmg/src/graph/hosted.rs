//! Initial input graph only: bounded mapping and canonical parallel sorting.

use super::{Edge, Laplacian, edge_sort, map_edge};
use crate::cancel::{CancelMeter, Meter};
use crate::{CmgError, ControlledStage, Team};

pub(super) const MIN_EDGES: usize = 65_536;
const CHUNK: usize = 8;

/// Validate and map a large borrowed input in fixed logical chunks on a team,
/// sort it in place with the shared counted introsort on the same team, then
/// reduce duplicates with the serial canonical merge.
///
/// The mapping chunks are identical at every worker ceiling, including one;
/// the total comparator makes the sorted order independent of the ceiling.
/// Successful graph bits match the serial construction. The borrowed input and
/// team runtime are outside the caller's admitted bound. The in-place sort
/// allocates nothing; the bound's second edge bank covers the transient
/// compaction copy after duplicate reduction. Mapping
/// workers poll the meter's query before every logical chunk and after each
/// join; sorting lanes poll inside every partition; the coordinator's meter
/// polls between its bounded serial operations. Cancellation dominates worker
/// errors after joining; otherwise the earliest logical invalid edge wins. No
/// partial stage is retried.
pub(super) fn build<M: Meter>(
    vertex_count: usize,
    edges: &[(usize, usize, f64)],
    team: Team<'_>,
    meter: &mut M,
) -> Result<Laplacian, CmgError> {
    meter.work("input graph memory bound", 128)?;
    let mut raw = initialized_bank(edges.len(), meter)?;
    stage(
        "hosted input edge mapping",
        CHUNK,
        team,
        &mut raw,
        meter.query(),
        |chunk, output| {
            for (index, value) in output.iter_mut().enumerate() {
                *value = map_edge(vertex_count, edges[chunk * CHUNK + index])?;
            }
            Ok(())
        },
    )?;
    edge_sort::sort(&mut raw, team, meter)?;
    Laplacian::from_sorted_edges(vertex_count, raw, true, meter)
}

fn initialized_bank<M: Meter>(len: usize, meter: &mut M) -> Result<Vec<Edge>, CmgError> {
    let mut values = meter.try_capacity(len, "hosted input edge bank")?;
    // Safe Rust requires initialized disjoint output storage. Its first-touch
    // cost is counted for cancellation polling; the bank never grows after admission.
    for start in (0..len).step_by(128) {
        let count = (len - start).min(128);
        meter.work("hosted input edge initialization", (8 * count) as u16)?;
        for _ in 0..count {
            values.push(Edge::from_compact_parts(0, 0, 0.0));
        }
    }
    Ok(values)
}

/// Dispatch one stage of fixed logical chunks on at most the team's worker
/// ceiling, translating its cancellation to the stage's phase. Chunk
/// boundaries never depend on the worker ceiling.
pub(crate) fn stage<T: Send>(
    phase: &'static str,
    chunk_len: usize,
    team: Team<'_>,
    values: &mut [T],
    query: Option<crate::Cancellation<'_>>,
    kernel: impl Fn(usize, &mut [T]) -> Result<(), CmgError> + Sync,
) -> Result<(), CmgError> {
    let cancelled = || query.is_some_and(|cancel| cancel());
    match ControlledStage::admit(values.len(), chunk_len)?.run(team, values, cancelled, kernel) {
        Err(CmgError::Cancelled { .. }) => Err(CancelMeter::cancelled(phase)),
        other => other,
    }
}
