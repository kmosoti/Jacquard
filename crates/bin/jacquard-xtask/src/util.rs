//! Running commands for the collectors.

use std::path::Path;
use std::process::Command;

/// Runs a program in a directory and returns its standard output.
///
/// # Errors
///
/// Returns the program's standard error, or the spawn error, when it cannot
/// be started or exits unsuccessfully.
pub fn run(program: &str, args: &[&str], dir: &Path) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "{program} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Reads a file to a string, naming the file in the error.
///
/// # Errors
///
/// Returns the path and the I/O error when the file cannot be read.
pub fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs `cargo metadata --format-version 1` on a workspace manifest.
///
/// # Errors
///
/// Returns a message when cargo fails or its output is not JSON.
pub fn metadata(manifest: &Path) -> Result<serde_json::Value, String> {
    let dir = manifest.parent().unwrap_or(Path::new("."));
    let manifest = manifest.to_string_lossy();
    let out = run(
        "cargo",
        &[
            "metadata",
            "--format-version",
            "1",
            "--manifest-path",
            &manifest,
        ],
        dir,
    )?;
    serde_json::from_str(&out).map_err(|e| format!("cargo metadata: {e}"))
}
