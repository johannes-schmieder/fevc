capture program _vckss_rust_linux, plugin using("varcomp_kss_rust_linux_x64.plugin")
local vckss_rust_plugin_loader_rc = _rc
if `vckss_rust_plugin_loader_rc' & `vckss_rust_plugin_loader_rc' != 110 {
    exit `vckss_rust_plugin_loader_rc'
}
