// SPDX-License-Identifier: GPL-3.0-only

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

const STATA_SDK_ENV: &str = "VCKSS_STATA_SDK_DIR";

#[derive(Debug, Eq, PartialEq)]
struct StataSdk {
    include_dir: PathBuf,
    source: PathBuf,
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let target = env::var("TARGET").expect("Cargo target triple");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    println!("cargo:rerun-if-env-changed={STATA_SDK_ENV}");
    let stata_sdk = locate_stata_sdk(env::var_os(STATA_SDK_ENV)).unwrap_or_else(|message| {
        panic!("Stata SDK preflight failed: {message}");
    });
    let stata_source = stata_sdk.source;
    let stata_include = stata_sdk.include_dir;
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

fn locate_stata_sdk(configured_dir: Option<OsString>) -> Result<StataSdk, String> {
    let Some(configured_dir) = configured_dir.filter(|value| !value.is_empty()) else {
        return Err(format!(
            "set {STATA_SDK_ENV} to the directory containing authentic Stata Plugin SDK \
             stplugin.c and stplugin.h inputs; the SDK is not bundled with this repository"
        ));
    };

    let include_dir = PathBuf::from(configured_dir);
    if !include_dir.is_dir() {
        return Err(format!(
            "{STATA_SDK_ENV}={} is not a directory",
            include_dir.display()
        ));
    }

    let source = include_dir.join("stplugin.c");
    let header = include_dir.join("stplugin.h");
    for required in [&source, &header] {
        if !required.is_file() {
            return Err(format!(
                "{STATA_SDK_ENV}={} is missing required SDK input {}; this preflight validates \
                 only the required filenames, not SDK provenance or content hashes",
                include_dir.display(),
                required.display()
            ));
        }
    }

    Ok(StataSdk {
        include_dir,
        source,
    })
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

#[cfg(test)]
mod tests {
    use super::{locate_stata_sdk, STATA_SDK_ENV};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn fixture_directory() -> PathBuf {
        let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "vckss-stata-build-test-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("create isolated SDK fixture directory");
        directory
    }

    #[test]
    fn sdk_directory_must_be_explicit() {
        let error = locate_stata_sdk(None).expect_err("missing SDK directory must fail closed");
        assert!(error.contains(STATA_SDK_ENV));
        assert!(error.contains("not bundled"));
    }

    #[test]
    fn sdk_directory_requires_both_expected_inputs() {
        let directory = fixture_directory();
        fs::write(directory.join("stplugin.h"), b"/* test fixture */\n")
            .expect("write fixture header");

        let error = locate_stata_sdk(Some(directory.clone().into_os_string()))
            .expect_err("missing SDK source must fail closed");
        assert!(error.contains("stplugin.c"));
        assert!(error.contains("not SDK provenance or content hashes"));

        fs::remove_dir_all(directory).expect("remove isolated SDK fixture directory");
    }

    #[test]
    fn sdk_directory_with_both_inputs_is_accepted() {
        let directory = fixture_directory();
        fs::write(directory.join("stplugin.c"), b"/* test fixture */\n")
            .expect("write fixture source");
        fs::write(directory.join("stplugin.h"), b"/* test fixture */\n")
            .expect("write fixture header");

        let sdk = locate_stata_sdk(Some(directory.clone().into_os_string()))
            .expect("complete SDK fixture must pass the filename preflight");
        assert_eq!(sdk.include_dir, directory);
        assert_eq!(sdk.source, sdk.include_dir.join("stplugin.c"));

        fs::remove_dir_all(sdk.include_dir).expect("remove isolated SDK fixture directory");
    }
}
