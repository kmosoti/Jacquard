//! `check-layers`: the dependency rule of ADR-0000 over the declared and the
//! resolved dependency graphs of the workspace.
//!
//! Every workspace crate has a layer in `docs/architecture/layers.json`, and
//! every edge between workspace crates goes from a layer to one it allows.
//! Dev, build, target-specific, optional and renamed dependencies get no
//! exemption: the declared graph catches the ones the resolver leaves out.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::finding::Finding;
use crate::util;

/// Every failure code this gate can print.
pub const CODES: &[&str] = &[
    "unassigned-crate",
    "stale-assignment",
    "unknown-layer",
    "forbidden-edge",
];

/// The layer policy, as `docs/architecture/layers.json` states it.
#[derive(Debug, Deserialize)]
pub struct Policy {
    /// The layer names, innermost first.
    pub layers: Vec<String>,
    /// For each layer, the layers its crates may depend on.
    pub allowed: BTreeMap<String, Vec<String>>,
    /// The layer of every workspace crate.
    pub crates: BTreeMap<String, String>,
}

impl Policy {
    /// Reads the policy from a file.
    ///
    /// # Errors
    ///
    /// Returns a message when the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, String> {
        serde_json::from_str(&util::read(path)?).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The layer a crate is assigned to, if any.
    #[must_use]
    pub fn layer_of(&self, krate: &str) -> Option<&str> {
        self.crates.get(krate).map(String::as_str)
    }
}

/// The workspace's crates and the edges between them.
#[derive(Debug, Default)]
pub struct Graph {
    /// Workspace member names.
    pub members: BTreeSet<String>,
    /// Edges between members, with how the first sighting of each was declared.
    pub edges: BTreeMap<(String, String), String>,
}

impl Graph {
    /// Builds the graph from `cargo metadata --format-version 1` output.
    ///
    /// # Errors
    ///
    /// Returns a message when the metadata lacks packages or members.
    pub fn from_metadata(meta: &Value) -> Result<Self, String> {
        let packages = meta["packages"]
            .as_array()
            .ok_or("metadata has no packages")?;
        let member_ids: BTreeSet<&str> = meta["workspace_members"]
            .as_array()
            .ok_or("metadata has no workspace_members")?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let names: BTreeMap<&str, &str> = packages
            .iter()
            .filter_map(|p| Some((p["id"].as_str()?, p["name"].as_str()?)))
            .collect();
        let mut graph = Graph::default();
        for id in &member_ids {
            if let Some(name) = names.get(id) {
                graph.members.insert((*name).to_string());
            }
        }
        for package in packages {
            let (Some(id), Some(from)) = (package["id"].as_str(), package["name"].as_str()) else {
                continue;
            };
            if !member_ids.contains(id) {
                continue;
            }
            for dep in package["dependencies"].as_array().into_iter().flatten() {
                let to = dep["name"].as_str().unwrap_or_default();
                if !graph.members.contains(to) {
                    continue;
                }
                let mut how = format!("declared {}", dep["kind"].as_str().unwrap_or("normal"));
                if let Some(rename) = dep["rename"].as_str() {
                    let _ = write!(how, " as {rename}");
                }
                if let Some(target) = dep["target"].as_str() {
                    let _ = write!(how, " for {target}");
                }
                if dep["optional"].as_bool() == Some(true) {
                    how.push_str(" optional");
                }
                graph
                    .edges
                    .entry((from.to_string(), to.to_string()))
                    .or_insert(how);
            }
        }
        for node in meta["resolve"]["nodes"].as_array().into_iter().flatten() {
            let Some(from) = node["id"]
                .as_str()
                .filter(|id| member_ids.contains(id))
                .and_then(|id| names.get(id))
            else {
                continue;
            };
            for dep in node["deps"].as_array().into_iter().flatten() {
                let Some(to) = dep["pkg"].as_str().and_then(|id| names.get(id)) else {
                    continue;
                };
                if graph.members.contains(*to) {
                    graph
                        .edges
                        .entry(((*from).to_string(), (*to).to_string()))
                        .or_insert_with(|| "resolved".to_string());
                }
            }
        }
        Ok(graph)
    }

    /// Runs `cargo metadata` on a workspace manifest and builds the graph.
    ///
    /// # Errors
    ///
    /// Returns a message when cargo fails or its output cannot be read.
    pub fn load(manifest: &Path) -> Result<Self, String> {
        Self::from_metadata(&util::metadata(manifest)?)
    }
}

/// Checks the graph against the policy and returns the violations, sorted.
#[must_use]
pub fn check(policy: &Policy, graph: &Graph) -> Vec<Finding> {
    let mut findings = Vec::new();
    let known = |layer: &str| policy.layers.iter().any(|l| l == layer);
    for (krate, layer) in &policy.crates {
        if !known(layer) {
            findings.push(Finding::new(
                "unknown-layer",
                format!("{krate} is assigned to `{layer}`, which is not a layer"),
            ));
        }
    }
    for (layer, allowed) in &policy.allowed {
        for name in std::iter::once(layer).chain(allowed) {
            if !known(name) {
                findings.push(Finding::new(
                    "unknown-layer",
                    format!("`allowed` names `{name}`, which is not a layer"),
                ));
            }
        }
    }
    for member in &graph.members {
        if !policy.crates.contains_key(member) {
            findings.push(Finding::new(
                "unassigned-crate",
                format!("{member} has no layer in layers.json"),
            ));
        }
    }
    for krate in policy.crates.keys() {
        if !graph.members.contains(krate) {
            findings.push(Finding::new(
                "stale-assignment",
                format!("layers.json assigns {krate}, which is not a workspace crate"),
            ));
        }
    }
    for ((from, to), how) in &graph.edges {
        let (Some(from_layer), Some(to_layer)) = (policy.layer_of(from), policy.layer_of(to))
        else {
            continue;
        };
        if !known(from_layer) || !known(to_layer) {
            // Already reported once as unknown-layer.
            continue;
        }
        let allowed = policy
            .allowed
            .get(from_layer)
            .is_some_and(|a| a.iter().any(|l| l == to_layer));
        if !allowed {
            findings.push(Finding::new(
                "forbidden-edge",
                format!("{from} ({from_layer}) -> {to} ({to_layer}), {how}"),
            ));
        }
    }
    findings.sort();
    findings.dedup();
    findings
}

/// Runs the gate on a repository root: its `Cargo.toml` and
/// `docs/architecture/layers.json`.
///
/// # Errors
///
/// Returns a message when the policy or the graph cannot be read.
pub fn run(root: &Path) -> Result<Vec<Finding>, String> {
    let policy = Policy::load(&root.join("docs/architecture/layers.json"))?;
    let graph = Graph::load(&root.join("Cargo.toml"))?;
    Ok(check(&policy, &graph))
}
