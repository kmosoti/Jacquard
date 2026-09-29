//! The filesystem Ledger: each stream is one NDJSON file under a directory,
//! appended and never rewritten. A record is a record of what ran or what was
//! chosen; it is committed, and it is not an artifact.
#![forbid(unsafe_code)]

use std::io::Write as _;
use std::path::{Path, PathBuf};

use jacquard_ports::{Failure, Ledger};

/// A Ledger rooted at a directory.
#[derive(Clone, Debug)]
pub struct FsLedger {
    dir: PathBuf,
}

impl FsLedger {
    /// A Ledger writing `<dir>/<kind>.ndjson`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }

    fn path(&self, kind: &str) -> Result<PathBuf, Failure> {
        let valid = !kind.is_empty()
            && kind
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.');
        if valid && !kind.starts_with('.') {
            Ok(self.dir.join(format!("{kind}.ndjson")))
        } else {
            Err(Failure::Failed(format!("`{kind}` is not a stream name")))
        }
    }
}

impl Ledger for FsLedger {
    fn append(&self, kind: &str, record: &str) -> Result<(), Failure> {
        if record.contains('\n') {
            return Err(Failure::Failed("a record is one line".into()));
        }
        let path = self.path(kind)?;
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| Failure::Failed(format!("{}: {e}", self.dir.display())))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| Failure::Failed(format!("{}: {e}", path.display())))?;
        writeln!(file, "{record}").map_err(|e| Failure::Failed(format!("{}: {e}", path.display())))
    }

    fn read(&self, kind: &str) -> Result<Vec<String>, Failure> {
        let path = self.path(kind)?;
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(text
                .lines()
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(Failure::Failed(format!("{}: {e}", path.display()))),
        }
    }
}
