//! Every parallel CMG stage dispatches through the team's ceiling-capped
//! dispatchers (`src/execution.rs`) or `ControlledStage`
//! (`src/execution_stage.rs`). No other source file may fork or install work.

use std::path::Path;

fn sources(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn only_the_dispatchers_fork_work() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    let allowed = ["execution.rs", "execution_stage.rs"];
    let forbidden = [
        "par_iter",
        "par_chunks",
        "par_sort",
        "into_par_iter",
        "par_bridge",
        "rayon::join",
        "rayon::scope",
        "rayon::spawn",
        "rayon::prelude",
        ".install(",
    ];
    let mut violations = Vec::new();
    for file in files {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        if allowed.contains(&name.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        for (number, line) in text.lines().enumerate() {
            if forbidden.iter().any(|token| line.contains(token)) {
                violations.push(format!(
                    "{}:{}: {}",
                    file.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}
