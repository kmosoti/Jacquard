//! `check-core-purity`: the core purity policy of ADR-0000 over every crate
//! the layer policy assigns to `core`.
//!
//! A core crate is `no_std`, forbids unsafe code, has no build script, and
//! takes dependencies only from other core crates or the allowlist in
//! `crates/core/PURITY.toml`, for every dependency kind. Its sources never
//! name `std`. What the compiler cannot see (reading the clock through an
//! allowlisted crate, for example) remains a review matter; this gate says
//! so rather than pretending otherwise.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::finding::Finding;
use crate::layers::Policy;
use crate::util;

/// Every failure code this gate can print.
pub const CODES: &[&str] = &[
    "core-dependency-denied",
    "core-build-script",
    "core-not-no-std",
    "core-unsafe-not-forbidden",
    "core-uses-std",
];

/// `crates/core/PURITY.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Purity {
    /// What a core crate may depend on beyond other core crates.
    #[serde(default)]
    pub allow: Allow,
}

/// The allowlist.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allow {
    /// Crate names a core crate may depend on.
    #[serde(default)]
    pub crates: Vec<String>,
}

impl Purity {
    /// Reads the purity policy.
    ///
    /// # Errors
    ///
    /// Returns a message when the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, String> {
        toml::from_str(&util::read(path)?).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// What the gate needs to know about one core crate.
#[derive(Debug)]
pub struct CoreCrate {
    /// Package name.
    pub name: String,
    /// Dependencies as (name, kind).
    pub dependencies: Vec<(String, String)>,
    /// Whether the package has a build script.
    pub has_build_script: bool,
    /// The library root's source, if the package has a library.
    pub lib_source: Option<String>,
    /// Every `.rs` file under the library's source directory, as (path, text).
    pub sources: Vec<(String, String)>,
}

/// Collects the core crates named by the layer policy from `cargo metadata`
/// output, reading their sources.
///
/// # Errors
///
/// Returns a message when a source file cannot be read.
pub fn collect(policy: &Policy, meta: &Value) -> Result<Vec<CoreCrate>, String> {
    let mut crates = Vec::new();
    for package in meta["packages"].as_array().into_iter().flatten() {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        if policy.layer_of(name) != Some("core") {
            continue;
        }
        let dependencies = package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| {
                Some((
                    d["name"].as_str()?.to_string(),
                    d["kind"].as_str().unwrap_or("normal").to_string(),
                ))
            })
            .collect();
        let targets: Vec<&Value> = package["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        let has_kind = |t: &Value, kind: &str| {
            t["kind"]
                .as_array()
                .is_some_and(|k| k.iter().any(|x| x.as_str() == Some(kind)))
        };
        let has_build_script = targets.iter().any(|t| has_kind(t, "custom-build"));
        let lib_path = targets
            .iter()
            .find(|t| has_kind(t, "lib"))
            .and_then(|t| t["src_path"].as_str())
            .map(PathBuf::from);
        let (lib_source, sources) = match &lib_path {
            Some(path) => {
                let dir = path.parent().unwrap_or(Path::new("."));
                (Some(util::read(path)?), read_sources(dir)?)
            }
            None => (None, Vec::new()),
        };
        crates.push(CoreCrate {
            name: name.to_string(),
            dependencies,
            has_build_script,
            lib_source,
            sources,
        });
    }
    crates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(crates)
}

fn read_sources(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push((path.display().to_string(), util::read(&path)?));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The first place a source names `std`, as (line number, line), skipping
/// comment lines. `core::` and `alloc::` do not match.
#[must_use]
pub fn std_use(text: &str) -> Option<(usize, String)> {
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        let names_std = trimmed.contains("extern crate std")
            || trimmed.match_indices("std::").any(|(at, _)| {
                at == 0
                    || !trimmed[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_')
            });
        if names_std {
            return Some((index + 1, trimmed.to_string()));
        }
    }
    None
}

fn has_attribute(source: &str, accepted: &[&str]) -> bool {
    source.lines().any(|l| accepted.contains(&l.trim()))
}

/// Checks the core crates against the purity policy, returning sorted findings.
#[must_use]
pub fn check(purity: &Purity, core: &BTreeSet<String>, crates: &[CoreCrate]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for krate in crates {
        let name = &krate.name;
        for (dep, kind) in &krate.dependencies {
            if !core.contains(dep) && !purity.allow.crates.contains(dep) {
                findings.push(Finding::new(
                    "core-dependency-denied",
                    format!(
                        "{name} depends on {dep} ({kind}), which is neither core nor allowlisted"
                    ),
                ));
            }
        }
        if krate.has_build_script {
            findings.push(Finding::new(
                "core-build-script",
                format!("{name} has a build script"),
            ));
        }
        let lib = krate.lib_source.as_deref().unwrap_or_default();
        if !has_attribute(lib, &["#![no_std]", "#![cfg_attr(not(test), no_std)]"]) {
            findings.push(Finding::new(
                "core-not-no-std",
                format!("{name}'s library root does not declare #![no_std]"),
            ));
        }
        if !has_attribute(lib, &["#![forbid(unsafe_code)]"]) {
            findings.push(Finding::new(
                "core-unsafe-not-forbidden",
                format!("{name}'s library root does not declare #![forbid(unsafe_code)]"),
            ));
        }
        for (path, text) in &krate.sources {
            if let Some((line, code)) = std_use(text) {
                findings.push(Finding::new(
                    "core-uses-std",
                    format!("{name}: {path}:{line}: {code}"),
                ));
            }
        }
    }
    findings.sort();
    findings
}

/// Runs the gate on a repository root.
///
/// # Errors
///
/// Returns a message when a policy, the metadata or a source cannot be read.
pub fn run(root: &Path) -> Result<Vec<Finding>, String> {
    let policy = Policy::load(&root.join("docs/architecture/layers.json"))?;
    let purity = Purity::load(&root.join("crates/core/PURITY.toml"))?;
    let meta = util::metadata(&root.join("Cargo.toml"))?;
    let crates = collect(&policy, &meta)?;
    let core = policy
        .crates
        .iter()
        .filter(|(_, layer)| layer.as_str() == "core")
        .map(|(name, _)| name.clone())
        .collect();
    let mut findings = check(&purity, &core, &crates);
    // Paths in details are absolute; show them relative to the root.
    let prefix = format!("{}/", root.display());
    for finding in &mut findings {
        finding.detail = finding.detail.replace(&prefix, "");
    }
    Ok(findings)
}
