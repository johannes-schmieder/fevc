*! version 0.1.0 21aug2026
program define _vckss_rust_windows, plugin using("varcomp_kss_rust_windows_x64.plugin")
program define _vckss_rust_linux,   plugin using("varcomp_kss_rust_linux_x64.plugin")
program define _vckss_rust_macos,   plugin using("varcomp_kss_rust_macos.plugin")

program define varcomp_kss_rust, rclass
    version 18.0

    gettoken subcommand 0 : 0, parse(" ,")
    local subcommand = lower(strtrim("`subcommand'"))
    if "`subcommand'" == "" {
        di as err "Rust backend subcommand required"
        di as err "valid subcommands: capabilities, version, selftest, prepare, snapshot, release, clear"
        exit 198
    }

    local plugin
    if "`c(os)'" == "Windows" {
        local plugin _vckss_rust_windows
    }
    else if "`c(os)'" == "Unix" {
        local plugin _vckss_rust_linux
    }
    else if "`c(os)'" == "MacOSX" {
        local plugin _vckss_rust_macos
    }
    else {
        di as err "unsupported operating system for the Rust backend: `c(os)'"
        exit 198
    }

    if inlist("`subcommand'", "capabilities", "version", "selftest", "clear") {
        if strtrim(`"`0'"') != "" {
            di as err "`subcommand' does not accept additional arguments"
            exit 198
        }
        plugin call `plugin', `subcommand'
        return local backend "rust"
        return local subcommand "`subcommand'"
        exit
    }

    if "`subcommand'" == "snapshot" {
        if strtrim(`"`0'"') != "" {
            di as err "snapshot does not accept additional arguments"
            exit 198
        }
        plugin call `plugin', snapshot
        return scalar state = scalar(__vckss_rust_state)
        return scalar handle = scalar(__vckss_rust_handle)
        return scalar last_released = scalar(__vckss_rust_last_released)
        capture scalar drop __vckss_rust_state
        capture scalar drop __vckss_rust_handle
        capture scalar drop __vckss_rust_last_released
        return local backend "rust"
        return local subcommand "snapshot"
        exit
    }

    if "`subcommand'" == "release" {
        syntax anything(name=handle id="native Rust generation")
        capture confirm integer number `handle'
        if _rc | real("`handle'") <= 0 {
            di as err "release requires one positive integer native generation"
            exit 198
        }
        plugin call `plugin', release `handle'
        return scalar handle = real("`handle'")
        return local backend "rust"
        return local subcommand "release"
        exit
    }

    if "`subcommand'" == "prepare" {
        syntax varlist(min=6 max=6 numeric) [if] [in], [CLEANUP]
        marksample touse, novarlist
        markout `touse' `varlist'

        local cleanup_arg nocleanup
        if "`cleanup'" != "" local cleanup_arg cleanup

        plugin call `plugin' `touse' `varlist' if `touse', prepare `cleanup_arg'

        return scalar handle = scalar(__vckss_rust_handle)
        return scalar input_rows = scalar(__vckss_rust_input_rows)
        return scalar retained_rows = scalar(__vckss_rust_retained_rows)
        return scalar workers = scalar(__vckss_rust_workers)
        return scalar firms = scalar(__vckss_rust_firms)
        return scalar cells = scalar(__vckss_rust_cells)
        return scalar deletion_units = scalar(__vckss_rust_deletion_units)
        return scalar target_strata = scalar(__vckss_rust_target_strata)
        return local backend "rust"
        return local subcommand "prepare"

        capture scalar drop __vckss_rust_handle
        capture scalar drop __vckss_rust_input_rows
        capture scalar drop __vckss_rust_retained_rows
        capture scalar drop __vckss_rust_workers
        capture scalar drop __vckss_rust_firms
        capture scalar drop __vckss_rust_cells
        capture scalar drop __vckss_rust_deletion_units
        capture scalar drop __vckss_rust_target_strata
        exit
    }

    di as err "unknown Rust backend subcommand: `subcommand'"
    di as err "valid subcommands: capabilities, version, selftest, prepare, snapshot, release, clear"
    exit 198
end
