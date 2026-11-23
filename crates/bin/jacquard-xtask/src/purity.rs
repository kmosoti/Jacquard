//! `check-core-purity`: the core purity policy of ADR-0000 over every crate
//! the layer policy assigns to `core`.
//!
//! A core crate is `no_std`, forbids unsafe code, inherits workspace lints
//! (`[lints] workspace = true` with `unsafe_code = "forbid"`), has no build
//! script, takes dependencies only from other core crates or the allowlist in
//! `crates/core/PURITY.toml`, keeps all compiled sources inside `src/` without
//! symlinks or escaping `#[path]` / `include*!` directives, and never uses `std`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::finding::Finding;
use crate::layers::Policy;
use crate::purity_lex::tokenize;
use crate::purity_scan::{find_std_use, has_crate_inner_attrs, scan_source_directives};
use crate::util;

/// Every failure code this gate can print.
pub const CODES: &[&str] = &[
    "core-dependency-denied",
    "core-build-script",
    "core-not-no-std",
    "core-unsafe-not-forbidden",
    "core-uses-std",
    "core-source-escapes-src",
    "core-missing-workspace-lints",
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
    /// Whether `[lints] workspace = true` and workspace `unsafe_code = "forbid"` hold.
    pub inherits_workspace_forbid_unsafe: bool,
    /// The library root's source, if the package has a library.
    pub lib_source: Option<String>,
    /// Every compiled source under `src/`, as (path, text).
    pub sources: Vec<(String, String)>,
    /// Any symlink or `#[path]` / `include*!` directive escaping `src/`.
    pub source_escapes: Vec<String>,
}

fn workspace_forbids_unsafe(workspace_manifest: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(workspace_manifest) else {
        return false;
    };
    let Ok(val) = toml::from_str::<toml::Value>(&raw) else {
        return false;
    };
    val.get("workspace")
        .and_then(|w| w.get("lints"))
        .and_then(|l| l.get("rust"))
        .and_then(|r| r.get("unsafe_code"))
        .and_then(toml::Value::as_str)
        == Some("forbid")
}

fn crate_inherits_workspace_lints(manifest_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(manifest_path) else {
        return false;
    };
    let Ok(val) = toml::from_str::<toml::Value>(&raw) else {
        return false;
    };
    val.get("lints")
        .and_then(|l| l.get("workspace"))
        .and_then(toml::Value::as_bool)
        == Some(true)
}

/// Collects the core crates named by the layer policy from `cargo metadata`
/// output, reading their sources.
///
/// # Errors
///
/// Returns a message when a source file cannot be read.
pub fn collect(policy: &Policy, meta: &Value) -> Result<Vec<CoreCrate>, String> {
    let ws_forbids = meta["workspace_root"]
        .as_str()
        .is_some_and(|r| workspace_forbids_unsafe(&Path::new(r).join("Cargo.toml")));

    let mut crates = Vec::new();
    for package in meta["packages"].as_array().into_iter().flatten() {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        if policy.layer_of(name) != Some("core") {
            continue;
        }
        let inherits_workspace_forbid_unsafe = ws_forbids
            && package["manifest_path"]
                .as_str()
                .is_some_and(|p| crate_inherits_workspace_lints(Path::new(p)));
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
        let (lib_source, sources, source_escapes) = match &lib_path {
            Some(path) => {
                let dir = path.parent().unwrap_or(Path::new("."));
                let lib_text = util::read(path)?;
                let (sources, escapes) = collect_sources_and_directives(dir)?;
                (Some(lib_text), sources, escapes)
            }
            None => (None, Vec::new(), Vec::new()),
        };
        crates.push(CoreCrate {
            name: name.to_string(),
            dependencies,
            has_build_script,
            inherits_workspace_forbid_unsafe,
            lib_source,
            sources,
            source_escapes,
        });
    }
    crates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(crates)
}

type CollectedSources = (Vec<(String, String)>, Vec<String>);

fn collect_sources_and_directives(src_dir: &Path) -> Result<CollectedSources, String> {
    let canon_src =
        std::fs::canonicalize(src_dir).map_err(|e| format!("{}: {e}", src_dir.display()))?;
    let mut visited = BTreeSet::new();
    let mut out = Vec::new();
    let mut escapes = Vec::new();
    let mut queue = Vec::new();
    let mut stack = vec![canon_src.clone()];

    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| e.to_string())?.path();
            let meta =
                std::fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if meta.file_type().is_symlink() {
                escapes.push(format!("{}: symlink inside crate src/", path.display()));
                continue;
            }
            if meta.is_dir() {
                let canon =
                    std::fs::canonicalize(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                if canon.starts_with(&canon_src) {
                    stack.push(canon);
                } else {
                    escapes.push(format!("{}: directory escapes src/", path.display()));
                }
            } else if meta.is_file() && path.extension().is_some_and(|e| e == "rs") {
                let canon =
                    std::fs::canonicalize(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                if visited.insert(canon.clone()) {
                    queue.push(canon);
                }
            }
        }
    }

    while let Some(file_path) = queue.pop() {
        let text = util::read(&file_path)?;
        let tokens = tokenize(&text);
        let (dir_escapes, included) = scan_source_directives(&tokens, &file_path, &canon_src);
        for (line, detail) in dir_escapes {
            escapes.push(format!("{}:{line}: {detail}", file_path.display()));
        }
        for inc in included {
            if visited.insert(inc.clone()) {
                queue.push(inc);
            }
        }
        out.push((file_path.display().to_string(), text));
    }

    out.sort();
    escapes.sort();
    Ok((out, escapes))
}

/// The first place a source names `std`, as `(line number, line)`, ignoring
/// comments and string literals. `core::` and `alloc::` do not match.
#[must_use]
pub fn std_use(text: &str) -> Option<(usize, String)> {
    let tokens = tokenize(text);
    find_std_use(&tokens, text)
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
        if !krate.inherits_workspace_forbid_unsafe {
            findings.push(Finding::new(
                "core-missing-workspace-lints",
                format!(
                    "{name}'s Cargo.toml must inherit [lints] workspace = true with unsafe_code = \"forbid\""
                ),
            ));
        }
        let lib = krate.lib_source.as_deref().unwrap_or_default();
        let (has_no_std, has_forbid_unsafe) = has_crate_inner_attrs(&tokenize(lib));
        if !has_no_std {
            findings.push(Finding::new(
                "core-not-no-std",
                format!("{name}'s library root does not declare #![no_std]"),
            ));
        }
        if !has_forbid_unsafe {
            findings.push(Finding::new(
                "core-unsafe-not-forbidden",
                format!("{name}'s library root does not declare #![forbid(unsafe_code)]"),
            ));
        }
        for esc in &krate.source_escapes {
            findings.push(Finding::new(
                "core-source-escapes-src",
                format!("{name}: {esc}"),
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
