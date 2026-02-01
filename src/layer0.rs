use crate::annotations::AnnotationStore;
use crate::model::{CrateInfo, Module};
use crate::output;

/// Generate Layer 0: Overview (overview.md)
///
/// Contains crate info, module tree with descriptions, and token estimates.
pub fn generate_overview(crates: &[CrateInfo], annotations: &AnnotationStore) -> String {
    let mut out = String::new();

    for crate_info in crates {
        out.push_str(&format!(
            "# Crate: {} ({})\n",
            crate_info.name, crate_info.kind
        ));
        out.push_str(&format!("Edition: {}\n", crate_info.edition));
        out.push_str(&format!("Version: {}\n", crate_info.version));

        if !crate_info.external_deps.is_empty() {
            out.push_str(&format!(
                "External deps: {}\n",
                crate_info.external_deps.join(", ")
            ));
        }

        out.push_str("\n## Module Tree\n");
        write_module_tree(&mut out, &crate_info.root_module, 0, annotations);

        out.push('\n');
    }

    out
}

fn write_module_tree(
    out: &mut String,
    module: &Module,
    depth: usize,
    annotations: &AnnotationStore,
) {
    let description = get_module_description(module, annotations);
    let entry = output::tree_entry(&module.path, &description, depth);
    out.push_str(&entry);
    out.push('\n');

    for sub in &module.submodules {
        write_module_tree(out, sub, depth + 1, annotations);
    }
}

/// Get module description from various sources (priority order):
/// 1. Inner doc comment (//!)
/// 2. Annotation
/// 3. Empty placeholder
fn get_module_description(module: &Module, annotations: &AnnotationStore) -> String {
    // Priority 1: Inner doc comment
    if let Some(ref doc) = module.doc_comment {
        // Take only the first line/sentence
        let first_line = doc.lines().next().unwrap_or("");
        let trimmed = first_line.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    // Priority 2: Annotation
    if let Some(entry) = annotations.modules.get(&module.path) {
        if !entry.note.is_empty() {
            return entry.note.clone();
        }
    }

    // Priority 3: Empty
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::path::PathBuf;

    fn sample_crate() -> CrateInfo {
        CrateInfo {
            name: "test_crate".to_string(),
            kind: CrateKind::Lib,
            edition: "2021".to_string(),
            version: "0.1.0".to_string(),
            external_deps: vec!["serde".to_string(), "tokio".to_string()],
            root_module: Module {
                path: "crate".to_string(),
                file_path: PathBuf::from("src/lib.rs"),
                file_hash: "abc123".to_string(),
                doc_comment: Some("Main library crate".to_string()),
                visibility: Visibility::Pub,
                items: vec![],
                submodules: vec![
                    Module {
                        path: "crate::config".to_string(),
                        file_path: PathBuf::from("src/config.rs"),
                        file_hash: "def456".to_string(),
                        doc_comment: Some("Configuration module".to_string()),
                        visibility: Visibility::Pub,
                        items: vec![],
                        submodules: vec![],
                        use_statements: vec![],
                        is_inline: false,
                    },
                    Module {
                        path: "crate::engine".to_string(),
                        file_path: PathBuf::from("src/engine/mod.rs"),
                        file_hash: "ghi789".to_string(),
                        doc_comment: None,
                        visibility: Visibility::Pub,
                        items: vec![],
                        submodules: vec![],
                        use_statements: vec![],
                        is_inline: false,
                    },
                ],
                use_statements: vec![],
                is_inline: false,
            },
        }
    }

    #[test]
    fn test_generate_overview() {
        let crates = vec![sample_crate()];
        let annotations = AnnotationStore::default();
        let output = generate_overview(&crates, &annotations);

        assert!(output.contains("# Crate: test_crate (lib)"));
        assert!(output.contains("Edition: 2021"));
        assert!(output.contains("serde, tokio"));
        assert!(output.contains("- crate — Main library crate"));
        assert!(output.contains("  - config — Configuration module"));
    }
}
