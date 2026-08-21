// SPDX-License-Identifier: GPL-3.0-only

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let target = env::var("TARGET").expect("Cargo target triple");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let stata_source = manifest.join("../include/stplugin.c");
    let stata_include = manifest.join("../include");
    let shim_source = manifest.join("cshim/stata_entry.c");
    let shim_include = manifest.join("include");

    for path in [&stata_source, &shim_source] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        stata_include.join("stplugin.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shim_include.join("vckss_rust.h").display()
    );

    let objects = if target.contains("windows-msvc") {
        compile_msvc(
            &target,
            &out,
            &stata_include,
            &shim_include,
            [&stata_source, &shim_source],
        )
    } else {
        compile_unix(
            &target,
            &out,
            &stata_include,
            &shim_include,
            [&stata_source, &shim_source],
        )
    };

    for object in objects {
        println!("cargo:rustc-cdylib-link-arg={}", object.display());
    }
}

fn compile_unix<'a>(
    target: &str,
    out: &Path,
    stata_include: &Path,
    shim_include: &Path,
    sources: impl IntoIterator<Item = &'a PathBuf>,
) -> Vec<PathBuf> {
    let compiler = env::var_os("CC").unwrap_or_else(|| {
        if target.contains("apple-darwin") {
            OsString::from("clang")
        } else {
            OsString::from("cc")
        }
    });
    sources
        .into_iter()
        .enumerate()
        .map(|(index, source)| {
            let object = out.join(format!("vckss_stata_{index}.o"));
            let mut command = Command::new(&compiler);
            command
                .arg("-std=c11")
                .arg("-O3")
                .arg("-fPIC")
                .arg("-Wall")
                .arg("-Wextra")
                .arg("-Werror")
                .arg("-I")
                .arg(stata_include)
                .arg("-I")
                .arg(shim_include)
                .arg("-c")
                .arg(source)
                .arg("-o")
                .arg(&object);
            if target == "x86_64-apple-darwin" {
                command.arg("-arch").arg("x86_64");
            } else if target == "aarch64-apple-darwin" {
                command.arg("-arch").arg("arm64");
            }
            run(command, "compile the Stata C shim");
            object
        })
        .collect()
}

fn compile_msvc<'a>(
    _target: &str,
    out: &Path,
    stata_include: &Path,
    shim_include: &Path,
    sources: impl IntoIterator<Item = &'a PathBuf>,
) -> Vec<PathBuf> {
    let compiler = env::var_os("CC").unwrap_or_else(|| OsString::from("cl.exe"));
    sources
        .into_iter()
        .enumerate()
        .map(|(index, source)| {
            let object = out.join(format!("vckss_stata_{index}.obj"));
            let mut command = Command::new(&compiler);
            command
                .arg("/nologo")
                .arg("/O2")
                .arg("/W4")
                .arg("/WX")
                .arg("/std:c11")
                .arg("/D_CRT_SECURE_NO_WARNINGS")
                .arg(format!("/I{}", stata_include.display()))
                .arg(format!("/I{}", shim_include.display()))
                .arg("/c")
                .arg(source)
                .arg(format!("/Fo{}", object.display()));
            run(command, "compile the Stata C shim with MSVC");
            object
        })
        .collect()
}

fn run(mut command: Command, action: &str) {
    let status = command.status().unwrap_or_else(|error| {
        panic!("could not {action}: {error}");
    });
    assert!(status.success(), "failed to {action}: {status}");
}
