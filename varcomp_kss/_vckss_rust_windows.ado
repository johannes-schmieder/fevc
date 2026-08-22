capture program _vckss_rust_windows, plugin using("varcomp_kss_rust_windows_x64.plugin")
local vckss_rust_plugin_loader_rc = _rc
if `vckss_rust_plugin_loader_rc' & `vckss_rust_plugin_loader_rc' != 110 {
    exit `vckss_rust_plugin_loader_rc'
}
