if strpos("`c(machine_type)'", "Apple Silicon") {
    capture program fevc__rust_macos, plugin using("fevc_rust_macos_arm64.plugin")
}
else {
    capture program fevc__rust_macos, plugin using("fevc_rust_macos_x86_64.plugin")
}
local vckss_rust_plugin_loader_rc = _rc
if `vckss_rust_plugin_loader_rc' & `vckss_rust_plugin_loader_rc' != 110 {
    exit `vckss_rust_plugin_loader_rc'
}
