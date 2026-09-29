//! The check registry, `verification/checks.toml`. A Receipt names one of
//! these checks, and its command must begin with the registered command up
//! to the first `<placeholder>`. Adding a check is a verifier change.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::util;

/// The registry.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// Checks by id.
    pub checks: BTreeMap<String, Check>,
}

/// One registered check.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// The command, words separated by single spaces, with `<placeholders>`.
    pub command: String,
    /// What a pass establishes.
    pub what: String,
    /// What a pass does not establish.
    pub unchecked: String,
    /// The profile that runs it: `fast` runs in CI on every push.
    pub profile: String,
}

impl Registry {
    /// Reads the registry.
    ///
    /// # Errors
    ///
    /// Returns a message when the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, String> {
        toml::from_str(&util::read(path)?).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Whether `command` is a use of the registered check `id`: equal to it
    /// when it has no placeholder, otherwise beginning with the text before
    /// the first placeholder and supplying something after it.
    ///
    /// # Errors
    ///
    /// Returns `receipt-unregistered-check` or `receipt-command-mismatch`.
    pub fn admits(&self, id: &str, command: &str) -> Result<(), &'static str> {
        let check = self.checks.get(id).ok_or("receipt-unregistered-check")?;
        let ok = match check.command.split_once('<') {
            None => command == check.command,
            Some((prefix, _)) => command.starts_with(prefix) && command.len() > prefix.len(),
        };
        if ok {
            Ok(())
        } else {
            Err("receipt-command-mismatch")
        }
    }
}
