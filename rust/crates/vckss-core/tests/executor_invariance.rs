// SPDX-License-Identifier: GPL-3.0-only

use vckss_core::parallel::DeterministicExecutor;

fn assert_exact_cover(length: usize, threads: usize) {
    let ranges = DeterministicExecutor::new(threads)
        .expect("positive thread count")
        .partitions(length);

    assert_eq!(ranges.len(), threads.min(length));
    let mut cursor = 0_usize;
    let mut minimum_width = usize::MAX;
    let mut maximum_width = 0_usize;
    for range in &ranges {
        assert_eq!(range.start, cursor, "partition gap or overlap");
        assert!(range.start < range.end, "empty partition");
        let width = range.end - range.start;
        minimum_width = minimum_width.min(width);
        maximum_width = maximum_width.max(width);
        cursor = range.end;
    }
    assert_eq!(cursor, length);
    if !ranges.is_empty() {
        assert!(maximum_width - minimum_width <= 1);
    }
}

#[test]
fn partitions_are_stable_for_one_through_thirty_two_threads() {
    for threads in 1..=32 {
        for length in [0, 1, 2, 3, 7, 31, 32, 33, 257, 4_099] {
            assert_exact_cover(length, threads);
            let executor = DeterministicExecutor::new(threads).expect("executor");
            assert_eq!(executor.partitions(length), executor.partitions(length));
        }
    }
}

#[test]
fn mapped_results_reconstruct_input_in_partition_order() {
    let input: Vec<u64> = (0..4_099)
        .map(|index| {
            let value = index as u64;
            value
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                .rotate_left((index % 63) as u32)
        })
        .collect();

    for threads in 1..=32 {
        let pieces = DeterministicExecutor::new(threads)
            .expect("executor")
            .map_partitions(input.len(), |range| Ok(input[range].to_vec()))
            .expect("partition mapping");
        let reconstructed: Vec<u64> = pieces.into_iter().flatten().collect();
        assert_eq!(
            reconstructed, input,
            "result order changed at {threads} threads"
        );
    }
}

#[test]
fn caller_executes_only_the_first_partition() {
    let caller = std::thread::current().id();
    for threads in 2..=32 {
        let on_caller = DeterministicExecutor::new(threads)
            .expect("executor")
            .map_partitions(threads * 4, |_| Ok(std::thread::current().id() == caller))
            .expect("partition mapping");

        assert_eq!(on_caller.len(), threads);
        assert!(on_caller[0], "first partition left the caller thread");
        assert!(
            on_caller[1..].iter().all(|&value| !value),
            "a spawned partition unexpectedly ran on the caller thread"
        );
    }
}
