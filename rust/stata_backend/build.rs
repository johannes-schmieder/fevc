// SPDX-License-Identifier: GPL-3.0-only

use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const STATA_SPI_ENV: &str = "VCKSS_STATA_SPI_DIR";
const LOCAL_SPI_DIRECTORY: &str = "stata-spi";

#[derive(Debug, Eq, PartialEq)]
struct StataSpi {
    include_dir: PathBuf,
    source: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
struct SpiHashes {
    source: String,
    header: String,
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let target = env::var("TARGET").expect("Cargo target triple");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let hash_manifest = manifest.join("stata-spi.sha256");
    println!("cargo:rerun-if-changed={}", hash_manifest.display());
    let expected_hashes = load_spi_hashes(&hash_manifest).unwrap_or_else(|message| {
        panic!("Stata SPI hash-manifest preflight failed: {message}");
    });
    println!("cargo:rerun-if-env-changed={STATA_SPI_ENV}");
    let configured_spi = env::var_os(STATA_SPI_ENV)
        .or_else(|| Some(manifest.join(LOCAL_SPI_DIRECTORY).into_os_string()));
    let stata_spi = locate_stata_spi(configured_spi, &expected_hashes).unwrap_or_else(|message| {
        panic!("Stata SPI preflight failed: {message}");
    });
    let stata_source = stata_spi.source;
    let stata_include = stata_spi.include_dir;
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

    println!(
        "cargo:rerun-if-changed={}",
        manifest.join("cshim/stata_progress.h").display()
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
    if target.contains("apple-darwin") {
        let minimum_version = minimum_macos_version(&target);
        println!("cargo:rustc-cdylib-link-arg=-mmacosx-version-min={minimum_version}");
        println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/fevc_rust_macos.plugin");
    }
}

fn locate_stata_spi(
    configured_dir: Option<OsString>,
    expected_hashes: &SpiHashes,
) -> Result<StataSpi, String> {
    let Some(configured_dir) = configured_dir.filter(|value| !value.is_empty()) else {
        return Err(format!(
            "run rust/stata_backend/fetch_stata_spi.sh or set {STATA_SPI_ENV} to a directory \
             containing the public Stata SPI files stplugin.c and stplugin.h"
        ));
    };

    let include_dir = PathBuf::from(configured_dir);
    if !include_dir.is_dir() {
        return Err(format!(
            "{STATA_SPI_ENV}={} is not a directory",
            include_dir.display()
        ));
    }

    let source = include_dir.join("stplugin.c");
    let header = include_dir.join("stplugin.h");
    for required in [&source, &header] {
        if !required.is_file() {
            return Err(format!(
                "{STATA_SPI_ENV}={} is missing required SPI input {}",
                include_dir.display(),
                required.display()
            ));
        }
    }
    verify_hash(&source, &expected_hashes.source)?;
    verify_hash(&header, &expected_hashes.header)?;

    Ok(StataSpi {
        include_dir,
        source,
    })
}

fn load_spi_hashes(path: &Path) -> Result<SpiHashes, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let mut source = None;
    let mut header = None;
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.split_whitespace();
        let hash = fields.next().unwrap_or_default();
        let name = fields.next().unwrap_or_default();
        if fields.next().is_some() || hash.len() != 64 {
            return Err(format!("invalid hash-manifest line: {line}"));
        }
        match name {
            "stplugin.c" => source = Some(hash.to_owned()),
            "stplugin.h" => header = Some(hash.to_owned()),
            _ => return Err(format!("unexpected SPI filename in hash manifest: {name}")),
        }
    }
    Ok(SpiHashes {
        source: source.ok_or_else(|| "stplugin.c hash is missing".to_owned())?,
        header: header.ok_or_else(|| "stplugin.h hash is missing".to_owned())?,
    })
}

fn verify_hash(path: &Path, expected: &str) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read SPI input {}: {error}", path.display()))?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(format!(
            "SPI input {} has SHA-256 {actual}, expected {expected}",
            path.display()
        ));
    }
    Ok(())
}

