//! Unified and `--cc` combined patch parser for `check-trust-boundary`.

use std::collections::BTreeSet;

/// Splits a unified or combined (`--cc`) patch into its changed paths and added lines.
#[must_use]
pub fn parse_patch(patch: &str) -> (Vec<String>, Vec<(String, String)>) {
    let mut paths = BTreeSet::new();
    let mut added = Vec::new();
    let mut current = String::new();
    let mut combined_parents: usize = 0;
    let mut in_hunk = false;
    for line in patch.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            in_hunk = false;
            combined_parents = 0;
            if let Some((a, b)) = rest.split_once(" b/") {
                paths.insert(a.to_string());
                paths.insert(b.to_string());
                current = b.to_string();
            }
        } else if let Some(rest) = line
            .strip_prefix("diff --cc ")
            .or_else(|| line.strip_prefix("diff --combined "))
        {
            in_hunk = false;
            combined_parents = 2;
            paths.insert(rest.to_string());
            current = rest.to_string();
        } else if line.starts_with("@@") {
            in_hunk = true;
            if combined_parents > 0 {
                let ats = line.bytes().take_while(|&b| b == b'@').count();
                combined_parents = ats.saturating_sub(1).max(2);
            }
        } else if in_hunk && combined_parents == 0 {
            if let Some(text) = line.strip_prefix('+') {
                added.push((current.clone(), text.to_string()));
            }
        } else if in_hunk
            && combined_parents > 0
            && line.len() >= combined_parents
            && line.as_bytes()[..combined_parents]
                .iter()
                .all(|&b| b == b'+')
        {
            added.push((current.clone(), line[combined_parents..].to_string()));
        }
    }
    (paths.into_iter().collect(), added)
}
