// SPDX-License-Identifier: GPL-3.0-only
// No draws or changes to the fixture: serialize its existing inputs only.
fn population_input(input: &vckss_core::types::InputColumns, rep: usize, seed: u64) {
    println!("{{\"kind\":\"input\",\"replication\":{rep},\"seed\":{seed},\"worker\":{:?},\"firm\":{:?},\"deletion\":{:?},\"frequency\":{:?},\"target_weight\":{:?},\"controls\":{:?},\"outcome\":{:?}}}", input.worker, input.firm, input.deletion, input.frequency, input.target_weight, input.controls, input.outcome);
}
