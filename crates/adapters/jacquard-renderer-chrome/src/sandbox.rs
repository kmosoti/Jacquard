//! Scratch containment, private directory creation, and cryptographic nonces.

use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Read as _, Write as _};
use std::path::{Path, PathBuf};

use jacquard_ports::{Failure, SwatchSource};

/// RAII temporary directory removed when dropped.
pub struct TempDir(pub PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Creates a private directory (`0700` on Unix), rejecting pre-existing symlinks.
///
/// # Errors
///
/// Returns [`Failure::Failed`] if creation fails or the target is a symlink.
pub fn create_private_dir(path: &Path) -> Result<(), Failure> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
    }
    builder
        .create(path)
        .map_err(|e| Failure::Failed(format!("{}: {e}", path.display())))?;
    let meta = std::fs::symlink_metadata(path)
        .map_err(|e| Failure::Failed(format!("{}: {e}", path.display())))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(Failure::Failed(format!(
            "{}: scratch directory must not be a symlink",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| Failure::Failed(format!("{}: {e}", path.display())))?;
    }
    Ok(())
}

/// Generates a 128-bit cryptographic random lowercase hex nonce from `/dev/urandom`.
///
/// # Errors
///
/// Returns [`Failure::Unavailable`] if cryptographic randomness cannot be read.
pub fn generate_nonce() -> Result<String, Failure> {
    let bytes = read_urandom::<16>()?;
    let mut hex = String::with_capacity(32);
    for b in bytes {
        let _ = write!(hex, "{b:02x}");
    }
    Ok(hex)
}

/// Generates a 256-bit cryptographic random key from `/dev/urandom`.
///
/// # Errors
///
/// Returns [`Failure::Unavailable`] if cryptographic randomness cannot be read.
pub fn generate_secret_bytes() -> Result<[u8; 32], Failure> {
    read_urandom::<32>()
}

fn read_urandom<const N: usize>() -> Result<[u8; N], Failure> {
    let mut file = std::fs::File::open("/dev/urandom")
        .map_err(|e| Failure::Unavailable(format!("/dev/urandom: {e}")))?;
    let mut bytes = [0u8; N];
    file.read_exact(&mut bytes)
        .map_err(|e| Failure::Unavailable(format!("/dev/urandom: {e}")))?;
    Ok(bytes)
}

/// Rejects any candidate Swatch whose files contain any per-render secret string.
///
/// # Errors
///
/// Returns [`Failure::Failed`] if any file contains any secret in `secrets`
/// (ASCII case-insensitive).
pub fn ensure_no_secrets(swatch: &SwatchSource, secrets: &[&str]) -> Result<(), Failure> {
    for (path, bytes) in &swatch.files {
        for secret in secrets {
            let needle = secret.as_bytes();
            if needle.is_empty() {
                continue;
            }
            if bytes
                .windows(needle.len())
                .any(|w| w.eq_ignore_ascii_case(needle))
            {
                return Err(Failure::Failed(format!(
                    "candidate file {path} contains reserved probe secret"
                )));
            }
        }
    }
    Ok(())
}

/// Materializes a validated [`SwatchSource`] inside `swatch_dir` without
/// following symlinks, verifying canonical physical containment beneath `swatch_dir`.
///
/// # Errors
///
/// Returns [`Failure::Failed`] if validation fails, any symlink is encountered,
/// or a resolved path escapes `swatch_dir`.
pub fn materialize_contained(swatch: &SwatchSource, swatch_dir: &Path) -> Result<(), Failure> {
    swatch.validate()?;
    create_private_dir(swatch_dir)?;
    let canon_root = std::fs::canonicalize(swatch_dir)
        .map_err(|e| Failure::Failed(format!("{}: {e}", swatch_dir.display())))?;

    for (rel, bytes) in &swatch.files {
        let parts: Vec<&str> = rel.split('/').collect();
        let mut cur = swatch_dir.to_path_buf();
        for dir_seg in &parts[..parts.len() - 1] {
            cur.push(dir_seg);
            match std::fs::symlink_metadata(&cur) {
                Ok(meta) => {
                    if meta.file_type().is_symlink() || !meta.is_dir() {
                        return Err(Failure::Failed(format!(
                            "{}: parent path is a symlink or non-directory",
                            cur.display()
                        )));
                    }
                }
                Err(e) if e.kind() == ErrorKind::NotFound => {
                    let mut db = std::fs::DirBuilder::new();
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::DirBuilderExt as _;
                        db.mode(0o700);
                    }
                    db.create(&cur)
                        .map_err(|err| Failure::Failed(format!("{}: {err}", cur.display())))?;
                }
                Err(e) => return Err(Failure::Failed(format!("{}: {e}", cur.display()))),
            }
            let canon_parent = std::fs::canonicalize(&cur)
                .map_err(|e| Failure::Failed(format!("{}: {e}", cur.display())))?;
            if !canon_parent.starts_with(&canon_root) {
                return Err(Failure::Failed(format!(
                    "{}: directory escapes swatch scratch root",
                    cur.display()
                )));
            }
        }

        let file_name = parts[parts.len() - 1];
        cur.push(file_name);
        if std::fs::symlink_metadata(&cur).is_ok() {
            return Err(Failure::Failed(format!(
                "{}: duplicate or pre-existing target path",
                cur.display()
            )));
        }
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            opts.mode(0o600);
        }
        let mut out = opts
            .open(&cur)
            .map_err(|e| Failure::Failed(format!("{}: {e}", cur.display())))?;
        out.write_all(bytes)
            .map_err(|e| Failure::Failed(format!("{}: {e}", cur.display())))?;
        let canon_file = std::fs::canonicalize(&cur)
            .map_err(|e| Failure::Failed(format!("{}: {e}", cur.display())))?;
        if !canon_file.starts_with(&canon_root) {
            return Err(Failure::Failed(format!(
                "{}: file escapes swatch scratch root",
                cur.display()
            )));
        }
    }

    let entry_path = swatch_dir.join(&swatch.entry);
    let entry_meta = std::fs::symlink_metadata(&entry_path)
        .map_err(|_| Failure::Failed(format!("entry {} is not among the files", swatch.entry)))?;
    if entry_meta.file_type().is_symlink() || !entry_meta.is_file() {
        return Err(Failure::Failed(format!(
            "entry {} is not a regular file",
            swatch.entry
        )));
    }
    let canon_entry = std::fs::canonicalize(&entry_path)
        .map_err(|e| Failure::Failed(format!("{}: {e}", entry_path.display())))?;
    if !canon_entry.starts_with(&canon_root) {
        return Err(Failure::Failed(format!(
            "entry {} escapes swatch scratch root",
            swatch.entry
        )));
    }
    Ok(())
}
