use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::model::{CrateInfo, ItemKind, Module};

/// Generate Layer 2: Relationships (relationships.md)
///
/// Includes trait implementation map, error chains, module dependencies,
/// and type usage hotspots.
pub fn generate_relationships(crates: &[CrateInfo]) -> String {
    let mut out = String::new();

    // Collect all data across crates
    let mut trait_impls: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut from_impls: BTreeSet<(String, String)> = BTreeSet::new();
    let mut module_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut type_usage: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for crate_info in crates {
        collect_relationships(
            &crate_info.root_module,
            &mut trait_impls,
            &mut from_impls,
            &mut module_deps,
            &mut type_usage,
        );
    }

    // Section 1: Trait Implementation Map
    out.push_str("## Trait Implementations\n\n");
    if trait_impls.is_empty() {
        out.push_str("(none found)\n\n");
    } else {
        // Find the longest trait name for alignment
        let max_len = trait_impls.keys().map(|k| k.len()).max().unwrap_or(0);
        for (trait_name, implementors) in &trait_impls {
            let impls: Vec<&str> = implementors.iter().map(|s| s.as_str()).collect();
            out.push_str(&format!(
                "{:<width$} <- {}\n",
                trait_name,
                impls.join(", "),
                width = max_len
            ));
        }
        out.push('\n');
    }

    // Section 2: Error Chains
    out.push_str("## Error Chains\n\n");
    let from_impls_vec: Vec<_> = from_impls.into_iter().collect();
    let error_chains = build_error_chains(&from_impls_vec);
    if error_chains.is_empty() {
        out.push_str("(no From impls found)\n\n");
    } else {
        for chain in &error_chains {
            out.push_str(&format!("{}\n", chain));
        }
        out.push('\n');
    }

    // Section 3: Module Dependencies
    out.push_str("## Module Dependencies\n\n");
    if module_deps.is_empty() {
        out.push_str("(none found)\n\n");
    } else {
        let max_len = module_deps.keys().map(|k| k.len()).max().unwrap_or(0);
        for (module, deps) in &module_deps {
            if deps.is_empty() {
                out.push_str(&format!(
                    "{:<width$} -> (no internal deps)\n",
                    module,
                    width = max_len
                ));
            } else {
                let dep_list: Vec<&str> = deps.iter().map(|s| s.as_str()).collect();
                out.push_str(&format!(
                    "{:<width$} -> {}\n",
                    module,
                    dep_list.join(", "),
                    width = max_len
                ));
            }
        }
        out.push('\n');
    }

    // Section 4: Type Usage Hotspots
    out.push_str("## Key Types (referenced from 3+ modules)\n\n");
    let mut hotspots: Vec<(&String, usize)> = type_usage
        .iter()
        .filter(|(_, modules)| modules.len() >= 3)
        .map(|(ty, modules)| (ty, modules.len()))
        .collect();
    hotspots.sort_by(|a, b| b.1.cmp(&a.1));

    if hotspots.is_empty() {
        out.push_str("(no types referenced from 3+ modules)\n\n");
    } else {
        let max_len = hotspots.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
        for (type_name, count) in &hotspots {
            out.push_str(&format!(
                "{:<width$} — used in {} modules\n",
                type_name,
                count,
                width = max_len
            ));
        }
        out.push('\n');
    }

    out
}

