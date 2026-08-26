#![cfg(feature = "parallel")]

use cmg::{ParallelExecutor, ParallelOptions};

#[test]
fn custom_executor_supports_up_to_thirty_two_threads() {
    for threads in [1, 2, 4, 8, 16, 32] {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            ..ParallelOptions::default()
        })
        .expect("custom Rayon pool should be constructible");
        assert_eq!(executor.thread_count(), threads);
    }
}
