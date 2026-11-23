//! Safe Swatch directory loader for the `jacquard` CLI.
//!
//! Enforces physical containment without following symbolic links, rejects
//! non-regular file types, and bounds total file count and byte volume before
//! buffering files into memory.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use jacquard_ports::{MAX_SWATCH_BYTES, MAX_SWATCH_FILES, SwatchSource};

#[cfg(unix)]
fn file_identity(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    (meta.dev(), meta.ino())
}

#[cfg(not(unix))]
fn file_identity(_meta: &std::fs::Metadata) -> (u64, u64) {
    (0, 0)
}

fn ensure_contained_dir(dir: &Path, canon_root: &Path) -> Result<(PathBuf, (u64, u64)), String> {
    let meta = std::fs::symlink_metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(format!(
            "{}: symlinks and non-directories are not allowed",
            dir.display()
        ));
    }
    let canon = std::fs::canonicalize(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if !canon.starts_with(canon_root) {
        return Err(format!(
            "{}: resolved directory escapes swatch root",
            dir.display()
        ));
    }
    let canon_meta =
        std::fs::symlink_metadata(&canon).map_err(|e| format!("{}: {e}", canon.display()))?;
    if file_identity(&meta) != file_identity(&canon_meta) {
        return Err(format!("{}: directory changed during load", dir.display()));
    }
    Ok((canon, file_identity(&meta)))
}

fn read_regular_contained(
    path: &Path,
    meta: &std::fs::Metadata,
    canon_root: &Path,
    remaining_bytes: usize,
) -> Result<Vec<u8>, String> {
    let file_len = usize::try_from(meta.len())
        .map_err(|_| format!("{}: file size exceeds address space", path.display()))?;
    if file_len > remaining_bytes {
        return Err(format!(
            "{}: swatch total size exceeds limit {MAX_SWATCH_BYTES} bytes",
            path.display()
        ));
    }
    let canon = std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !canon.starts_with(canon_root) {
        return Err(format!(
            "{}: resolved file escapes swatch root",
            path.display()
        ));
    }
    let file = std::fs::File::open(&canon).map_err(|e| format!("{}: {e}", path.display()))?;
    let opened_meta = file
        .metadata()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !opened_meta.is_file() || file_identity(meta) != file_identity(&opened_meta) {
        return Err(format!("{}: file changed during load", path.display()));
    }
    let cap = u64::try_from(remaining_bytes).unwrap_or(u64::MAX);
    let mut bytes = Vec::with_capacity(file_len);
    file.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > remaining_bytes {
        return Err(format!(
            "{}: swatch total size exceeds limit {MAX_SWATCH_BYTES} bytes",
            path.display()
        ));
    }
    Ok(bytes)
}