fn minimum_macos_version(target: &str) -> &'static str {
    if target == "aarch64-apple-darwin" {
        "11.0"
    } else {
        "10.13"
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
            if target.contains("apple-darwin") {
                command.arg("-DSYSTEM=APPLEMAC");
                command.arg(format!(
                    "-mmacosx-version-min={}",
                    minimum_macos_version(target)
                ));
            } else {
                command.arg("-DSYSTEM=OPUNIX");
            }
            if index == 0 {
                command.arg("-Dpginit=vckss_spi_pginit");
            }
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
    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let crt_flag = msvc_crt_flag(&target_features);
    sources
        .into_iter()
        .enumerate()
        .map(|(index, source)| {
            let object = out.join(format!("vckss_stata_{index}.obj"));
            let mut command = Command::new(&compiler);
            command
                .arg("/nologo")
                .arg("/O2")
                // C and Rust must link the same CRT, including static RC builds.
                .arg(crt_flag)
                .arg("/W4")
                .arg("/WX")
                .arg("/std:c11")
                .arg("/D_CRT_SECURE_NO_WARNINGS")
                .arg(format!("/I{}", stata_include.display()))
                .arg(format!("/I{}", shim_include.display()))
                .arg("/c")
                .arg(source)
                .arg(format!("/Fo{}", object.display()));
            if index == 0 {
                command.arg("/Dpginit=vckss_spi_pginit");
            }
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

fn msvc_crt_flag(target_features: &str) -> &'static str {
    if target_features
        .split(',')
        .any(|feature| feature == "crt-static")
    {
        "/MT"
    } else {
        "/MD"
    }
}

#[cfg(test)]
mod tests {
    use super::{locate_stata_spi, minimum_macos_version, msvc_crt_flag, SpiHashes, STATA_SPI_ENV};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
    const SOURCE: &[u8] = b"/* SPI source fixture */\n";
    const HEADER: &[u8] = b"/* SPI header fixture */\n";

    #[test]
    fn windows_c_crt_matches_rust_target_features() {
        assert_eq!(msvc_crt_flag("crt-static,sse2"), "/MT");
        assert_eq!(msvc_crt_flag("sse2"), "/MD");
        assert_eq!(msvc_crt_flag(""), "/MD");
    }

    fn fixture_hashes() -> SpiHashes {
        SpiHashes {
            source: format!("{:x}", Sha256::digest(SOURCE)),
            header: format!("{:x}", Sha256::digest(HEADER)),
        }
    }

    fn fixture_directory() -> PathBuf {
        let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "vckss-stata-build-test-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("create isolated SPI fixture directory");
        directory
    }

    #[test]
    fn spi_directory_must_be_explicit() {
        let error = locate_stata_spi(None, &fixture_hashes())
            .expect_err("missing SPI directory must fail closed");
        assert!(error.contains(STATA_SPI_ENV));
        assert!(error.contains("fetch_stata_spi.sh"));
    }

    #[test]
    fn spi_directory_requires_both_expected_inputs() {
        let directory = fixture_directory();
        fs::write(directory.join("stplugin.h"), HEADER).expect("write fixture header");

        let error = locate_stata_spi(Some(directory.clone().into_os_string()), &fixture_hashes())
            .expect_err("missing SPI source must fail closed");
        assert!(error.contains("stplugin.c"));
        assert!(error.contains("missing required SPI input"));

        fs::remove_dir_all(directory).expect("remove isolated SPI fixture directory");
    }

    #[test]
    fn spi_directory_with_both_inputs_is_accepted() {
        let directory = fixture_directory();
        fs::write(directory.join("stplugin.c"), SOURCE).expect("write fixture source");
        fs::write(directory.join("stplugin.h"), HEADER).expect("write fixture header");

        let spi = locate_stata_spi(Some(directory.clone().into_os_string()), &fixture_hashes())
            .expect("complete SPI fixture must pass the filename preflight");
        assert_eq!(spi.include_dir, directory);
        assert_eq!(spi.source, spi.include_dir.join("stplugin.c"));

        fs::remove_dir_all(spi.include_dir).expect("remove isolated SPI fixture directory");
    }

    #[test]
    fn spi_directory_rejects_unreviewed_contents() {
        let directory = fixture_directory();
        fs::write(directory.join("stplugin.c"), b"unexpected source\n")
            .expect("write fixture source");
        fs::write(directory.join("stplugin.h"), HEADER).expect("write fixture header");

        let error = locate_stata_spi(Some(directory.clone().into_os_string()), &fixture_hashes())
            .expect_err("unreviewed SPI contents must fail closed");
        assert!(error.contains("SHA-256"));

        fs::remove_dir_all(directory).expect("remove isolated SPI fixture directory");
    }

    #[test]
    fn macos_minimum_versions_match_supported_architectures() {
        assert_eq!(minimum_macos_version("aarch64-apple-darwin"), "11.0");
        assert_eq!(minimum_macos_version("x86_64-apple-darwin"), "10.13");
    }
}
