// SPDX-License-Identifier: GPL-3.0-only

use vckss_core::engine::JlaEngineResult;
use vckss_core::jla::VarianceComponents;

fn exported_components(result: &JlaEngineResult) -> (VarianceComponents, VarianceComponents) {
    (result.plugin, result.corrected)
}

#[test]
fn JLA_result_exposes_plugin_and_corrected_components_by_value() {
    let projection: fn(&JlaEngineResult) -> (VarianceComponents, VarianceComponents) =
        exported_components;
    let _ = projection;
}