fn collect_relationships(
    module: &Module,
    trait_impls: &mut BTreeMap<String, BTreeSet<String>>,
    from_impls: &mut BTreeSet<(String, String)>,
    module_deps: &mut BTreeMap<String, BTreeSet<String>>,
    type_usage: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let mod_short = module
        .path
        .strip_prefix("crate::")
        .unwrap_or(&module.path)
        .to_string();

    // Initialize module deps entry
    module_deps.entry(mod_short.clone()).or_default();

    for item in &module.items {
        // Collect trait implementations
        if let ItemKind::Impl {
            ref self_ty,
            ref trait_name,
        } = item.kind
        {
            if let Some(ref tn) = trait_name {
                let clean_trait = clean_type_name(tn);
                let clean_self = clean_type_name(self_ty);

                trait_impls
                    .entry(clean_trait.clone())
                    .or_default()
                    .insert(clean_self.clone());

                // Track From impls for error chains
                if clean_trait.starts_with("From") {
                    // Extract the source type from From<SourceType>
                    if let Some(source) = extract_from_source(tn) {
                        from_impls.insert((source, clean_self));
                    }
                }
            }
        }

        // Track type references for hotspot analysis
        // We approximate this by looking at type names mentioned in signatures
        let types_in_sig = extract_type_names_from_signature(&item.signature);
        for ty in types_in_sig {
            type_usage.entry(ty).or_default().insert(mod_short.clone());
        }
    }

    // Collect module dependencies from use statements
    for use_path in &module.use_statements {
        if let Some(dep_mod) = extract_internal_module_dep(use_path) {
            if dep_mod != mod_short && !dep_mod.is_empty() {
                module_deps
                    .entry(mod_short.clone())
                    .or_default()
                    .insert(dep_mod);
            }
        }
    }

    // Recurse into submodules
    for sub in &module.submodules {
        collect_relationships(sub, trait_impls, from_impls, module_deps, type_usage);
    }
}