/// Loads a Swatch directory safely without following symlinks or exceeding budgets.
///
/// # Errors
///
/// Fails if any directory or file is a symlink, non-regular file, escapes `dir`,
/// exceeds [`MAX_SWATCH_FILES`] or [`MAX_SWATCH_BYTES`], or fails [`SwatchSource::validate`].
pub fn load(dir: &Path) -> Result<SwatchSource, String> {
    let root_meta =
        std::fs::symlink_metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if root_meta.file_type().is_symlink() || !root_meta.is_dir() {
        return Err(format!(
            "{}: swatch root must be a real directory, not a symlink",
            dir.display()
        ));
    }
    let canon_root = std::fs::canonicalize(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let _ = ensure_contained_dir(dir, &canon_root)?;

    let mut files = BTreeMap::new();
    let mut total_bytes: usize = 0;
    let mut stack = vec![(canon_root.clone(), String::new())];
    while let Some((cur_dir, prefix)) = stack.pop() {
        let mut entries: Vec<_> = std::fs::read_dir(&cur_dir)
            .map_err(|e| format!("{}: {e}", cur_dir.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("{}: {e}", cur_dir.display()))?;
        entries.sort_by_key(std::fs::DirEntry::file_name);

        for entry in entries {
            let path = entry.path();
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| format!("{}: non-UTF-8 path in swatch", path.display()))?;
            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            SwatchSource::validate_path(&rel).map_err(|e| e.to_string())?;

            let meta =
                std::fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if meta.file_type().is_symlink() {
                return Err(format!(
                    "{}: symlinks are not allowed in swatch directory",
                    path.display()
                ));
            }
            if meta.is_dir() {
                let (canon_sub, _) = ensure_contained_dir(&path, &canon_root)?;
                stack.push((canon_sub, rel));
            } else if meta.file_type().is_file() {
                if files.len() >= MAX_SWATCH_FILES {
                    return Err(format!(
                        "{}: swatch contains more than {MAX_SWATCH_FILES} files",
                        dir.display()
                    ));
                }
                let remaining = MAX_SWATCH_BYTES.saturating_sub(total_bytes);
                let bytes = read_regular_contained(&path, &meta, &canon_root, remaining)?;
                total_bytes += bytes.len();
                files.insert(rel, bytes);
            } else {
                return Err(format!(
                    "{}: non-regular file is not allowed in swatch directory",
                    path.display()
                ));
            }
        }
    }

    let source = SwatchSource {
        files,
        entry: "index.html".into(),
    };
    source.validate().map_err(|e| e.to_string())?;
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp_dir(label: &str) -> Cleanup {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "jacquard-loader-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        Cleanup(dir)
    }

    #[test]
    fn loads_valid_swatch_tree() {
        let dir = temp_dir("valid");
        std::fs::create_dir_all(dir.0.join("css")).expect("mkdir");
        std::fs::write(dir.0.join("index.html"), b"<p>ok</p>").expect("write");
        std::fs::write(dir.0.join("css/app.css"), b"p{}").expect("write");
        let source = load(&dir.0).expect("valid swatch");
        assert_eq!(source.entry, "index.html");
        assert_eq!(source.files.len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_file_and_dir_symlinks_and_sockets() {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;

        let outside = temp_dir("outside");
        std::fs::write(outside.0.join("secret"), b"TOP_SECRET").expect("write");

        let with_file_link = temp_dir("file-link");
        std::fs::write(with_file_link.0.join("index.html"), b"<p>ok</p>").expect("write");
        symlink(outside.0.join("secret"), with_file_link.0.join("leak.txt")).expect("symlink");
        assert!(load(&with_file_link.0).is_err());

        let with_dir_link = temp_dir("dir-link");
        std::fs::write(with_dir_link.0.join("index.html"), b"<p>ok</p>").expect("write");
        symlink(&outside.0, with_dir_link.0.join("sub")).expect("dir symlink");
        assert!(load(&with_dir_link.0).is_err());

        let with_socket = temp_dir("socket");
        std::fs::write(with_socket.0.join("index.html"), b"<p>ok</p>").expect("write");
        let _sock = UnixListener::bind(with_socket.0.join("sock")).expect("unix socket");
        assert!(load(&with_socket.0).is_err());
    }

    #[test]
    fn rejects_exceeding_file_count_and_byte_limits() {
        let many = temp_dir("many");
        std::fs::write(many.0.join("index.html"), b"<p>ok</p>").expect("write");
        for i in 0..MAX_SWATCH_FILES {
            std::fs::write(many.0.join(format!("f-{i}.txt")), b"x").expect("write");
        }
        assert!(load(&many.0).is_err());

        let huge = temp_dir("huge");
        std::fs::write(huge.0.join("index.html"), b"<p>ok</p>").expect("write");
        let file = std::fs::File::create(huge.0.join("big.bin")).expect("create");
        file.set_len(u64::try_from(MAX_SWATCH_BYTES + 1).expect("len"))
            .expect("set_len");
        assert!(load(&huge.0).is_err());
    }
}
