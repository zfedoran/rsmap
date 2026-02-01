use crate::annotations::AnnotationStore;
use crate::model::{CrateInfo, Item, ItemKind, Module};

/// Generate Layer 1: API Surface (api-surface.md)
///
/// All items (pub AND private), grouped by module, signatures only.
pub fn generate_api_surface(crates: &[CrateInfo], annotations: &AnnotationStore) -> String {
    let mut out = String::new();

    for crate_info in crates {
        out.push_str(&format!(
            "# Crate: {} ({})\n\n",
            crate_info.name, crate_info.kind
        ));
        write_module_surface(&mut out, &crate_info.root_module, annotations);
    }

    out
}

fn write_module_surface(out: &mut String, module: &Module, annotations: &AnnotationStore) {
    // Module header
    out.push_str(&format!("# {}\n", module.path));
    out.push_str(&format!(
        "<!-- file: {} -->\n\n",
        module.file_path.display(),
    ));

    // Group items by kind
    let types: Vec<&Item> = module
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Struct | ItemKind::Enum | ItemKind::TypeAlias))
        .collect();

    let traits: Vec<&Item> = module
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Trait))
        .collect();

    let functions: Vec<&Item> = module
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Function))
        .collect();

    let impls: Vec<&Item> = module
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Impl { .. }))
        .collect();

    let consts: Vec<&Item> = module
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Const | ItemKind::Static))
        .collect();

    let macros: Vec<&Item> = module
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Macro))
        .collect();

    let uses: Vec<&Item> = module
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Use))
        .collect();

    if !types.is_empty() {
        out.push_str("## Types\n\n");
        for item in &types {
            write_item(out, item, annotations, &module.path);
        }
        out.push('\n');
    }

    if !traits.is_empty() {
        out.push_str("## Traits\n\n");
        for item in &traits {
            write_item(out, item, annotations, &module.path);
        }
        out.push('\n');
    }

    if !functions.is_empty() {
        out.push_str("## Functions\n\n");
        for item in &functions {
            write_item(out, item, annotations, &module.path);
        }
        out.push('\n');
    }

    if !impls.is_empty() {
        for item in &impls {
            // Use the impl block's name as section header
            out.push_str(&format!("## {}\n\n", format_impl_header(&item.kind)));
            // The signature contains the full impl with methods
            write_item(out, item, annotations, &module.path);
            out.push('\n');
        }
    }

    if !consts.is_empty() {
        out.push_str("## Constants\n\n");
        for item in &consts {
            write_item(out, item, annotations, &module.path);
        }
        out.push('\n');
    }

    if !macros.is_empty() {
        out.push_str("## Macros\n\n");
        for item in &macros {
            write_item(out, item, annotations, &module.path);
        }
        out.push('\n');
    }

    if !uses.is_empty() {
        out.push_str("## Re-exports\n\n");
        for item in &uses {
            write_item(out, item, annotations, &module.path);
        }
        out.push('\n');
    }

    out.push_str("---\n\n");

    // Recurse into submodules
    for sub in &module.submodules {
        write_module_surface(out, sub, annotations);
    }
}

fn write_item(out: &mut String, item: &Item, annotations: &AnnotationStore, module_path: &str) {
    // Add doc comment if present
    if let Some(ref doc) = item.doc_comment {
        for line in doc.lines() {
            out.push_str(&format!("/// {}\n", line));
        }
    }

    // Add annotation if present
    let item_path = format!("{}::{}", module_path, item.name);
    if let Some(entry) = annotations.items.get(&item_path) {
        if !entry.note.is_empty() {
            out.push_str(&format!("// NOTE: {}\n", entry.note));
        }
    }

    out.push_str(&item.signature);
    out.push_str("\n\n");
}

fn format_impl_header(kind: &ItemKind) -> String {
    match kind {
        ItemKind::Impl {
            self_ty,
            trait_name: Some(trait_name),
        } => format!("Impl {} for {}", trait_name, self_ty),
        ItemKind::Impl {
            self_ty,
            trait_name: None,
        } => format!("Impl {}", self_ty),
        _ => "Impl".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::path::PathBuf;

    #[test]
    fn test_generate_api_surface() {
        let crates = vec![CrateInfo {
            name: "test".to_string(),
            kind: CrateKind::Lib,
            edition: "2021".to_string(),
            version: "0.1.0".to_string(),
            external_deps: vec![],
            root_module: Module {
                path: "crate".to_string(),
                file_path: PathBuf::from("src/lib.rs"),
                file_hash: "abc12345".to_string(),
                doc_comment: None,
                visibility: Visibility::Pub,
                items: vec![
                    Item {
                        name: "Config".to_string(),
                        kind: ItemKind::Struct,
                        visibility: Visibility::Pub,
                        signature: "pub struct Config {\n    pub name: String,\n}".to_string(),
                        doc_comment: Some("Configuration struct".to_string()),
                        file_path: PathBuf::from("src/lib.rs"),
                        line_start: 1,
                        line_end: 3,
                        content_hash: "hash1".to_string(),
                    },
                    Item {
                        name: "init".to_string(),
                        kind: ItemKind::Function,
                        visibility: Visibility::Pub,
                        signature: "pub fn init() -> Config;".to_string(),
                        doc_comment: None,
                        file_path: PathBuf::from("src/lib.rs"),
                        line_start: 5,
                        line_end: 10,
                        content_hash: "hash2".to_string(),
                    },
                ],
                submodules: vec![],
                use_statements: vec![],
                is_inline: false,
            },
        }];

        let annotations = AnnotationStore::default();
        let output = generate_api_surface(&crates, &annotations);

        assert!(output.contains("## Types"));
        assert!(output.contains("pub struct Config"));
        assert!(output.contains("## Functions"));
        assert!(output.contains("pub fn init() -> Config;"));
        assert!(output.contains("/// Configuration struct"));
    }
}