/// Clean a type name by removing generics and whitespace
fn clean_type_name(name: &str) -> String {
    // Remove leading/trailing whitespace
    let name = name.trim();

    // For simple names without generics, just return
    if !name.contains('<') {
        return name.to_string();
    }

    // For names with generics, keep the full form but clean whitespace
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract the source type from a From<T> trait name
fn extract_from_source(trait_str: &str) -> Option<String> {
    let trimmed = trait_str.trim();
    if trimmed.starts_with("From") {
        if let Some(start) = trimmed.find('<') {
            if let Some(end) = trimmed.rfind('>') {
                let inner = trimmed[start + 1..end].trim();
                return Some(clean_type_name(inner));
            }
        }
    }
    None
}

/// Build error chain strings from From impls
fn build_error_chains(from_impls: &[(String, String)]) -> Vec<String> {
    if from_impls.is_empty() {
        return Vec::new();
    }

    // Build a graph: source -> targets (what can be converted to)
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for (source, target) in from_impls {
        graph.entry(source.clone()).or_default().push(target.clone());
    }

    // Find chain starts (types that are sources but not targets)
    let targets: HashSet<&String> = from_impls.iter().map(|(_, t)| t).collect();
    let sources: HashSet<&String> = from_impls.iter().map(|(s, _)| s).collect();

    let mut starts: Vec<&String> = sources.difference(&targets).copied().collect();
    starts.sort();

    let mut chains = Vec::new();
    let mut visited = HashSet::new();

    for start in starts {
        let mut chain = vec![start.clone()];
        visited.insert(start.clone());
        follow_chain(&graph, start, &mut chain, &mut visited, &mut chains);
    }

    // Also output any remaining cycles or disconnected edges
    for (source, target) in from_impls {
        if !visited.contains(source) {
            chains.push(format!("{} -> {}", source, target));
            visited.insert(source.clone());
        }
    }

    chains
}

/// Follow a chain from current node to its end, outputting the complete chain
fn follow_chain(
    graph: &HashMap<String, Vec<String>>,
    current: &str,
    chain: &mut Vec<String>,
    visited: &mut HashSet<String>,
    results: &mut Vec<String>,
) {
    let nexts = match graph.get(current) {
        Some(n) => n.clone(),
        None => {
            // End of chain — output it
            if chain.len() > 1 {
                results.push(chain.join(" -> "));
            }
            return;
        }
    };

    let mut any_followed = false;
    for next in &nexts {
        if !visited.contains(next) {
            any_followed = true;
            chain.push(next.clone());
            visited.insert(next.clone());
            follow_chain(graph, next, chain, visited, results);
            chain.pop();
        }
    }

    // If all neighbors were already visited, this is the end of the chain
    if !any_followed && chain.len() > 1 {
        results.push(chain.join(" -> "));
    }
}

/// Extract internal module dependency from a use path
fn extract_internal_module_dep(use_path: &str) -> Option<String> {
    if use_path.starts_with("crate::") {
        let parts: Vec<&str> = use_path
            .strip_prefix("crate::")
            .unwrap()
            .split("::")
            .collect();
        // The module is everything except the last segment (which is the item name)
        if parts.len() >= 2 {
            Some(parts[..parts.len() - 1].join("::"))
        } else if parts.len() == 1 {
            Some(parts[0].to_string())
        } else {
            None
        }
    } else if use_path.starts_with("super::") {
        // Handle relative imports — extract the module portion
        let parts: Vec<&str> = use_path.split("::").collect();
        // "super::ItemName" -> just "super" (the parent module)
        // "super::submod::ItemName" -> "super::submod"
        if parts.len() >= 2 {
            // If last segment starts with uppercase or is *, it's an item, not a module
            let last = parts.last().unwrap();
            if last.chars().next().map_or(false, |c| c.is_uppercase()) || *last == "*" {
                if parts.len() > 2 {
                    Some(parts[..parts.len() - 1].join("::"))
                } else {
                    Some("super".to_string())
                }
            } else {
                Some(use_path.to_string())
            }
        } else {
            Some("super".to_string())
        }
    } else {
        None // external crate import
    }
}

/// Extract type names from a signature string (heuristic)
fn extract_type_names_from_signature(sig: &str) -> Vec<String> {
    let mut types = Vec::new();

    // Simple heuristic: find capitalized words that look like type names
    for word in sig.split(|c: char| !c.is_alphanumeric() && c != '_') {
        let trimmed = word.trim();
        if !trimmed.is_empty()
            && trimmed.chars().next().map_or(false, |c| c.is_uppercase())
            && trimmed.len() > 1
            && !is_keyword(trimmed)
        {
            types.push(trimmed.to_string());
        }
    }

    types
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "Self" | "String" | "Vec" | "Box" | "Option" | "Result" | "Ok" | "Err" | "Some" | "None"
            | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" | "Rc" | "Arc" | "Mutex"
            | "RwLock" | "Pin" | "Cow" | "PhantomData" | "Where" | "Fn" | "FnMut" | "FnOnce"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_type_name() {
        assert_eq!(clean_type_name("  MyType  "), "MyType");
        assert_eq!(clean_type_name("From<Error>"), "From<Error>");
    }

    #[test]
    fn test_extract_from_source() {
        assert_eq!(
            extract_from_source("From<IoError>"),
            Some("IoError".to_string())
        );
        assert_eq!(
            extract_from_source("From < std::io::Error >"),
            Some("std::io::Error".to_string())
        );
        assert_eq!(extract_from_source("Display"), None);
    }

    #[test]
    fn test_extract_internal_module_dep() {
        assert_eq!(
            extract_internal_module_dep("crate::engine::eval::Value"),
            Some("engine::eval".to_string())
        );
        assert_eq!(
            extract_internal_module_dep("crate::model::Item"),
            Some("model".to_string())
        );
        assert_eq!(extract_internal_module_dep("std::collections::HashMap"), None);
    }

    #[test]
    fn test_extract_type_names() {
        let sig = "pub fn evaluate(expr: &Expr, ctx: &mut EvalContext) -> Result<Value, EvalError>;";
        let types = extract_type_names_from_signature(sig);
        assert!(types.contains(&"Expr".to_string()));
        assert!(types.contains(&"EvalContext".to_string()));
        assert!(types.contains(&"EvalError".to_string()));
    }

    #[test]
    fn test_build_error_chains() {
        let from_impls = vec![
            ("IoError".to_string(), "ConfigError".to_string()),
            ("ConfigError".to_string(), "AppError".to_string()),
        ];
        let chains = build_error_chains(&from_impls);
        assert!(!chains.is_empty());
        // Should find IoError -> ConfigError -> AppError
        assert!(chains.iter().any(|c| c.contains("IoError") && c.contains("AppError")));
    }
}
