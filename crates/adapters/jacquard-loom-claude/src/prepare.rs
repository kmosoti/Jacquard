//! Safe Loom workspace materialization with symlink rejection, canonical
//! containment, and sensitive untracked filename exclusion.

use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Read as _, Write as _};
use std::path::{Component, Path};

use jacquard_ports::Failure;

use crate::git;

#[cfg(unix)]
fn file_identity(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    (meta.dev(), meta.ino())
}

#[cfg(not(unix))]
fn file_identity(_meta: &std::fs::Metadata) -> (u64, u64) {
    (0, 0)
}

fn validate_rel_path(rel: &str) -> Result<(), Failure> {
    if rel.is_empty()
        || rel.contains('\0')
        || rel.starts_with('/')
        || rel.contains('\\')
        || rel.contains(':')
    {
        return Err(Failure::Failed(format!(
            "invalid relative path {rel:?} in Loom workspace"
        )));
    }
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(Failure::Failed(format!(
                "invalid segment {seg:?} in Loom path {rel:?}"
            )));
        }
    }
    if !Path::new(rel)
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(Failure::Failed(format!(
            "non-normal path component in Loom path {rel:?}"
        )));
    }
    Ok(())
}

/// Returns true if an untracked relative path matches a sensitive filename or
/// credential pattern that must never cross into the external model workspace.
fn is_sensitive_untracked(rel: &str) -> bool {
    let lower_path = rel.to_ascii_lowercase();
    let segments: Vec<&str> = lower_path.split('/').collect();
    if segments[..segments.len().saturating_sub(1)]
        .iter()
        .any(|seg| {
            *seg == ".ssh"
                || *seg == ".gnupg"
                || *seg == ".aws"
                || seg.contains("secret")
                || seg.contains("credential")
        })
    {
        return true;
    }
    let name = segments.last().copied().unwrap_or("");
    let ext_sensitive = Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pem") || ext.eq_ignore_ascii_case("key"));
    name == ".env"
        || name.starts_with(".env.")
        || ext_sensitive
        || name.contains("secret")
        || name.contains("credential")
        || name.contains("token")
        || name == ".npmrc"
        || name == ".netrc"
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
}

fn copy_contained(
    base: &Path,
    canon_base: &Path,
    out: &Path,
    rel: &str,
    tracked: bool,
) -> Result<(), Failure> {
    validate_rel_path(rel)?;
    let mut cur = base.to_path_buf();
    let segs: Vec<&str> = rel.split('/').collect();
    for dir_seg in &segs[..segs.len() - 1] {
        cur.push(dir_seg);
        let dmeta = match std::fs::symlink_metadata(&cur) {
            Ok(m) => m,
            Err(e) if tracked && e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(Failure::Failed(format!("{}: {e}", cur.display()))),
        };
        if dmeta.file_type().is_symlink() || !dmeta.is_dir() {
            return Err(Failure::Failed(format!(
                "symlink or non-directory component {rel} is not allowed in Loom workspace"
            )));
        }
    }

    let from = base.join(rel);
    let meta = match std::fs::symlink_metadata(&from) {
        Ok(m) => m,
        Err(e) if tracked && e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Failure::Failed(format!("{}: {e}", from.display()))),
    };
    if meta.file_type().is_symlink() {
        return Err(Failure::Failed(format!(
            "symlink {rel} is not allowed in Loom workspace"
        )));
    }
    if !meta.file_type().is_file() {
        return Err(Failure::Failed(format!(
            "non-regular file {rel} is not allowed in Loom workspace"
        )));
    }
    let canon_from = std::fs::canonicalize(&from)
        .map_err(|e| Failure::Failed(format!("{}: {e}", from.display())))?;
    if !canon_from.starts_with(canon_base) {
        return Err(Failure::Failed(format!(
            "path {rel} resolves outside Loom base directory"
        )));
    }

    if !tracked && is_sensitive_untracked(rel) {
        return Ok(());
    }

    let mut src = std::fs::File::open(&canon_from)
        .map_err(|e| Failure::Failed(format!("{}: {e}", from.display())))?;
    let opened_meta = src
        .metadata()
        .map_err(|e| Failure::Failed(format!("{}: {e}", from.display())))?;
    if !opened_meta.is_file() || file_identity(&meta) != file_identity(&opened_meta) {
        return Err(Failure::Failed(format!(
            "file {rel} changed during Loom preparation"
        )));
    }

    let to = out.join(rel);
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Failure::Failed(e.to_string()))?;
    }
    let mut dst = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&to)
        .map_err(|e| Failure::Failed(format!("{}: {e}", to.display())))?;
    let mut buf = Vec::new();
    src.read_to_end(&mut buf)
        .map_err(|e| Failure::Failed(format!("{}: {e}", from.display())))?;
    dst.write_all(&buf)
        .map_err(|e| Failure::Failed(format!("{}: {e}", to.display())))?;
    Ok(())
}

/// Copies tracked and non-sensitive untracked regular files under `base`
/// into `out`, rejecting symlinks/non-regular entries and path escapes, and
/// initializes a clean git repository at `out`.
///
/// # Errors
///
/// Fails when `base` is invalid, a symlink or non-regular entry is listed,
/// a path escapes `base`, or copying/git initialization fails.
pub fn prepare(base: &Path, out: &Path) -> Result<(), Failure> {
    let base_meta = std::fs::symlink_metadata(base)
        .map_err(|e| Failure::Failed(format!("{}: {e}", base.display())))?;
    if base_meta.file_type().is_symlink() || !base_meta.is_dir() {
        return Err(Failure::Failed(format!(
            "{}: Loom base must be a real directory, not a symlink",
            base.display()
        )));
    }
    let canon_base = std::fs::canonicalize(base)
        .map_err(|e| Failure::Failed(format!("{}: {e}", base.display())))?;

    let tracked_raw = git(base, &["ls-files", "-z", "--cached", "."])?;
    let untracked_raw = git(
        base,
        &["ls-files", "-z", "--others", "--exclude-standard", "."],
    )?;

    let _ = std::fs::remove_dir_all(out);
    std::fs::create_dir_all(out).map_err(|e| Failure::Failed(e.to_string()))?;

    let mut seen = BTreeSet::new();
    for rel in tracked_raw.split('\0').filter(|s| !s.is_empty()) {
        if seen.insert(rel.to_string()) {
            copy_contained(base, &canon_base, out, rel, true)?;
        }
    }
    for rel in untracked_raw.split('\0').filter(|s| !s.is_empty()) {
        if seen.insert(rel.to_string()) {
            copy_contained(base, &canon_base, out, rel, false)?;
        }
    }

    git(out, &["init", "-q", "-b", "base"])?;
    git(out, &["add", "-A"])?;
    git(out, &["commit", "-q", "-m", "base"])?;
    Ok(())
}
